use std::path::Path;

use converter_artifact::{
    ArtifactIndex, DeviceEndpoint, DeviceProfile, DeviceType, Resolution, from_binary, from_json,
};
use serde_json::Value;

fn arg(name: &str) -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next().expect("value");
        }
    }
    panic!("missing {name}");
}

fn device_type(value: Option<&Value>) -> DeviceType {
    match value.and_then(Value::as_str) {
        Some("Coordinator") => DeviceType::Coordinator,
        Some("Router") => DeviceType::Router,
        Some("EndDevice") => DeviceType::EndDevice,
        _ => DeviceType::Unknown,
    }
}

fn endpoint(value: &Value) -> DeviceEndpoint {
    DeviceEndpoint {
        id: value
            .get("ID")
            .and_then(Value::as_u64)
            .map(|value| value as u8),
        profile_id: value
            .get("profileID")
            .and_then(Value::as_u64)
            .map(|value| value as u16),
        device_id: value
            .get("deviceID")
            .and_then(Value::as_u64)
            .map(|value| value as u16),
        input_clusters: value
            .get("inputClusters")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|value| value as u16)
                    .collect()
            })
            .unwrap_or_default(),
        output_clusters: value
            .get("outputClusters")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|value| value as u16)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn profile(value: &Value) -> DeviceProfile {
    DeviceProfile {
        ieee_addr: value
            .get("ieeeAddr")
            .and_then(Value::as_str)
            .map(str::to_owned),
        r#type: device_type(value.get("type")),
        model_id: value
            .get("modelID")
            .and_then(Value::as_str)
            .map(str::to_owned),
        manufacturer_name: value
            .get("manufacturerName")
            .and_then(Value::as_str)
            .map(str::to_owned),
        manufacturer_id: value
            .get("manufacturerID")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        power_source: value
            .get("powerSource")
            .and_then(Value::as_str)
            .map(str::to_owned),
        application_version: value
            .get("applicationVersion")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        stack_version: value
            .get("stackVersion")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        hardware_version: value
            .get("hardwareVersion")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        zcl_version: value
            .get("zclVersion")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        date_code: value
            .get("dateCode")
            .and_then(Value::as_str)
            .map(str::to_owned),
        software_build_id: value
            .get("softwareBuildID")
            .and_then(Value::as_str)
            .map(str::to_owned),
        endpoints: value
            .get("endpoints")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(endpoint).collect())
            .unwrap_or_default(),
    }
}

fn identity(value: &Value) -> Option<Value> {
    value.as_object()?;
    Some(serde_json::json!({
        "model": value.get("model"),
        "vendor": value.get("vendor"),
        "description": value.get("description"),
        "version": value.get("version"),
    }))
}

fn resolved(resolution: Option<Resolution>) -> Option<Value> {
    let resolution = resolution?;
    Some(serde_json::json!({
        "model": resolution.model,
        "vendor": resolution.vendor,
        "description": resolution.description,
        "version": resolution.version,
    }))
}

fn compare(name: &str, index: &ArtifactIndex, cases: &[Value]) -> usize {
    let mut failures = 0;
    for case in cases {
        let Some(profile_value) = case.get("profile") else {
            continue;
        };
        let expected = case.get("expected").map(identity).unwrap_or(None);
        let actual = resolved(index.resolve(&profile(profile_value)));
        if actual != expected {
            failures += 1;
            if failures <= 20 {
                eprintln!(
                    "{name} mismatch {}\n expected {expected:?}\n actual   {actual:?}\n profile {profile_value}",
                    case.get("id").and_then(Value::as_str).unwrap_or("?")
                );
            }
        }
    }
    failures
}

fn main() {
    let json_path = arg("--index-json");
    let binary_path = arg("--index-bin");
    let oracle_path = arg("--oracle");
    let parse_started = std::time::Instant::now();
    let json = from_json(&std::fs::read(&json_path).expect("json")).expect("json index");
    let json_ms = parse_started.elapsed().as_secs_f64() * 1000.0;
    let parse_started = std::time::Instant::now();
    let binary = from_binary(&std::fs::read(&binary_path).expect("binary")).expect("binary index");
    let binary_ms = parse_started.elapsed().as_secs_f64() * 1000.0;
    let oracle: Value =
        serde_json::from_slice(&std::fs::read(&oracle_path).expect("oracle")).expect("oracle json");
    let cases = oracle
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    eprintln!(
        "cases {} json {} defs/{:?} in {:.2}ms, binary {} defs/{:?} in {:.2}ms",
        cases.len(),
        json.definitions.len(),
        json.keys.len(),
        json_ms,
        binary.definitions.len(),
        binary.keys.len(),
        binary_ms
    );
    let json_failures = compare("json", &json, &cases);
    let binary_failures = compare("binary", &binary, &cases);
    println!(
        "{}",
        serde_json::json!({
            "cases": cases.len(),
            "jsonFailures": json_failures,
            "binaryFailures": binary_failures,
            "jsonParseMs": json_ms,
            "binaryParseMs": binary_ms,
            "jsonBytes": std::fs::metadata(Path::new(&json_path)).map(|m| m.len()).unwrap_or(0),
            "binaryBytes": std::fs::metadata(Path::new(&binary_path)).map(|m| m.len()).unwrap_or(0),
        })
    );
    if json_failures != 0 || binary_failures != 0 {
        std::process::exit(1);
    }
}
