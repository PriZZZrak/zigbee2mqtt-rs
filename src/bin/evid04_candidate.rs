//! EVID-04 candidate driver.
//!
//! Reads a fixture JSON object on stdin and prints the converted state as JSON.
//! It exists only for differential runs against the pinned Node reference; it is
//! not part of the bridge CLI surface.
//!
//! Environment: `EVID04_CACHE` must point at a staged converter package
//! (`registry.json`, `zcl.json`, `zcl-commands.json`, `devices/`, ...).
use std::io::Read;

use serde_json::json;
use zigbee2mqtt_rs::converters::external::{ExternalConverter, ExternalDeviceInfo};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-candidate: {message}");
    std::process::exit(2);
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
    let input: serde_json::Value = serde_json::from_str(&raw)
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
    let msg_type = input
        .get("msg_type")
        .and_then(|value| value.as_str())
        .unwrap_or("attributeReport");
    let attribute = input
        .get("attribute")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| fail("fixture is missing 'attribute'"));
    let value = input
        .get("value")
        .cloned()
        .unwrap_or_else(|| fail("fixture is missing 'value'"));

    let attribute_id = converter
        .attribute_id(cluster_id, attribute)
        .or_else(|| attribute.parse::<u16>().ok())
        .unwrap_or_else(|| fail(format!("cannot resolve attribute '{attribute}'")));

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
        endpoint_ids: vec![1],
        ..ExternalDeviceInfo::default()
    };
    let state = input
        .get("state")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();

    let converted = converter.convert_from_zigbee(
        model,
        cluster_id,
        msg_type,
        &[(attribute_id, value)],
        &device,
        &state,
    );
    println!(
        "{}",
        serde_json::to_string(&json!(converted.unwrap_or_default()))
            .unwrap_or_else(|_| "{}".into())
    );
}
