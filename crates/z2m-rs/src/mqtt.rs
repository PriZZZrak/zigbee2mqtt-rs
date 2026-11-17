// Original: .upstream/zigbee2mqtt/lib/mqtt.ts
//
// Thin async wrapper around `rumqttc` (pure Rust, no default features) that
// keeps the upstream publish/retain/subscribe contract used by the bridge.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{json, Value};

use crate::event_bus::EventBus;
use zigbee_apiarist::utils::logger::logger;

/// MQTT remaining-length protocol maximum (256 MiB - 1). Used for the outgoing
/// direction because upstream has no client-side outgoing limit.
const MAX_OUTGOING_PACKET_SIZE: usize = 268_435_455;

/// Upstream `MqttPublishOptions`.
#[derive(Debug, Clone)]
pub struct MqttPublishOptions {
    pub retain: bool,
    pub qos: u8,
    pub skip_log: bool,
    pub skip_receive: bool,
    pub is_entity_state: bool,
    /// Upstream `properties.messageExpiryInterval` (retention). rumqttc 0.25
    /// is built without MQTT v5 in this workspace, so it is tracked but not
    /// sent on the wire yet.
    pub message_expiry_interval: Option<u32>,
    /// Upstream `baseTopic` override (HA discovery uses its own topic root).
    pub base_topic: Option<String>,
}

