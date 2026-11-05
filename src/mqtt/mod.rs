/// MQTT bridge – publishes device state and subscribes to set/get commands.
/// Message formats are compatible with zigbee2mqtt for Home Assistant integration.
pub mod attribute_output;

use std::time::Duration;

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

use crate::config::MqttConfig;
use crate::error::{Error, Result};

const MQTT_RECONNECT_DELAY_MIN: Duration = Duration::from_secs(1);
const MQTT_RECONNECT_DELAY_MAX: Duration = Duration::from_secs(30);

// ── Inbound MQTT commands ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum MqttCommand {
    Connected,
    PermitJoin {
        duration: u8,
    },
    SetDevice {
        friendly_name: String,
        payload: serde_json::Value,
        /// Endpoint selected by a multi-endpoint command topic
        /// (`<base>/<name>/l1/set`), or `None` for the whole device.
        endpoint: Option<u8>,
        /// Attribute from `<base>/<name>/set/<attribute>`.
        attribute: Option<String>,
    },
    GetDevice {
        friendly_name: String,
        /// Requested keys (JSON object) or null/empty for all readable keys.
        payload: serde_json::Value,
        /// Endpoint selected by `<base>/<name>/<endpoint>/get`.
        endpoint: Option<u8>,
    },
    BridgeRequest {
        request: String,
        payload: serde_json::Value,
    },
}

/// Build a device command from an MQTT topic/payload pair.
///
/// Kept separate from the event loop so the MQTT command contract can be tested
/// without a broker.
pub(crate) fn parse_device_command(
    base_topic: &str,
    topic: &str,
    payload: &[u8],
) -> Option<MqttCommand> {
    // Mirrors z2m's `^<base>/(?!bridge)(.+?)/(get|set)(?:/(.+))?$`:
    // the first `get`/`set` segment splits the entity path from the optional
    // attribute. Entity paths may themselves contain '/'.
    let rest = topic.strip_prefix(base_topic)?.strip_prefix('/')?;
    let mut segments: Vec<&str> = rest.split('/').collect();
    let position = segments
        .iter()
        .position(|segment| *segment == "set" || *segment == "get")?;
    let is_set = segments[position] == "set";
    let attribute = if segments.len() > position + 1 {
        let joined = segments[position + 1..].join("/");
        (!joined.is_empty()).then_some(joined)
    } else {
        None
    };
    segments.truncate(position);
    if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    // A trailing endpoint selector is the last entity-path segment.
    let endpoint = segments.last().and_then(|last| endpoint_index(last));
    if endpoint.is_some() {
        segments.pop();
    }
    if segments.is_empty() {
        return None;
    }
    let friendly_name = segments.join("/");

    let json_value = match &attribute {
        // `<...>/set/<attribute>`: the payload is the attribute value.
        Some(attribute) => {
            let value = serde_json::from_slice::<serde_json::Value>(payload).unwrap_or_else(|_| {
                match std::str::from_utf8(payload) {
                    Ok(text) => json!(text.trim()),
                    Err(_) => serde_json::Value::Null,
                }
            });
            json!({ attribute.as_str(): value })
        }
        None if !is_set && payload.is_empty() => serde_json::Value::Null,
        None => serde_json::from_slice::<serde_json::Value>(payload).unwrap_or_else(|_| {
            if let Ok(text) = std::str::from_utf8(payload) {
                json!({ "state": text.trim() })
            } else {
                serde_json::Value::Null
            }
        }),
    };

    Some(if is_set {
        MqttCommand::SetDevice {
            friendly_name,
            payload: json_value,
            endpoint,
            attribute,
        }
    } else {
        MqttCommand::GetDevice {
            friendly_name,
            payload: json_value,
            endpoint,
        }
    })
}

pub(crate) type MqttFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>>;

/// Publishing seam used by bridge request handlers, allowing a recording fake in
/// broker-free tests.
pub(crate) trait MqttPublishIo {
    fn publish<'a>(
        &'a self,
        topic: String,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
    ) -> MqttFuture<'a, ()>;
    fn publish_response<'a>(
        &'a self,
        request: String,
        payload: serde_json::Value,
    ) -> MqttFuture<'a, ()>;
    fn publish_devices<'a>(&'a self, devices: serde_json::Value) -> MqttFuture<'a, ()>;
}

impl MqttPublishIo for MqttBridge {
    fn publish<'a>(
        &'a self,
        topic: String,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
    ) -> MqttFuture<'a, ()> {
        Box::pin(async move {
            let qos = match qos {
                0 => QoS::AtMostOnce,
                2 => QoS::ExactlyOnce,
                _ => QoS::AtLeastOnce,
            };
            self.client
                .publish(&topic, qos, retain, payload)
                .await
                .map_err(Error::Mqtt)
        })
    }

    fn publish_response<'a>(
        &'a self,
        request: String,
        payload: serde_json::Value,
    ) -> MqttFuture<'a, ()> {
        Box::pin(async move { self.publish_bridge_response(&request, &payload).await })
    }

    fn publish_devices<'a>(&'a self, devices: serde_json::Value) -> MqttFuture<'a, ()> {
        Box::pin(async move { self.publish_bridge_devices(&devices).await })
    }
}

