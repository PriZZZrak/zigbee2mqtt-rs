//! Model-layer transcript helper (audit CP-08).
//!
//! Runs the same Device/Endpoint/Group operations as
//! `tools/oracle/model-transcript.mjs` (upstream zigbee-herdsman model classes)
//! and prints the JSON transcript for comparison.
//!
//! Usage: cargo run -p zigbee-apiarist --example model_transcript

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group;

fn main() {
    let path = std::env::temp_dir().join(format!(
        "z2m-model-transcript-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    inject_database(Arc::new(Mutex::new(Database::open(&path))));
    Device::reset_cache();
    Group::reset_cache();

    let mut transcript: Vec<Value> = Vec::new();
    macro_rules! record {
        ($op:expr, $body:block) => {{
            let result = (|| -> Result<Value, String> { $body })();
            match result {
                Ok(value) => transcript.push(json!({"op": $op, "value": value})),
                Err(error) => transcript.push(json!({"op": $op, "error": error})),
            }
        }};
    }

    let ieee = "0x0017880104e45519";
    record!("create-device", {
        let device = Device::create(
            DeviceType::Router,
            ieee,
            1234,
            None,
            Some("Test".to_string()),
            Some("Mains (single phase)".to_string()),
            Some("TEST".to_string()),
            InterviewState::Successful,
            None,
        )?;
        Ok(json!({
            "type": match device.r#type() {
                DeviceType::Router => "Router",
                DeviceType::Coordinator => "Coordinator",
                DeviceType::EndDevice => "EndDevice",
                DeviceType::GreenPower => "GreenPower",
                DeviceType::Unknown => "Unknown",
            },
            "ieeeAddr": device.ieee_addr(),
            "networkAddress": device.network_address(),
            "manufacturerName": device.manufacturer_name(),
            "modelID": device.model_id(),
            "interviewState": device.interview_state().as_str(),
        }))
    });
    record!("create-duplicate-device", {
        Device::create(
            DeviceType::Router,
            ieee,
            1235,
            None,
            Some("Test".to_string()),
            Some("Mains (single phase)".to_string()),
            Some("TEST".to_string()),
            InterviewState::Successful,
            None,
        )?;
        Ok(Value::Null)
    });

    let device = Device::by_ieee_addr(ieee, false).expect("device");
    record!("endpoint1", {
        let endpoint = device.create_endpoint(1)?;
        Ok(json!({
            "ID": endpoint.ID,
            "deviceIeeeAddress": endpoint.device_ieee_address.lock().unwrap().clone(),
            "deviceNetworkAddress": *endpoint.device_network_address.lock().unwrap(),
            "profileID": endpoint.profile_id.lock().unwrap().map(|value| json!(value)).unwrap_or(Value::Null),
            "deviceID": endpoint.device_id.lock().unwrap().map(|value| json!(value)).unwrap_or(Value::Null),
            "inputClusters": endpoint.input_clusters.lock().unwrap().clone(),
            "outputClusters": endpoint.output_clusters.lock().unwrap().clone(),
        }))
    });
    record!("endpoint-count", {
        let _ = device.create_endpoint(2)?;
        Ok(json!(device.endpoints().len()))
    });
    record!("create-duplicate-endpoint", {
        device.create_endpoint(1)?;
        Ok(Value::Null)
    });
    record!("get-endpoint", {
        Ok(json!(device.get_endpoint(2).expect("endpoint").ID))
    });
    let endpoint2 = device.get_endpoint(2).expect("endpoint2");
    endpoint2.input_clusters.lock().unwrap().extend([6u16, 8]);
    endpoint2.output_clusters.lock().unwrap().push(25);
    record!("endpoint-clusters", {
        Ok(json!({
            "input": endpoint2.input_clusters.lock().unwrap().clone(),
            "output": endpoint2.output_clusters.lock().unwrap().clone(),
        }))
    });

    record!("create-group", {
        let group = Group::create(1)?;
        Ok(json!({"groupID": group.group_id, "members": group.members().len()}))
    });
    record!("create-duplicate-group", {
        Group::create(1)?;
        Ok(Value::Null)
    });
    record!("create-invalid-group-low", {
        Group::create(0)?;
        Ok(Value::Null)
    });
    record!("create-invalid-group-high", {
        Group::create(0xfff8)?;
        Ok(Value::Null)
    });

    let group = Group::by_group_id(1).expect("group");
    let endpoint1 = device.get_endpoint(1).expect("endpoint1");
    group.add_member(&endpoint1);
    group.add_member(&endpoint1);
    record!("group-add-duplicate", {
        Ok(json!(group
            .members()
            .iter()
            .map(|member| member.ID)
            .collect::<Vec<_>>()))
    });
    group.add_member(&endpoint2);
    record!("group-members", {
        Ok(json!(group
            .members()
            .iter()
            .map(|member| json!([
                member.device_ieee_address.lock().unwrap().clone(),
                member.ID
            ]))
            .collect::<Vec<_>>()))
    });
    record!("group-has-member", {
        Ok(json!(group.has_member(&endpoint2)))
    });
    group.remove_member(&endpoint1);
    record!("group-remove", {
        Ok(json!(group
            .members()
            .iter()
            .map(|member| member.ID)
            .collect::<Vec<_>>()))
    });
    record!("group-by-id", {
        Ok(json!(Group::by_group_id(1).expect("group").members().len()))
    });

    let _ = std::fs::remove_file(&path);
    println!("{}", serde_json::to_string_pretty(&transcript).unwrap());
}