impl Default for MqttPublishOptions {
    fn default() -> Self {
        Self {
            retain: false,
            qos: 0,
            skip_log: false,
            // Upstream default: skip messages we published ourselves.
            skip_receive: true,
            is_entity_state: false,
            message_expiry_interval: None,
            base_topic: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetainedMessage {
    pub topic: String,
    pub payload: String,
    pub options: MqttPublishOptions,
}

pub struct Mqtt {
    pub event_bus: Arc<Mutex<EventBus>>,
    pub base_topic: String,
    pub server: String,
    pub connected: Arc<AtomicBool>,
    pub published_topics: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Publishes currently enqueued in `rumqttc` (upstream `client.queue.length`).
    queued: Arc<AtomicUsize>,
    /// QoS1/2 publishes awaiting broker acknowledgement.
    unacked: Arc<Mutex<std::collections::HashSet<u16>>>,
    /// Upstream `retainedMessages`; shared with the event loop so the
    /// CONNACK `bridge/state` publish is recorded like any other retained one.
    pub retained_messages: Arc<Mutex<BTreeMap<String, RetainedMessage>>>,
    /// Upstream `mqtt.force_disable_retain`: retain off on the wire, no tracking.
    pub force_disable_retain: bool,
    /// Upstream tests spy on `mqtt.publish`; this keeps the same observable
    /// call log when no broker is connected.
    pub attempted: Vec<(String, String, MqttPublishOptions)>,
    client: Option<AsyncClient>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Mqtt {
    pub fn new(event_bus: Arc<Mutex<EventBus>>) -> Self {
        Self {
            event_bus,
            base_topic: "zigbee2mqtt".to_string(),
            server: "mqtt://localhost:1883".to_string(),
            connected: Arc::new(AtomicBool::new(false)),
            published_topics: Arc::new(Mutex::new(std::collections::HashSet::new())),
            queued: Arc::new(AtomicUsize::new(0)),
            unacked: Arc::new(Mutex::new(std::collections::HashSet::new())),
            retained_messages: Arc::new(Mutex::new(BTreeMap::new())),
            force_disable_retain: false,
            attempted: Vec::new(),
            client: None,
            task: None,
        }
    }

    pub fn with_settings(event_bus: Arc<Mutex<EventBus>>, settings: &Value) -> Self {
        let mut mqtt = Self::new(event_bus);
        mqtt.base_topic = settings["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        mqtt.server = settings["mqtt"]["server"]
            .as_str()
            .unwrap_or("mqtt://localhost:1883")
            .to_string();
        mqtt
    }

    /// Upstream `Mqtt.connect()`.
    pub async fn connect(&mut self, settings: &Value) -> Result<(), String> {
        let mqtt_settings = &settings["mqtt"];
        let server = mqtt_settings["server"]
            .as_str()
            .unwrap_or("mqtt://localhost:1883");
        let (host, port, use_tls) = parse_server(server)?;
        self.base_topic = mqtt_settings["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        self.server = server.to_string();

        let client_id = mqtt_settings["client_id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("zigbee2mqtt-{}", std::process::id()));
        let mut options = MqttOptions::new(client_id, (host.as_str(), port));
        if use_tls {
            let config = build_tls_config(mqtt_settings)?;
            options.set_transport(rumqttc::Transport::Tls(
                rumqttc::TlsConfiguration::Rustls(config),
            ));
        }
        if let Some(keepalive) = mqtt_settings["keepalive"].as_u64() {
            options.set_keep_alive(keepalive.min(u16::MAX as u64) as u16);
        }
        // Upstream (mqtt.js) does not limit outgoing packets; the broker does.
        // `mqtt.maximum_packet_size` describes what the server may send to us
        // (MQTT 5 CONNECT property), so keep the outgoing direction at the
        // protocol maximum instead of rumqttc's 10 KiB default.
        let maximum_packet_size = mqtt_settings["maximum_packet_size"]
            .as_u64()
            .unwrap_or(1_048_576)
            .clamp(20, MAX_OUTGOING_PACKET_SIZE as u64) as usize;
        options.set_max_packet_size(maximum_packet_size, MAX_OUTGOING_PACKET_SIZE);
        if let (Some(user), Some(password)) = (
            mqtt_settings["user"].as_str(),
            mqtt_settings["password"].as_str(),
        ) {
            options.set_credentials(user, password.to_string());
        } else if let Some(user) = mqtt_settings["user"].as_str() {
            options.set_credentials(user, String::new());
        }
        if let Some(id) = mqtt_settings["client_id"].as_str() {
            options.set_clean_session(false);
            let _ = id;
        }

        self.force_disable_retain = mqtt_settings["force_disable_retain"]
            .as_bool()
            .unwrap_or(false);
        let (client, mut eventloop) = AsyncClient::builder(options).capacity(64).build();
        let base_topic = self.base_topic.clone();
        let event_bus = self.event_bus.clone();
        let published_topics = self.published_topics.clone();
        let retained_messages = self.retained_messages.clone();
        let unacked = self.unacked.clone();
        let connected = self.connected.clone();
        let force_disable_retain = mqtt_settings["force_disable_retain"]
            .as_bool()
            .unwrap_or(false);
        let republish: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
        let republish_in_loop = republish.clone();
        let connect_client = client.clone();
        let task = tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let topic = String::from_utf8_lossy(&publish.topic).to_string();
                        // Retained `bridge/info` means the broker kept our messages:
                        // cancel the pending republish (mqtt.ts:201-206).
                        if topic == format!("{base_topic}/bridge/info") {
                            if let Some(timer) = republish_in_loop.lock().unwrap().take() {
                                timer.abort();
                            }
                        }
                        if published_topics.lock().unwrap().remove(&topic) {
                            continue;
                        }
                        // Upstream uses `payload.toString()`; lossy UTF-8 keeps
                        // the same contract instead of dropping the message.
                        let payload = String::from_utf8_lossy(&publish.payload).to_string();
                        event_bus.lock().unwrap().emit_mqtt_message(json!({
                            "topic": topic,
                            "message": payload,
                        }));
                    }
                    Ok(Event::Outgoing(rumqttc::Outgoing::Publish(pkid))) => {
                        unacked.lock().unwrap().insert(pkid);
                    }
                    Ok(Event::Incoming(Packet::PubAck(ack))) => {
                        unacked.lock().unwrap().remove(&ack.pkid);
                    }
                    Ok(Event::Incoming(Packet::PubComp(ack))) => {
                        unacked.lock().unwrap().remove(&ack.pkid);
                    }
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        // Upstream publishes the state through its own
                        // `publish()`, so the retained map and the publish event
                        // see it too (znp state is replayed by the frontend).
                        let state_topic = format!("{base_topic}/bridge/state");
                        let state_payload =
                            serde_json::to_vec(&json!({"state": "online"})).unwrap_or_default();
                        let _ = connect_client
                            .publish(
                                state_topic.clone(),
                                state_payload.clone(),
                                rumqttc::PublishOptions::new(QoS::AtLeastOnce).retain(true),
                            )
                            .await;
                        let text = String::from_utf8_lossy(&state_payload).to_string();
                        retained_messages.lock().unwrap().insert(
                            state_topic.clone(),
                            RetainedMessage {
                                topic: state_topic.clone(),
                                payload: text.clone(),
                                options: MqttPublishOptions {
                                    retain: true,
                                    qos: 1,
                                    ..Default::default()
                                },
                            },
                        );
                        published_topics.lock().unwrap().insert(state_topic.clone());
                        event_bus.lock().unwrap().emit_mqtt_message_published(json!({
                            "topic": state_topic,
                            "payload": text,
                            "options": {"meta": {"isEntityState": false}},
                        }));
                        let _ = connect_client
                            .subscribe(format!("{base_topic}/#"), QoS::AtLeastOnce)
                            .await;

                        // Upstream republishes tracked retained messages 2 s after
                        // connect unless `bridge/info` arrives first (mqtt.ts:177-189).
                        if let Some(previous) = republish_in_loop.lock().unwrap().take() {
                            previous.abort();
                        }
                        let client = connect_client.clone();
                        let retained = retained_messages.clone();
                        let timer = tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            let messages: Vec<RetainedMessage> =
                                retained.lock().unwrap().values().cloned().collect();
                            for message in messages {
                                let options = rumqttc::PublishOptions::new(QoS::AtLeastOnce)
                                    .retain(message.options.retain && !force_disable_retain);
                                let _ = client
                                    .publish(
                                        message.topic.clone(),
                                        message.payload.clone().into_bytes(),
                                        options,
                                    )
                                    .await;
                            }
                        });
                        *republish_in_loop.lock().unwrap() = Some(timer);
                        connected.store(true, Ordering::SeqCst);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        connected.store(false, Ordering::SeqCst);
                        logger().error(&format!("MQTT error: {error}"), "z2m:mqtt");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
                let _ = &base_topic;
            }
        });

        self.client = Some(client);
        self.task = Some(task);
        Ok(())
    }

    /// Upstream `Mqtt.disconnect()`.
    pub async fn disconnect(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let offline_topic = format!("{}/bridge/state", self.base_topic);
            let offline_payload =
                serde_json::to_vec(&json!({"state": "offline"})).unwrap_or_default();
            // Upstream publishes the offline state with qos 0 (mqtt.ts stop()).
            let _ = client
                .publish(
                    offline_topic.clone(),
                    offline_payload.clone(),
                    rumqttc::PublishOptions::new(QoS::AtMostOnce).retain(true),
                )
                .await;
            self.retained_messages.lock().unwrap().insert(
                offline_topic.clone(),
                RetainedMessage {
                    topic: offline_topic,
                    payload: String::from_utf8_lossy(&offline_payload).to_string(),
                    options: MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        ..Default::default()
                    },
                },
            );
            let _ = client.disconnect().await;
        }
        self.connected.store(false, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    /// Upstream `Mqtt.publish(topic, payload, options)`.
    pub async fn publish(
        &mut self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        options: MqttPublishOptions,
    ) -> bool {
        let payload = payload.into();
        let qos = match options.qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        };
        if topic.contains('+') || topic.contains('#') {
            logger().error(
                &format!("Topic '{topic}' includes wildcard characters, skipping publish."),
                "z2m:mqtt",
            );
            return false;
        }
        let base_topic = options
            .base_topic
            .clone()
            .unwrap_or_else(|| self.base_topic.clone());
        let topic = format!("{}/{}", base_topic, topic);
        self.queued.fetch_add(1, Ordering::SeqCst);
        let _pending = PendingPublish(self.queued.clone());
        self.attempted.push((
            topic.clone(),
            String::from_utf8_lossy(&payload).to_string(),
            options.clone(),
        ));
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        // Upstream forces retain off on the wire when `force_disable_retain`
        // is set and then does not track the message as retained (mqtt.ts:246, :225).
        let retain = options.retain && !self.force_disable_retain;
        if client
            .publish(&topic, payload.clone(), rumqttc::PublishOptions::new(qos).retain(retain))
            .await
            .is_err()
        {
            return false;
        }
        if options.skip_receive {
            self.published_topics.lock().unwrap().insert(topic.clone());
        }
        if retain {
            self.retained_messages.lock().unwrap().insert(
                topic.clone(),
                RetainedMessage {
                    topic: topic.clone(),
                    payload: String::from_utf8_lossy(&payload).to_string(),
                    options: options.clone(),
                },
            );
        } else {
            self.retained_messages.lock().unwrap().remove(&topic);
        }
        self.event_bus
            .lock()
            .unwrap()
            .emit_mqtt_message_published(json!({
                "topic": topic,
                "payload": String::from_utf8_lossy(&payload).to_string(),
                "options": {"meta": {"isEntityState": options.is_entity_state}},
            }));
        true
    }

