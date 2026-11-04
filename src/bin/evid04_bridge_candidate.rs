//! EVID-04 bridge differential driver (harness only).
//!
//! Reads a fixture JSON object on stdin, feeds the same incoming ZCL frame to
//! the real bridge message path (`Bridge::process_zcl_message`) with an
//! in-process MQTT broker, and prints the published topic/payload:
//!
//! ```json
//! {"topic": "zigbee2mqtt/trv", "payload": {"local_temperature": 14, ...}}
//! ```
//!
//! Build with `--features test-support`. Environment: `EVID04_CACHE` points at a
//! staged converter package.
use std::io::Read;
use std::time::Duration;

use serde_json::{json, Map, Value};
use zigbee2mqtt_rs::bridge::Bridge;
use zigbee2mqtt_rs::config::{MqttConfig, OutputMode};
use zigbee2mqtt_rs::converters::external::ExternalConverter;
use zigbee2mqtt_rs::devices::{Device, DeviceRegistry};
use zigbee2mqtt_rs::mqtt::fake_broker::FakeBroker;
use zigbee2mqtt_rs::mqtt::MqttBridge;
use zigbee2mqtt_rs::zigbee::{EndpointDesc, IeeeAddr};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-bridge-candidate: {message}");
    std::process::exit(2);
}

