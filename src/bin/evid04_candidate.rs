//! EVID-04 candidate driver.
//!
//! Reads a fixture JSON object on stdin and prints JSON in the same shape as
//! `scripts/evid04_node_reference.js`:
//!   - mode `fromZigbee` (default): converter output map.
//!   - mode `convertSet`: `{state, ops}` with the recorded ZCL operations.
//!
//! Environment: `EVID04_CACHE` must point at a staged converter package
//! (`registry.json`, `zcl.json`, `zcl-commands.json`, `devices/`, ...).
use std::io::Read;

use serde_json::{json, Map, Value};
use zigbee2mqtt_rs::converters::external::{
    ConvertSetOp, ExternalConverter, ExternalDeviceInfo, IncomingMessageInfo,
};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-candidate: {message}");
    std::process::exit(2);
}

fn options(manufacturer_code: Option<u16>, disable_default_response: bool) -> Value {
    json!({
        "manufacturerCode": manufacturer_code,
        "disableDefaultResponse": disable_default_response,
    })
}

fn main() {
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
    let converter = ExternalConverter::new(&cache, "file://unused");

    let model = input
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| fail("fixture is missing 'model'"));
    let cluster_id = input
        .get("cluster_id")
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or_else(|| fail("fixture is missing a valid 'cluster_id'"));
    let endpoint = input
        .get("endpoint")
        .and_then(|value| value.as_u64())
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(1);
    let endpoint_name = input
        .get("endpoint_name")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let msg_type = input
        .get("msg_type")
        .and_then(|value| value.as_str())
        .unwrap_or("attributeReport");
    let linkquality = input
        .get("linkquality")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u8;

    let device = ExternalDeviceInfo {
        model_id: Some(model.to_string()),
        manufacturer_id: input
            .get("manufacturer_id")
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok()),
        manufacturer_name: input
            .get("manufacturer_name")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        ieee_addr: input
            .get("ieee_addr")
            .and_then(|value| value.as_str())
            .unwrap_or("0x00124b0000000000")
            .to_string(),
        power_source: input
            .get("power_source")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        device_type: input
            .get("device_type")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        endpoint_ids: vec![1, 2, 3, 4],
        ..ExternalDeviceInfo::default()
    };
    if std::env::var("EVID04_DEBUG").is_ok() {
        eprintln!(
            "evid04-candidate: selected {:?}",
            converter.definition_for(model, &device)
        );
    }
    let state = input
        .get("state")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();

    let mode = input
        .get("mode")
        .and_then(|value| value.as_str())
        .unwrap_or("fromZigbee");

    if mode == "fromZigbee" {
        let message = IncomingMessageInfo {
            endpoint,
            linkquality,
            group_id: 0,
            tsn: input
                .get("tsn")
                .and_then(|value| value.as_u64())
                .and_then(|value| u8::try_from(value).ok())
                .or(Some(1)),
            manufacturer_code: input
                .get("manufacturer_code")
                .and_then(|value| value.as_u64())
                .and_then(|value| u16::try_from(value).ok()),
        };
        let converted = match input.get("data").and_then(|value| value.as_object()) {
            Some(data) => converter.convert_from_zigbee_session_with_data_full(
                model, cluster_id, msg_type, data, &device, &message, &state,
            ),
            None => {
                let attribute = input
                    .get("attribute")
                    .and_then(|value| value.as_str())
                    .unwrap_or_else(|| fail("fixture is missing 'attribute' or 'data'"));
                let value = input
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| fail("fixture is missing 'value'"));
                let attribute_id = converter
                    .attribute_id(cluster_id, attribute)
                    .or_else(|| attribute.parse::<u16>().ok())
                    .unwrap_or_else(|| fail(format!("cannot resolve attribute '{attribute}'")));
                converter.convert_from_zigbee_session_full(
                    model,
                    cluster_id,
                    msg_type,
                    &[(attribute_id, value)],
                    &device,
                    &message,
                    &state,
                )
            }
        };
        let state_only = converted.map(|frame| frame.state).unwrap_or_default();
        println!(
            "{}",
            serde_json::to_string(&json!(state_only)).unwrap_or_else(|_| "{}".into())
        );
        return;
    }

    if mode == "fromZigbeeOps" {
        let message = IncomingMessageInfo {
            endpoint,
            linkquality,
            group_id: 0,
            tsn: input
                .get("tsn")
                .and_then(|value| value.as_u64())
                .and_then(|value| u8::try_from(value).ok())
                .or(Some(1)),
            manufacturer_code: input
                .get("manufacturer_code")
                .and_then(|value| value.as_u64())
                .and_then(|value| u16::try_from(value).ok()),
        };
        let frame = match input.get("data").and_then(|value| value.as_object()) {
            Some(data) => converter.convert_from_zigbee_session_with_data_full(
                model, cluster_id, msg_type, data, &device, &message, &state,
            ),
            None => {
                let attribute = input
                    .get("attribute")
                    .and_then(|value| value.as_str())
                    .unwrap_or_else(|| fail("fixture is missing 'attribute' or 'data'"));
                let value = input
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| fail("fixture is missing 'value'"));
                let attribute_id = converter
                    .attribute_id(cluster_id, attribute)
                    .or_else(|| attribute.parse::<u16>().ok())
                    .unwrap_or_else(|| fail(format!("cannot resolve attribute '{attribute}'")));
                converter.convert_from_zigbee_session_full(
                    model,
                    cluster_id,
                    msg_type,
                    &[(attribute_id, value)],
                    &device,
                    &message,
                    &state,
                )
            }
        }
        .unwrap_or_default();
        let commands: Vec<Value> = frame
            .commands
            .iter()
            .map(|command| {
                json!({
                    "cluster": command.cluster,
                    "command": command.command,
                    "payload": command.payload,
                    "options": {
                        "manufacturerCode": command.manufacturer_code,
                        "disableDefaultResponse": command.disable_default_response,
                    },
                })
            })
            .collect();
        let writes: Vec<Value> = frame
            .writes
            .iter()
            .map(|write| {
                let attributes: Map<String, Value> = write.attributes.iter().cloned().collect();
                json!({
                    "cluster": write.cluster,
                    "attributes": attributes,
                    "options": {
                        "manufacturerCode": write.manufacturer_code,
                        "disableDefaultResponse": write.disable_default_response,
                    },
                })
            })
            .collect();
        let reads: Vec<Value> = frame
            .reads
            .iter()
            .map(|read| {
                json!({
                    "cluster": read.cluster,
                    "attributes": read.attributes,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&json!({
                "state": frame.state,
                "commands": commands,
                "writes": writes,
                "reads": reads,
            }))
            .unwrap_or_else(|_| "{}".into())
        );
        return;
    }

    if mode == "convertSet" {
        let key = input
            .get("key")
            .and_then(|value| value.as_str())
            .unwrap_or_else(|| fail("convertSet fixture is missing 'key'"));
        let value = input
            .get("value")
            .cloned()
            .unwrap_or_else(|| fail("convertSet fixture is missing 'value'"));
        let Some(outcome) = converter.convert_set(
            model,
            endpoint,
            endpoint_name.as_deref(),
            key,
            &value,
            &device,
            &state,
        ) else {
            fail(format!("no conversion outcome for '{key}'"));
        };
        if !outcome.handled {
            fail(format!("no toZigbee converter handles '{key}'"));
        }
        let mut ops = Vec::new();
        for op in &outcome.ops {
            match op {
                ConvertSetOp::Write {
                    cluster,
                    manufacturer_code,
                    disable_default_response,
                    attributes,
                } => {
                    let attributes: Map<String, Value> = attributes.iter().cloned().collect();
                    ops.push(json!({
                        "op": "write",
                        "cluster": cluster,
                        "attributes": attributes,
                        "options": options(*manufacturer_code, *disable_default_response),
                    }));
                }
                ConvertSetOp::Command {
                    cluster,
                    command,
                    manufacturer_code,
                    disable_default_response,
                    payload,
                } => {
                    ops.push(json!({
                        "op": "command",
                        "cluster": cluster,
                        "command": command,
                        "payload": payload,
                        "options": options(*manufacturer_code, *disable_default_response),
                    }));
                }
                ConvertSetOp::Read {
                    cluster,
                    attributes,
                } => {
                    ops.push(json!({
                        "op": "read",
                        "cluster": cluster,
                        "attributes": attributes,
                    }));
                }
                ConvertSetOp::Unsupported { kind } => {
                    fail(format!("unsupported converter operation: {kind}"));
                }
            }
        }
        println!(
            "{}",
            serde_json::to_string(&json!({"state": outcome.state, "ops": ops}))
                .unwrap_or_else(|_| "{}".into())
        );
        return;
    }

    fail(format!("unknown mode '{mode}'"));
}