/// Topic for a Home Assistant discovery config.
pub(crate) fn ha_discovery_topic(component: &str, node_id: &str, object_id: &str) -> String {
    format!("homeassistant/{component}/{node_id}/{object_id}/config")
}

/// Empty payload clears a retained discovery topic.
#[must_use]
pub(crate) fn ha_clear_payload() -> Vec<u8> {
    Vec::new()
}

/// Topic for a bridge request response.
pub(crate) fn bridge_response_topic(base_topic: &str, request: &str) -> String {
    format!("{base_topic}/bridge/response/{request}")
}

/// Parse a `<base>/bridge/request/<request>` publish.
pub(crate) fn parse_bridge_request(
    base_topic: &str,
    topic: &str,
    payload: &[u8],
) -> Option<(String, serde_json::Value)> {
    let marker = "bridge/request";
    if !topic.contains(marker) {
        return None;
    }
    let prefix = format!("{base_topic}/bridge/request/");
    let request = topic.strip_prefix(&prefix).unwrap_or(topic).to_string();
    let payload =
        serde_json::from_slice::<serde_json::Value>(payload).unwrap_or(serde_json::Value::Null);
    Some((request, payload))
}

/// Per-endpoint state topic used for stripped multi-gang snapshots.
pub(crate) fn endpoint_state_topic(
    base_topic: &str,
    friendly_name: &str,
    endpoint: &str,
) -> String {
    format!("{base_topic}/{friendly_name}/{endpoint}")
}

/// Map a multi-endpoint selector (`l1`, `l2`, ... or `button`) to an endpoint.
///
/// This is the cluster-fallback naming used by the bridge; definitions that
/// provide explicit endpoint names are handled by the exposes path.
pub(crate) fn endpoint_index(selector: &str) -> Option<u8> {
    if selector == "button" {
        return Some(3);
    }
    selector
        .strip_prefix('l')
        .and_then(|digits| digits.parse::<u8>().ok())
        .filter(|endpoint| *endpoint > 0)
}

// ── MqttBridge ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MqttBridge {
    client: AsyncClient,
    base_topic: String,
    /// Frontend-facing copy of every publish (`lib/extension/frontend.ts`).
    events: Option<tokio::sync::broadcast::Sender<PublishedEvent>>,
}

/// One message published by the bridge, forwarded to the frontend.
#[derive(Debug, Clone)]
pub struct PublishedEvent {
    pub topic: String,
    pub payload: Vec<u8>,
    pub retain: bool,
}

impl MqttBridge {
    /// Connect to the broker, subscribe to command topics, and spawn the event loop.
    pub async fn connect(cfg: &MqttConfig) -> Result<(Self, mpsc::Receiver<MqttCommand>)> {
        Self::connect_with_events(cfg, None).await
    }

    /// Connect and forward every publish to the frontend event bus.
    pub async fn connect_with_events(
        cfg: &MqttConfig,
        events: Option<tokio::sync::broadcast::Sender<PublishedEvent>>,
    ) -> Result<(Self, mpsc::Receiver<MqttCommand>)> {
        if cfg.tls_required {
            return Err(Error::Config(
                "mqtts:// requires TLS, which this build does not support; refusing to fall back to plain TCP"
                    .to_string(),
            ));
        }
        let mut opts = MqttOptions::new(&cfg.client_id, &cfg.server, cfg.port);
        opts.set_keep_alive(Duration::from_secs(cfg.keepalive as u64));
        opts.set_clean_session(true);
        // rumqttc defaults to a 10 KiB outgoing packet limit, which silently
        // drops `bridge/devices` (and large state payloads) for real networks.
        // The MQTT remaining-length field allows up to 256 MiB.
        opts.set_max_packet_size(256 * 1024 * 1024, 256 * 1024 * 1024);

        if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
            opts.set_credentials(user, pass);
        }

        // Last-will: bridge goes offline (z2m JSON format)
        let will_topic = format!("{}/bridge/state", cfg.base_topic);
        let will_payload = serde_json::to_vec(&json!({"state": "offline"})).unwrap();
        opts.set_last_will(rumqttc::LastWill::new(
            &will_topic,
            will_payload,
            QoS::AtLeastOnce,
            true,
        ));

        let (client, event_loop) = AsyncClient::new(opts, 64);
        let (cmd_tx, cmd_rx) = mpsc::channel::<MqttCommand>(64);
        let (connected_tx, connected_rx) = oneshot::channel();

        let base_topic = cfg.base_topic.clone();
        let client_clone = client.clone();

        tokio::spawn(async move {
            run_event_loop(event_loop, client_clone, &base_topic, cmd_tx, connected_tx).await;
        });

        connected_rx.await.map_err(|_| {
            Error::Config("MQTT event loop exited before initial connection".to_string())
        })?;