    /// Upstream `Mqtt.subscribe(topic, callback)`.
    pub async fn subscribe(&mut self, topic: &str, qos: u8) -> bool {
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        let qos = match qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        };
        client.subscribe(topic, qos).await.is_ok()
    }

    /// Upstream `Mqtt.unsubscribe(topic)`.
    pub async fn unsubscribe(&mut self, topic: &str) -> bool {
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        client.unsubscribe(topic).await.is_ok()
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn info(&self) -> Value {
        json!({
            "version": 4,
            "server": self.server,
        })
    }

    /// Upstream `Mqtt.stats` (`mqtt.ts:37-40`).
    pub fn stats(&self) -> Value {
        json!({
            "connected": self.is_connected(),
            "queued": self.queued.load(Ordering::SeqCst)
                + self.unacked.lock().unwrap().len(),
        })
    }

    /// Upstream `Mqtt.onMessage`.
    pub async fn on_message(&mut self, topic: &str, payload: Value) {
        self.event_bus.lock().unwrap().emit_mqtt_message(json!({
            "topic": topic,
            "message": payload,
        }));
    }
}

fn parse_server(server: &str) -> Result<(String, u16, bool), String> {
    // Upstream (mqtt.js) accepts a server without an explicit port and falls
    // back to the scheme default; zigbee2mqtt's own default config uses
    // `mqtt://localhost`.
    let (rest, default_port, tls) = if let Some(rest) = server
        .strip_prefix("mqtts://")
        .or_else(|| server.strip_prefix("ssl://"))
    {
        (rest, 8883, true)
    } else if let Some(rest) = server
        .strip_prefix("mqtt://")
        .or_else(|| server.strip_prefix("tcp://"))
    {
        (rest, 1883, false)
    } else {
        return Err(format!("Unsupported MQTT server '{server}'"));
    };

    // Bracketed IPv6 literal, optionally followed by `:port`.
    if let Some(tail) = rest.strip_prefix('[') {
        let (host, rest) = tail
            .split_once(']')
            .ok_or_else(|| format!("Invalid MQTT server '{server}'"))?;
        let port = match rest.strip_prefix(':') {
            Some(port) => port
                .parse::<u16>()
                .map_err(|_| format!("Invalid MQTT port in '{server}'"))?,
            None if rest.is_empty() => default_port,
            None => return Err(format!("Invalid MQTT server '{server}'")),
        };
        if host.is_empty() {
            return Err(format!("Invalid MQTT server '{server}'"));
        }
        return Ok((host.to_string(), port, tls));
    }

    // A bare IPv6 address (multiple colons, no brackets) has no port.
    let (host, port) = if rest.matches(':').count() > 1 {
        (rest, default_port)
    } else {
        match rest.rsplit_once(':') {
            Some((host, port)) => (
                host,
                port.parse::<u16>()
                    .map_err(|_| format!("Invalid MQTT port in '{server}'"))?,
            ),
            None => (rest, default_port),
        }
    };
    if host.is_empty() {
        return Err(format!("Invalid MQTT server '{server}'"));
    }
    Ok((host.to_string(), port, tls))
}

