//! Generated-definition differential driver (harness only).
//!
//! Reads a device fixture on stdin and prints the `bridge/devices` definition
//! payload the bridge would publish for a device with no matching definition
//! (upstream `findByDevice(..., true)` -> `lib/generateDefinition.js`).
use std::io::Read;

use serde_json::{json, Value};
use zigbee2mqtt_rs::converters::external::{
    ExternalConverter, ExternalDeviceInfo, ExternalEndpoint,
};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-generated-candidate: {message}");
    std::process::exit(2);
}

fn u16_list(value: Option<&Value>) -> Vec<u16> {
    value
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_u64())
                .filter_map(|value| u16::try_from(value).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn main() {
    let filter = std::env::var("EVID04_LOG").unwrap_or_else(|_| "warn".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_new(filter).unwrap_or_default())
        .with_writer(std::io::stderr)
        .try_init();

    let cache = std::env::var("EVID04_CACHE").unwrap_or_else(|_| fail("EVID04_CACHE is not set"));
    let mut raw = String::new();
    std::io::stdin()
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| fail(format!("cannot read stdin: {error}")));
    let input: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|error| fail(format!("invalid fixture JSON: {error}")));
    let device_json = input.get("device").unwrap_or(&input);

    let model_id = device_json
        .get("model_id")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let endpoints: Vec<ExternalEndpoint> = device_json
        .get("endpoints")
        .and_then(|value| value.as_array())
        .map(|endpoints| {
            endpoints
                .iter()
                .map(|endpoint| ExternalEndpoint {
                    id: endpoint
                        .get("id")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| u8::try_from(value).ok())
                        .unwrap_or(1),
                    profile_id: 260,
                    device_id: 0,
                    input_clusters: u16_list(endpoint.get("input_clusters")),
                    output_clusters: u16_list(endpoint.get("output_clusters")),
                })
                .collect()
        })
        .unwrap_or_default();
    let device = ExternalDeviceInfo {
        model_id: Some(model_id.to_string()),
        manufacturer_name: device_json
            .get("manufacturer_name")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        manufacturer_id: device_json
            .get("manufacturer_id")
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok()),
        ieee_addr: device_json
            .get("ieee_addr")
            .and_then(|value| value.as_str())
            .unwrap_or("0x00124b0000000000")
            .to_string(),
        power_source: device_json
            .get("power_source")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        device_type: device_json
            .get("device_type")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        software_build_id: None,
        application_version: None,
        date_code: None,
        hardware_version: None,
        stack_version: None,
        zcl_version: None,
        endpoint_ids: endpoints.iter().map(|endpoint| endpoint.id).collect(),
        endpoints,
    };

    let converter = ExternalConverter::new(&cache, "file://unused");
    let payload = converter
        .generated_definition_payload(&device)
        .unwrap_or_else(|| fail("definition generation failed"));
    println!(
        "{}",
        serde_json::to_string(&json!({
            "definition": payload,
            "model_id": model_id,
        }))
        .unwrap_or_else(|_| "{}".into())
    );
}