        Ok((
            Self {
                client,
                base_topic: cfg.base_topic.clone(),
                events,
            },
            cmd_rx,
        ))
    }

    // ── Publish helpers ───────────────────────────────────────────────────────

    /// Publish bridge/state as JSON (z2m format).
    pub async fn publish_bridge_state(&self, online: bool) -> Result<()> {
        let topic = format!("{}/bridge/state", self.base_topic);
        let state = if online { "online" } else { "offline" };
        let payload = serde_json::to_vec(&json!({"state": state}))?;
        self.publish_retained(&topic, &payload).await
    }

    /// Publish bridge/info (z2m format).
    pub async fn publish_bridge_info(&self, info: &serde_json::Value) -> Result<()> {
        let topic = format!("{}/bridge/info", self.base_topic);
        let payload = serde_json::to_vec(info)?;
        self.publish_retained(&topic, &payload).await
    }

    pub async fn publish_device_state(
        &self,
        friendly_name: &str,
        state: &serde_json::Value,
    ) -> Result<()> {
        let topic = format!("{}/{}", self.base_topic, friendly_name);
        let payload = serde_json::to_vec(state)?;
        self.publish_retained(&topic, &payload).await
    }

    /// Publish one raw attribute value for `advanced.output: attribute`/
    /// `attribute_and_json` mode -- z2m: topic
    /// `<base_topic>/<friendly_name>/<key_path>`, unquoted raw string
    /// payload, retained (same as the merged-JSON publish). See
    /// `attribute_output::flatten_attribute_output` for how `key_path`/
    /// `value` are derived from a device's state.
    pub async fn publish_device_attribute(
        &self,
        friendly_name: &str,
        key_path: &str,
        value: &str,
    ) -> Result<()> {
        let topic = format!("{}/{}/{}", self.base_topic, friendly_name, key_path);
        self.publish_retained(&topic, value.as_bytes()).await
    }

    pub async fn publish_bridge_devices(&self, devices: &serde_json::Value) -> Result<()> {
        let topic = format!("{}/bridge/devices", self.base_topic);
        let payload = serde_json::to_vec(devices)?;
        self.publish_retained(&topic, &payload).await
    }

    /// Publish to bridge/logging (z2m format: {message, level, namespace}).
    pub async fn publish_bridge_log(&self, level: &str, message: &str) -> Result<()> {
        self.publish_bridge_log_with_ns(level, message, "z2m").await
    }

    pub async fn publish_bridge_log_with_ns(
        &self,
        level: &str,
        message: &str,
        namespace: &str,
    ) -> Result<()> {
        let topic = format!("{}/bridge/logging", self.base_topic);
        let payload = json!({
            "level": level,
            "message": message,
            "namespace": namespace,
        });
        let bytes = serde_json::to_vec(&payload)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, bytes.clone())
            .await
            .map_err(Error::Mqtt)?;
        self.emit(&topic, &bytes, false);
        Ok(())
    }

    pub async fn publish_bridge_health(&self, health: &serde_json::Value) -> Result<()> {
        let topic = format!("{}/bridge/health", self.base_topic);
        let bytes = serde_json::to_vec(health)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, bytes)
            .await
            .map_err(Error::Mqtt)
    }

    pub async fn publish_bridge_response(
        &self,
        request: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let topic = bridge_response_topic(&self.base_topic, request);
        let bytes = serde_json::to_vec(payload)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, bytes)
            .await
            .map_err(Error::Mqtt)
    }

    pub async fn publish_bridge_networkmap(
        &self,
        devices: &crate::devices::DeviceRegistry,
    ) -> Result<()> {
        let topic = format!("{}/bridge/networkmap", self.base_topic);
        let nodes: Vec<serde_json::Value> = devices
            .all_devices()
            .into_iter()
            .map(|d| {
                json!({
                    "ieeeAddr": d.ieee_addr.as_hex(),
                    "friendlyName": d.friendly_name,
                    "nwkAddr": d.nwk_addr,
                })
            })
            .collect();
        let payload = json!({ "nodes": nodes, "links": [] });
        let bytes = serde_json::to_vec(&payload)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, bytes)
            .await
            .map_err(Error::Mqtt)
    }

    pub async fn publish_availability(&self, friendly_name: &str, online: bool) -> Result<()> {
        let topic = format!("{}/{}/availability", self.base_topic, friendly_name);
        let payload = if online { "online" } else { "offline" };
        self.publish_retained(&topic, payload.as_bytes()).await
    }

    pub async fn publish_bridge_extensions(&self) -> Result<()> {
        let topic = format!("{}/bridge/extensions", self.base_topic);
        let bytes = serde_json::to_vec(&json!([]))?;
        self.publish_retained(&topic, &bytes).await
    }

    pub async fn publish_bridge_groups(&self, groups: &crate::groups::GroupRegistry) -> Result<()> {
        let topic = format!("{}/bridge/groups", self.base_topic);
        let payload = groups.to_json();
        let bytes = serde_json::to_vec(&payload)?;
        self.publish_retained(&topic, &bytes).await
    }

    /// Publish a per-endpoint state snapshot (`<base>/<friendly>/<endpoint>`).
    ///
    /// z2m republishes stripped endpoint keys here because the HA MQTT JSON
    /// light used for multi-gang devices cannot read suffixed `state_lN` keys
    /// from the main topic.
    pub async fn publish_endpoint_state(
        &self,
        friendly_name: &str,
        endpoint: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let topic = endpoint_state_topic(&self.base_topic, friendly_name, endpoint);
        let bytes = serde_json::to_vec(payload)?;
        self.client
            .publish(&topic, QoS::AtMostOnce, false, bytes.clone())
            .await
            .map_err(Error::Mqtt)?;
        self.emit(&topic, &bytes, false);
        Ok(())
    }

    /// Delete a retained Home Assistant discovery config (empty payload).
    pub async fn clear_ha_discovery(
        &self,
        component: &str,
        node_id: &str,
        object_id: &str,
    ) -> Result<()> {
        let topic = ha_discovery_topic(component, node_id, object_id);
        self.clear_ha_topic(&topic).await
    }

    /// Clear one retained discovery topic by exact topic name.
    pub async fn clear_ha_topic(&self, topic: &str) -> Result<()> {
        let payload = ha_clear_payload();
        self.client
            .publish(topic, QoS::AtLeastOnce, true, payload.clone())
            .await
            .map_err(Error::Mqtt)?;
        self.emit(topic, &payload, true);
        Ok(())
    }

    /// Publish a Home Assistant MQTT discovery config.
    pub async fn publish_ha_discovery(
        &self,
        component: &str,
        node_id: &str,
        object_id: &str,
        config: &serde_json::Value,
    ) -> Result<()> {
        let topic = ha_discovery_topic(component, node_id, object_id);
        tracing::debug!("HA discovery → {topic}");
        let payload = serde_json::to_vec(config)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, payload)
            .await
            .map_err(|e| {
                tracing::error!("Failed to publish HA discovery to {topic}: {e}");
                Error::Mqtt(e)
            })
    }

    /// Publish a raw frontend-originated message (`frontend.ts` calls
    /// `mqtt.onMessage`, which this bridge models as a normal publish).
    pub async fn publish_raw(&self, topic: &str, payload: &[u8]) -> Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload.to_vec())
            .await
            .map_err(Error::Mqtt)?;
        self.emit(topic, payload, false);
        Ok(())
    }

    async fn publish_retained(&self, topic: &str, payload: &[u8]) -> Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, true, payload.to_vec())
            .await
            .map_err(Error::Mqtt)?;
        self.emit(topic, payload, true);
        Ok(())
    }

    /// Forward one successful publish to the frontend event bus.
    fn emit(&self, topic: &str, payload: &[u8], retain: bool) {
        if let Some(events) = &self.events {
            let _ = events.send(PublishedEvent {
                topic: topic.to_string(),
                payload: payload.to_vec(),
                retain,
            });
        }
    }
}