/// Upstream mqtt.js TLS options: `reject_unauthorized`, `ca`, `cert`, `key`.
fn build_tls_config(mqtt_settings: &Value) -> Result<Arc<tokio_rustls::rustls::ClientConfig>, String> {
    use std::sync::Arc as StdArc;
    use tokio_rustls::rustls;

    let _ = rustls::crypto::CryptoProvider::install_default(rustls_rustcrypto::provider());
    let provider = StdArc::new(rustls_rustcrypto::provider());
    let reject_unauthorized = mqtt_settings["reject_unauthorized"]
        .as_bool()
        .unwrap_or(true);
    let client_auth = match (
        mqtt_settings["cert"].as_str(),
        mqtt_settings["key"].as_str(),
    ) {
        (Some(cert), Some(key)) => {
            let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(
                std::fs::File::open(cert).map_err(|error| error.to_string())?,
            ))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
            let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(
                std::fs::File::open(key).map_err(|error| error.to_string())?,
            ))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "No private key found in mqtt.key".to_string())?;
            Some((certs, key))
        }
        _ => None,
    };

    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?;
    let config = if reject_unauthorized {
        let roots = root_store(mqtt_settings)?;
        let builder = builder.with_root_certificates(roots);
        match client_auth {
            Some((certs, key)) => builder
                .with_client_auth_cert(certs, key)
                .map_err(|error| error.to_string())?,
            None => builder.with_no_client_auth(),
        }
    } else {
        let builder = builder
            .dangerous()
            .with_custom_certificate_verifier(StdArc::new(NoVerification::new()));
        match client_auth {
            Some((certs, key)) => builder
                .with_client_auth_cert(certs, key)
                .map_err(|error| error.to_string())?,
            None => builder.with_no_client_auth(),
        }
    };
    Ok(Arc::new(config))
}