fn number_u16(input: &Value, key: &str) -> Option<u16> {
    input
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let filter = std::env::var("EVID04_LOG").unwrap_or_else(|_| "warn".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_new(filter).unwrap_or_default())
        .with_writer(std::io::stderr)
        .try_init();

    let mut raw = String::new();
    std::io::stdin()
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| fail(format!("cannot read stdin: {error}")));
    let input: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|error| fail(format!("invalid fixture JSON: {error}")));

    let cache = std::env::var("EVID04_CACHE").unwrap_or_else(|_| fail("EVID04_CACHE is not set"));
    let external = ExternalConverter::new(&cache, "file://unused");

    let model = input
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| fail("fixture is missing 'model'"));
    let cluster_id = number_u16(&input, "cluster_id")
        .unwrap_or_else(|| fail("fixture is missing a valid 'cluster_id'"));
    let endpoint = number_u16(&input, "endpoint")
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(1);
    let link_quality = number_u16(&input, "linkquality")
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(0);
    let base_topic = input
        .get("base_topic")
        .and_then(|value| value.as_str())
        .unwrap_or("zigbee2mqtt");
    let friendly_name = input
        .get("friendly_name")
        .and_then(|value| value.as_str())
        .unwrap_or("device");
    let ieee = IeeeAddr::from_hex(
        input
            .get("ieee_addr")
            .and_then(|value| value.as_str())
            .unwrap_or("0x00124b0000000000"),
    )
    .unwrap_or_else(|| fail("invalid 'ieee_addr'"));
    let nwk_addr = number_u16(&input, "nwk_addr").unwrap_or(0x1234);

    let mut device = Device::new(ieee, nwk_addr);
    device.friendly_name = friendly_name.to_string();
    device.model = Some(model.to_string());
    device.manufacturer = input
        .get("manufacturer_name")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    device.power_source = input
        .get("power_source")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    device.interview_complete = true;
    if let Some(state) = input.get("state").and_then(|value| value.as_object()) {
        device.state = state.clone();
    }
    let endpoints: Vec<EndpointDesc> =
        match input.get("endpoints").and_then(|value| value.as_array()) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|entry| {
                    let id = entry
                        .get("id")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(1);
                    let input_clusters = entry
                        .get("input_clusters")
                        .and_then(|value| value.as_array())
                        .map(|clusters| {
                            clusters
                                .iter()
                                .filter_map(|value| value.as_u64())
                                .filter_map(|value| u16::try_from(value).ok())
                                .collect()
                        })
                        .unwrap_or_else(|| vec![cluster_id]);
                    EndpointDesc {
                        endpoint: id,
                        profile_id: 0x0104,
                        device_id: 0,
                        input_clusters,
                        output_clusters: vec![],
                    }
                })
                .collect(),
            _ => vec![EndpointDesc {
                endpoint: 1,
                profile_id: 0x0104,
                device_id: 0,
                input_clusters: vec![cluster_id],
                output_clusters: vec![],
            }],
        };
    device.endpoints = endpoints;
    let devices = DeviceRegistry::new();
    devices.add(device);

    let broker = FakeBroker::start().await;
    let cfg = MqttConfig {
        server: "127.0.0.1".into(),
        port: broker.port,
        base_topic: base_topic.to_string(),
        client_id: format!("evid04-bridge-{}", std::process::id()),
        ..MqttConfig::default()
    };
    let (mqtt, _commands) = MqttBridge::connect(&cfg)
        .await
        .unwrap_or_else(|error| fail(format!("cannot connect to the test broker: {error}")));

    // Build the same ZCL frame the radio would deliver.
    let msg_type = input
        .get("msg_type")
        .and_then(|value| value.as_str())
        .unwrap_or("attributeReport");
    let data = input
        .get("data")
        .and_then(|value| value.as_object())
        .cloned();
    let raw_payload: Option<Vec<u8>> = input
        .get("raw_payload")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_u64())
                .filter_map(|value| u8::try_from(value).ok())
                .collect()
        });
    let frame = match (raw_payload, data) {
        (Some(raw), _) => {
            let command = input
                .get("command_id")
                .and_then(|value| value.as_u64())
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or_else(|| fail("fixture is missing 'command_id'"));
            let mut frame = vec![0x15, 0x02, 0x10, 0x01, command];
            frame.extend_from_slice(&raw);
            frame
        }
        (None, Some(data)) => {
            // Cluster-specific command payloads: control, TSN, command.
            let command = input
                .get("command_id")
                .and_then(|value| value.as_u64())
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or_else(|| fail("cluster-command fixture is missing 'command_id'"));
            let mut frame = vec![0x15, 0x02, 0x10, 0x01, command];
            frame.extend_from_slice(&encode_cluster_command(&data));
            frame
        }
        (None, None) => {
            let attribute = input
                .get("attribute")
                .and_then(|value| value.as_str())
                .unwrap_or_else(|| fail("fixture is missing 'attribute' or 'data'"));
            let value = input
                .get("value")
                .cloned()
                .unwrap_or_else(|| fail("fixture is missing 'value'"));
            let attribute_id = external
                .attribute_id(cluster_id, attribute)
                .or_else(|| attribute.parse::<u16>().ok())
                .unwrap_or_else(|| fail(format!("cannot resolve attribute '{attribute}'")));
            let data_type = external
                .zcl_types()
                .attribute(cluster_id, attribute_id)
                .map(|info| info.data_type)
                .unwrap_or_else(|| fail(format!("no ZCL type for attribute '{attribute}'")));
            let encoded = zigbee2mqtt_rs::converters::zcl_types::encode_value(data_type, &value)
                .unwrap_or_else(|error| fail(format!("cannot encode attribute value: {error}")));
            // Global report/read-response frame: control, TSN, command id.
            let (control, command) = if msg_type == "readResponse" {
                (0x18u8, 0x01u8)
            } else {
                (0x18u8, 0x0Au8)
            };
            let mut frame = vec![control, 0x01, command];
            frame.extend_from_slice(&attribute_id.to_le_bytes());
            frame.push(u8::try_from(data_type).unwrap_or(0));
            frame.extend_from_slice(&encoded);
            frame
        }
    };

    Bridge::process_zcl_message_for_diff(
        &devices,
        &external,
        &mqtt,
        OutputMode::Json,
        nwk_addr,
        endpoint,
        cluster_id,
        link_quality,
        frame,
    )
    .await;

    let topic = format!("{base_topic}/{friendly_name}");
    let expect_no_publish =
        input.get("expect").and_then(|value| value.as_str()) == Some("no_publish");
    let mut published = None;
    let attempts = if expect_no_publish { 25 } else { 250 };
    for _ in 0..attempts {
        if let Some(item) = broker
            .publishes()
            .into_iter()
            .find(|item| item.topic == topic)
        {
            published = Some(item);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let payload = match published {
        Some(published) => serde_json::from_slice(&published.payload)
            .unwrap_or_else(|error| fail(format!("published payload is not JSON: {error}"))),
        None if expect_no_publish => Value::Null,
        None => fail(format!(
            "no state publish on {topic}; saw {:?}",
            broker.publishes()
        )),
    };
    println!(
        "{}",
        serde_json::to_string(&json!({"topic": topic, "payload": payload}))
            .unwrap_or_else(|_| "{}".into())
    );
}

/// Encode Tuya datapoint payloads (`seq` + `dp/datatype/length/data`).
fn encode_cluster_command(data: &Map<String, Value>) -> Vec<u8> {
    let sequence = data
        .get("seq")
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or(0);
    let mut out = sequence.to_le_bytes().to_vec();
    for point in data
        .get("dpValues")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let dp = point
            .get("dp")
            .and_then(|value| value.as_u64())
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(0);
        let datatype = point
            .get("datatype")
            .and_then(|value| value.as_u64())
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(0);
        let bytes: Vec<u8> = point
            .get("data")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_u64())
                    .filter_map(|value| u8::try_from(value).ok())
                    .collect()
            })
            .unwrap_or_default();
        out.push(dp);
        out.push(datatype);
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(&bytes);
    }
    out
}