// ── Event loop task ───────────────────────────────────────────────────────────

async fn run_event_loop(
    mut event_loop: EventLoop,
    client: AsyncClient,
    base_topic: &str,
    cmd_tx: mpsc::Sender<MqttCommand>,
    connected_tx: oneshot::Sender<()>,
) {
    let mut connected_once = false;
    let mut connected_tx = Some(connected_tx);
    let mut reconnect_delay = MQTT_RECONNECT_DELAY_MIN;

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                reconnect_delay = MQTT_RECONNECT_DELAY_MIN;
                if subscribe_topics(&client, base_topic).await {
                    if connected_once {
                        let _ = cmd_tx.send(MqttCommand::Connected).await;
                    } else {
                        connected_once = true;
                        if let Some(tx) = connected_tx.take() {
                            let _ = tx.send(());
                        }
                    }
                    info!("MQTT connected and subscribed");
                }
            }
            Ok(Event::Incoming(Packet::Publish(pub_msg))) => {
                let topic = &pub_msg.topic;
                let payload = &pub_msg.payload;

                if let Some(cmd) = parse_device_command(base_topic, topic, payload) {
                    let _ = cmd_tx.send(cmd).await;
                } else if let Some((request, json_value)) =
                    parse_bridge_request(base_topic, topic, payload)
                {
                    // permit_join has legacy dedicated handling but also goes via bridge/request
                    if request == "permit_join" || request.starts_with("permit_join") {
                        let duration = parse_permit_join_payload(payload);
                        let _ = cmd_tx.send(MqttCommand::PermitJoin { duration }).await;
                        // also forward as BridgeRequest for transaction response
                        let _ = cmd_tx
                            .send(MqttCommand::BridgeRequest {
                                request: "permit_join".to_string(),
                                payload: json_value.clone(),
                            })
                            .await;
                    } else {
                        let _ = cmd_tx
                            .send(MqttCommand::BridgeRequest {
                                request,
                                payload: json_value,
                            })
                            .await;
                    }
                }
            }
            Ok(Event::Incoming(Packet::Disconnect)) => {
                warn!(
                    "MQTT disconnected, retrying in {}s",
                    reconnect_delay.as_secs()
                );
                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = next_backoff(reconnect_delay);
            }
            Ok(_) => {}
            Err(e) => {
                error!(
                    "MQTT event loop error: {e}; retrying in {}s",
                    reconnect_delay.as_secs()
                );
                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = next_backoff(reconnect_delay);
            }
        }
    }
}

/// Double the reconnect delay, capped at MQTT_RECONNECT_DELAY_MAX.
fn next_backoff(current: Duration) -> Duration {
    (current * 2).min(MQTT_RECONNECT_DELAY_MAX)
}