fn root_store(mqtt_settings: &Value) -> Result<tokio_rustls::rustls::RootCertStore, String> {
    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        let _ = roots.add(cert);
    }
    if let Some(path) = mqtt_settings["ca"].as_str() {
        let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(
            std::fs::File::open(path).map_err(|error| error.to_string())?,
        ))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
        for cert in certs {
            roots.add(cert).map_err(|error| error.to_string())?;
        }
    }
    Ok(roots)
}

/// Upstream mqtt.js `rejectUnauthorized: false`.
#[derive(Debug)]
struct NoVerification {
    provider: std::sync::Arc<tokio_rustls::rustls::crypto::CryptoProvider>,
}

impl NoVerification {
    fn new() -> Self {
        Self {
            provider: std::sync::Arc::new(rustls_rustcrypto::provider()),
        }
    }
}

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &tokio_rustls::rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

struct PendingPublish(Arc<AtomicUsize>);

impl Drop for PendingPublish {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::{build_tls_config, parse_server};
    use serde_json::json;

    #[test]
    fn parses_plain_and_tls_servers() {
        assert_eq!(
            parse_server("mqtt://localhost:1883").unwrap(),
            ("localhost".to_string(), 1883, false)
        );
        // mqtt.js defaults the port from the scheme (z2m's default config is
        // `server: mqtt://localhost`).
        assert_eq!(
            parse_server("mqtt://localhost").unwrap(),
            ("localhost".to_string(), 1883, false)
        );
        assert_eq!(
            parse_server("mqtts://broker.example").unwrap(),
            ("broker.example".to_string(), 8883, true)
        );
        assert_eq!(
            parse_server("mqtt://[::1]").unwrap(),
            ("::1".to_string(), 1883, false)
        );
        assert_eq!(
            parse_server("mqtt://[::1]:1885").unwrap(),
            ("::1".to_string(), 1885, false)
        );
        assert_eq!(
            parse_server("tcp://broker:1884").unwrap(),
            ("broker".to_string(), 1884, false)
        );
        assert_eq!(
            parse_server("mqtts://secure:8883").unwrap(),
            ("secure".to_string(), 8883, true)
        );
        assert_eq!(
            parse_server("ssl://legacy:8883").unwrap(),
            ("legacy".to_string(), 8883, true)
        );
        assert!(parse_server("ws://localhost:8080").is_err());
        assert!(parse_server("mqtt://").is_err());
    }

    #[test]
    fn builds_a_pure_rust_tls_config() {
        let config = build_tls_config(&json!({"reject_unauthorized": false}))
            .expect("TLS config builds with the rustcrypto provider");
        assert!(config.alpn_protocols.is_empty());
    }
}