/// Extract the friendly name from a "{base_topic}/{friendly_name}/{suffix}" topic.
/// Subscriptions are always "{base_topic}/+/set" or ".../get", so this always
/// matches for topics reaching this function; falls back to the raw topic on
/// any unexpected shape.
#[cfg(test)]
fn friendly_name_from_topic(topic: &str, base_topic: &str, suffix: &str) -> String {
    topic
        .strip_prefix(base_topic)
        .and_then(|s| s.strip_prefix('/'))
        .and_then(|s| s.strip_suffix(suffix))
        .unwrap_or(topic)
        .to_string()
}

async fn subscribe_topics(client: &AsyncClient, base_topic: &str) -> bool {
    // z2m subscribes to `<base>/#` and filters locally; this covers nested
    // friendly names, endpoint selectors and per-attribute set topics.
    let wildcard = format!("{}/#", base_topic);
    let bridge_req_wildcard = format!("{}/bridge/request/#", base_topic);

    for topic in &[&wildcard, &bridge_req_wildcard] {
        if let Err(e) = client.subscribe(*topic, QoS::AtLeastOnce).await {
            error!("MQTT subscribe error for {topic}: {e}");
            return false;
        }
    }

    true
}
/// Parse permit_join payload. Supports z2m JSON format and plain number.
fn parse_permit_join_payload(payload: &[u8]) -> u8 {
    // Try JSON: {"value": true, "time": 254} or {"value": true}
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(payload) {
        if let Some(time) = v.get("time").and_then(|t| t.as_u64()) {
            return time.min(254) as u8;
        }
        if let Some(val) = v.get("value") {
            if val.as_bool() == Some(true) {
                return 254;
            }
            if val.as_bool() == Some(false) {
                return 0;
            }
            if let Some(n) = val.as_u64() {
                return n.min(254) as u8;
            }
        }
    }
    // Plain number. Fail closed (no join) on anything unparseable, rather than
    // enabling pairing for the max duration on a garbage/unexpected payload.
    std::str::from_utf8(payload)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0)
}

/// Minimal in-process MQTT 3.1.1 broker used only by tests.
///
/// Supports CONNECT/CONNACK, SUBSCRIBE/SUBACK, PUBLISH both directions,
/// PUBACK and PINGREQ/PINGRESP — enough to drive the real rumqttc event loop
/// over a TCP socket without an external broker.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod fake_broker {
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Published {
        pub topic: String,
        pub payload: Vec<u8>,
        pub qos: u8,
        pub retain: bool,
    }

    pub struct Outbound {
        pub topic: String,
        pub payload: Vec<u8>,
    }

    pub struct FakeBroker {
        pub port: u16,
        pub publishes: Arc<Mutex<Vec<Published>>>,
        outbound: mpsc::Sender<Outbound>,
    }

    impl FakeBroker {
        /// Start the broker; returns once it has bound its port.
        pub async fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let publishes = Arc::new(Mutex::new(Vec::new()));
            let (outbound, outbound_rx) = mpsc::channel(16);
            let recorded = Arc::clone(&publishes);
            tokio::spawn(async move {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let (mut reader, mut writer) = tokio::io::split(stream);
                let mut outbound_rx = outbound_rx;
                loop {
                    tokio::select! {
                        packet = read_packet(&mut reader) => {
                            let Ok((header, body)) = packet else {
                                break;
                            };
                            let kind = header >> 4;
                            match kind {
                                1 => {
                                    // CONNECT -> CONNACK
                                    let _ = writer.write_all(&[0x20, 0x02, 0x00, 0x00]).await;
                                }
                                8 => {
                                    // SUBSCRIBE -> SUBACK with one QoS1 code per filter
                                    let mut pos = 2;
                                    let mut codes = Vec::new();
                                    while pos + 2 <= body.len() {
                                        let len =
                                            u16::from_be_bytes([body[pos], body[pos + 1]]) as usize;
                                        pos += 2 + len + 1;
                                        codes.push(0x01u8);
                                    }
                                    let mut response = vec![
                                        0x90,
                                        (2 + codes.len()) as u8,
                                        body[0],
                                        body[1],
                                    ];
                                    response.extend_from_slice(&codes);
                                    let _ = writer.write_all(&response).await;
                                }
                                12 => {
                                    // PINGREQ -> PINGRESP
                                    let _ = writer.write_all(&[0xD0, 0x00]).await;
                                }
                                3 => {
                                    // PUBLISH from client
                                    let qos = (header >> 1) & 0x03;
                                    let retain = header & 0x01 != 0;
                                    let topic_len =
                                        u16::from_be_bytes([body[0], body[1]]) as usize;
                                    let topic = String::from_utf8_lossy(
                                        &body[2..2 + topic_len],
                                    )
                                    .to_string();
                                    let mut pos = 2 + topic_len;
                                    let mut packet_id = Vec::new();
                                    if qos > 0 {
                                        packet_id =
                                            body[pos..pos + 2].to_vec();
                                        pos += 2;
                                    }
                                    recorded.lock().unwrap().push(Published {
                                        topic,
                                        payload: body[pos..].to_vec(),
                                        qos,
                                        retain,
                                    });
                                    if qos == 1 {
                                        let _ = writer
                                            .write_all(&[
                                                0x40,
                                                0x02,
                                                packet_id[0],
                                                packet_id[1],
                                            ])
                                            .await;
                                    }
                                }
                                _ => {}
                            }
                        }
                        message = outbound_rx.recv() => {
                            let Some(Outbound { topic, payload }) = message else {
                                break;
                            };
                            let _ = writer.write_all(&encode_publish(&topic, &payload)).await;
                        }
                    }
                }
            });
            Self {
                port,
                publishes,
                outbound,
            }
        }

        /// Send a QoS0 PUBLISH to the connected client.
        pub async fn send(&self, topic: &str, payload: &[u8]) {
            let _ = self
                .outbound
                .send(Outbound {
                    topic: topic.to_string(),
                    payload: payload.to_vec(),
                })
                .await;
        }

        pub fn publishes(&self) -> Vec<Published> {
            self.publishes.lock().unwrap().clone()
        }
    }

    async fn read_packet<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<(u8, Vec<u8>)> {
        let mut first = [0u8; 1];
        reader.read_exact(&mut first).await?;
        let mut multiplier = 1usize;
        let mut length = 0usize;
        loop {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte).await?;
            length += (byte[0] as usize & 0x7F) * multiplier;
            multiplier *= 128;
            if byte[0] & 0x80 == 0 {
                break;
            }
            if multiplier > 128 * 128 * 128 {
                return Err(std::io::Error::other("malformed remaining length"));
            }
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).await?;
        Ok((first[0], body))
    }

    fn remaining_length(mut length: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let mut byte = (length % 128) as u8;
            length /= 128;
            if length > 0 {
                byte |= 0x80;
            }
            bytes.push(byte);
            if length == 0 {
                break;
            }
        }
        bytes
    }

    fn encode_publish(topic: &str, payload: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(topic.len() as u16).to_be_bytes());
        body.extend_from_slice(topic.as_bytes());
        body.extend_from_slice(payload);
        let mut packet = vec![0x30];
        packet.extend_from_slice(&remaining_length(body.len()));
        packet.extend_from_slice(&body);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_permit_join_json_with_time() {
        assert_eq!(
            parse_permit_join_payload(br#"{"value": true, "time": 120}"#),
            120
        );
    }

    #[test]
    fn parse_permit_join_json_true() {
        assert_eq!(parse_permit_join_payload(br#"{"value": true}"#), 254);
    }

    #[test]
    fn parse_permit_join_json_false() {
        assert_eq!(parse_permit_join_payload(br#"{"value": false}"#), 0);
    }

    #[test]
    fn parse_permit_join_plain_number() {
        assert_eq!(parse_permit_join_payload(b"120"), 120);
    }

    #[test]
    fn parse_permit_join_empty_fails_closed() {
        assert_eq!(parse_permit_join_payload(b""), 0);
    }

    #[test]
    fn parse_permit_join_garbage_fails_closed() {
        assert_eq!(parse_permit_join_payload(b"not a number or json"), 0);
    }

    #[test]
    fn backoff_doubles_and_caps() {
        let mut delay = MQTT_RECONNECT_DELAY_MIN;
        for _ in 0..10 {
            delay = next_backoff(delay);
        }
        assert_eq!(delay, MQTT_RECONNECT_DELAY_MAX);
    }

    #[test]
    fn backoff_starts_at_double_the_min() {
        assert_eq!(
            next_backoff(MQTT_RECONNECT_DELAY_MIN),
            MQTT_RECONNECT_DELAY_MIN * 2
        );
    }

    #[test]
    fn friendly_name_from_set_topic() {
        assert_eq!(
            friendly_name_from_topic("zigbee2mqtt/kitchen_light/set", "zigbee2mqtt", "/set"),
            "kitchen_light"
        );
    }

    #[test]
    fn friendly_name_from_get_topic() {
        assert_eq!(
            friendly_name_from_topic("zigbee2mqtt/kitchen_light/get", "zigbee2mqtt", "/get"),
            "kitchen_light"
        );
    }

    #[test]
    fn friendly_name_with_nested_path() {
        // z2m allows friendly names containing '/' (e.g. "room/light")
        assert_eq!(
            friendly_name_from_topic("zigbee2mqtt/room/light/set", "zigbee2mqtt", "/set"),
            "room/light"
        );
    }

    #[tokio::test]
    async fn mqtts_is_rejected_instead_of_downgrading_to_plain_tcp() {
        let cfg = MqttConfig {
            tls_required: true,
            ..MqttConfig::default()
        };
        let error = match MqttBridge::connect(&cfg).await {
            Ok(_) => panic!("mqtts must not fall back to plain TCP"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("TLS"));
        assert!(error.to_string().contains("plain TCP"));
    }

    #[test]
    fn mqtt_get_command_preserves_requested_keys_and_empty_payload() {
        let command = parse_device_command("zigbee2mqtt", "zigbee2mqtt/kitchen/get", b"");
        match command {
            Some(MqttCommand::GetDevice {
                friendly_name,
                payload,
                ..
            }) => {
                assert_eq!(friendly_name, "kitchen");
                assert!(payload.is_null());
            }
            other => panic!("unexpected command: {other:?}"),
        }
        let command = parse_device_command(
            "zigbee2mqtt",
            "zigbee2mqtt/kitchen/get",
            br#"{"temperature":""}"#,
        );
        match command {
            Some(MqttCommand::GetDevice { payload, .. }) => {
                assert!(payload.get("temperature").is_some());
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn mqtt_set_command_keeps_raw_state_fallback() {
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/kitchen/set", b"ON") {
            Some(MqttCommand::SetDevice { payload, .. }) => {
                assert_eq!(payload["state"], "ON");
            }
            other => panic!("unexpected command: {other:?}"),
        }
        assert!(parse_device_command("zigbee2mqtt", "zigbee2mqtt/kitchen", b"").is_none());
    }

    #[test]
    fn bridge_request_parsing_and_response_topic() {
        let (request, payload) = parse_bridge_request(
            "zigbee2mqtt",
            "zigbee2mqtt/bridge/request/device/reporting/read",
            br#"{"id":"0xec1bbdfffeaa66db","endpoint":1}"#,
        )
        .unwrap();
        assert_eq!(request, "device/reporting/read");
        assert_eq!(payload["endpoint"], 1);
        assert!(parse_bridge_request("zigbee2mqtt", "zigbee2mqtt/kitchen", b"{}").is_none());
        assert_eq!(
            bridge_response_topic("zigbee2mqtt", "device/reporting/read"),
            "zigbee2mqtt/bridge/response/device/reporting/read"
        );
    }

    #[test]
    fn endpoint_set_topic_selects_the_endpoint() {
        match parse_device_command(
            "zigbee2mqtt",
            "zigbee2mqtt/two_gang/l2/set",
            br#"{"state":"ON"}"#,
        ) {
            Some(MqttCommand::SetDevice {
                friendly_name,
                endpoint,
                payload,
                ..
            }) => {
                assert_eq!(friendly_name, "two_gang");
                assert_eq!(endpoint, Some(2));
                assert_eq!(payload["state"], "ON");
            }
            other => panic!("unexpected command: {other:?}"),
        }
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/room/light/l1/set", b"ON") {
            Some(MqttCommand::SetDevice {
                friendly_name,
                endpoint,
                ..
            }) => {
                assert_eq!(friendly_name, "room/light");
                assert_eq!(endpoint, Some(1));
            }
            other => panic!("unexpected command: {other:?}"),
        }
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/two_gang/set", b"ON") {
            Some(MqttCommand::SetDevice { endpoint, .. }) => assert_eq!(endpoint, None),
            other => panic!("unexpected command: {other:?}"),
        }
        assert_eq!(endpoint_index("l3"), Some(3));
        assert_eq!(endpoint_index("button"), Some(3));
        assert_eq!(endpoint_index("left"), None);
    }

    #[test]
    fn endpoint_state_topic_matches_discovery_contract() {
        assert_eq!(
            endpoint_state_topic("zigbee2mqtt", "two_gang", "l1"),
            "zigbee2mqtt/two_gang/l1"
        );
    }

    #[test]
    fn per_attribute_set_topics_wrap_the_value() {
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/curtain/set/position", b"40") {
            Some(MqttCommand::SetDevice {
                friendly_name,
                payload,
                endpoint,
                attribute,
            }) => {
                assert_eq!(friendly_name, "curtain");
                assert_eq!(endpoint, None);
                assert_eq!(attribute.as_deref(), Some("position"));
                assert_eq!(payload, json!({"position": 40}));
            }
            other => panic!("unexpected command: {other:?}"),
        }
        // Endpoint + attribute: `<base>/<name>/l2/set/tilt`.
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/curtain/l2/set/tilt", b"15") {
            Some(MqttCommand::SetDevice {
                friendly_name,
                payload,
                endpoint,
                attribute,
            }) => {
                assert_eq!(friendly_name, "curtain");
                assert_eq!(endpoint, Some(2));
                assert_eq!(attribute.as_deref(), Some("tilt"));
                assert_eq!(payload, json!({"tilt": 15}));
            }
            other => panic!("unexpected command: {other:?}"),
        }
        // JSON object values are parsed, raw strings kept as strings.
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/curtain/set/tilt", br#"{"x":1}"#) {
            Some(MqttCommand::SetDevice { payload, .. }) => {
                assert_eq!(payload["tilt"], json!({"x": 1}));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn per_attribute_get_topic_requests_one_key() {
        match parse_device_command("zigbee2mqtt", "zigbee2mqtt/curtain/get/position", b"") {
            Some(MqttCommand::GetDevice {
                friendly_name,
                payload,
                endpoint,
            }) => {
                assert_eq!(friendly_name, "curtain");
                assert_eq!(endpoint, None);
                assert_eq!(payload, json!({"position": ""}));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn bridge_topics_are_not_parsed_as_device_commands() {
        assert!(parse_device_command(
            "zigbee2mqtt",
            "zigbee2mqtt/bridge/request/device/remove",
            b"{}"
        )
        .is_none());
        assert!(parse_device_command("zigbee2mqtt", "zigbee2mqtt/bulb", b"{}").is_none());
    }

    async fn wait_for_publish(
        broker: &fake_broker::FakeBroker,
        predicate: impl Fn(&fake_broker::Published) -> bool,
    ) -> fake_broker::Published {
        for _ in 0..250 {
            if let Some(found) = broker.publishes().into_iter().find(|item| predicate(item)) {
                return found;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("publish not observed; saw {:?}", broker.publishes());
    }

    #[tokio::test]
    async fn large_bridge_devices_payload_is_published() {
        let broker = fake_broker::FakeBroker::start().await;
        let cfg = MqttConfig {
            server: "127.0.0.1".into(),
            port: broker.port,
            base_topic: "zigbee2mqtt".into(),
            client_id: format!("z2m-large-{}", std::process::id()),
            ..MqttConfig::default()
        };
        let (mqtt, _commands) = MqttBridge::connect(&cfg).await.expect("connect");
        let devices = serde_json::json!([
            {"ieee_address": "0x00124b0000000001", "description": "x".repeat(20 * 1024)}
        ]);
        mqtt.publish_bridge_devices(&devices)
            .await
            .expect("large bridge/devices publish must not be dropped");
        let published =
            wait_for_publish(&broker, |item| item.topic == "zigbee2mqtt/bridge/devices").await;
        assert!(published.payload.len() > 10 * 1024);
        assert!(published.retain);
    }

    #[tokio::test]
    async fn socket_event_loop_receives_commands_and_publishes_retained_cleanup() {
        let broker = fake_broker::FakeBroker::start().await;
        let cfg = MqttConfig {
            server: "127.0.0.1".into(),
            port: broker.port,
            base_topic: "zigbee2mqtt".into(),
            client_id: format!("z2m-test-{}", std::process::id()),
            ..MqttConfig::default()
        };
        let (mqtt, mut commands) = MqttBridge::connect(&cfg)
            .await
            .expect("connect to the in-process broker");

        // Broker -> client: endpoint set topic.
        broker
            .send("zigbee2mqtt/two_gang/l2/set", br#"{"state":"ON"}"#)
            .await;
        match tokio::time::timeout(Duration::from_secs(5), commands.recv())
            .await
            .expect("set command timing out")
            .expect("command channel")
        {
            MqttCommand::SetDevice {
                friendly_name,
                payload,
                endpoint,
                attribute,
            } => {
                assert_eq!(friendly_name, "two_gang");
                assert_eq!(endpoint, Some(2));
                assert_eq!(attribute, None);
                assert_eq!(payload["state"], "ON");
            }
            other => panic!("unexpected command: {other:?}"),
        }

        // Broker -> client: per-attribute get topic.
        broker.send("zigbee2mqtt/curtain/get/position", b"").await;
        match tokio::time::timeout(Duration::from_secs(5), commands.recv())
            .await
            .expect("get command timing out")
            .expect("command channel")
        {
            MqttCommand::GetDevice {
                friendly_name,
                payload,
                endpoint,
            } => {
                assert_eq!(friendly_name, "curtain");
                assert_eq!(endpoint, None);
                assert_eq!(payload, json!({"position": ""}));
            }
            other => panic!("unexpected command: {other:?}"),
        }

        // Broker -> client: config-switch per-attribute topic.
        broker
            .send("zigbee2mqtt/plug/set/window_detection", b"ON")
            .await;
        match tokio::time::timeout(Duration::from_secs(5), commands.recv())
            .await
            .expect("config switch command timing out")
            .expect("command channel")
        {
            MqttCommand::SetDevice {
                friendly_name,
                payload,
                endpoint,
                attribute,
            } => {
                assert_eq!(friendly_name, "plug");
                assert_eq!(endpoint, None);
                assert_eq!(attribute.as_deref(), Some("window_detection"));
                assert_eq!(payload, json!({"window_detection": "ON"}));
            }
            other => panic!("unexpected command: {other:?}"),
        }

        // Broker -> client: endpoint lock with the suffixed attribute.
        broker
            .send("zigbee2mqtt/front_door/l1/set/state_l1", b"LOCK")
            .await;
        match tokio::time::timeout(Duration::from_secs(5), commands.recv())
            .await
            .expect("lock command timing out")
            .expect("command channel")
        {
            MqttCommand::SetDevice {
                friendly_name,
                payload,
                endpoint,
                attribute,
            } => {
                assert_eq!(friendly_name, "front_door");
                assert_eq!(endpoint, Some(1));
                assert_eq!(attribute.as_deref(), Some("state_l1"));
                assert_eq!(payload, json!({"state_l1": "LOCK"}));
            }
            other => panic!("unexpected command: {other:?}"),
        }

        // Client -> broker: retained discovery cleanup over the real socket.
        mqtt.clear_ha_topic("homeassistant/cover/0x9a1122334455667f/cover_l1/config")
            .await
            .expect("cleanup publish");
        let cleanup = wait_for_publish(&broker, |item| {
            item.topic == "homeassistant/cover/0x9a1122334455667f/cover_l1/config"
        })
        .await;
        assert!(cleanup.payload.is_empty());
        assert_eq!(cleanup.qos, 1);
        assert!(cleanup.retain);
    }
}
