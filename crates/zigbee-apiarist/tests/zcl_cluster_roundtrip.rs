// Round-trip checks for every generated ZCL cluster definition.
//
// The catalog is generated from zigbee-herdsman v11.2.0; each cluster must at
// least serialize/parse a command and a read frame through the same codecs the
// controller uses.

use serde_json::{json, Value};
use zigbee_apiarist::zspec::zcl::utils::{ClusterKey, CustomClusters, Utils};
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType, Header};

#[test]
fn every_cluster_round_trips_a_command_and_read_frame() {
    let root: Value =
        serde_json::from_str(include_str!("../src/zspec/zcl/definition/clusters.json"))
            .expect("cluster catalog");
    let custom = CustomClusters::empty();
    let global_read = Utils::get_global_command("read");
    let mut commands_checked = 0usize;
    let mut reads_checked = 0usize;

    for (name, raw) in root.as_object().expect("clusters object") {
        let cluster = Utils::get_cluster(ClusterKey::Name(name.clone()), None, &custom);

        let commands = raw.get("commands").and_then(Value::as_object);
        if let Some(commands) = commands {
            for (command_name, command) in commands {
                if command
                    .get("parameters")
                    .and_then(Value::as_array)
                    .is_some_and(|parameters| !parameters.is_empty())
                {
                    continue;
                }
                let definition = Utils::get_cluster_command(&cluster, command_name.as_str());
                let frame = Frame::create_with_definitions(
                    cluster.clone(),
                    definition.clone(),
                    FrameType::SPECIFIC as u8,
                    Direction::CLIENT_TO_SERVER as u8,
                    true,
                    None,
                    1,
                    json!({}),
                    0,
                );
                let buffer = frame.to_buffer();
                let header = Header::from_buffer(&buffer).expect("header");
                let parsed = Frame::from_buffer(cluster.ID as u16, Some(header), &buffer, &custom);
                assert_eq!(parsed.cluster.name, *name, "{name}.{command_name}: cluster");
                assert_eq!(
                    parsed.command.ID, definition.ID,
                    "{name}.{command_name}: command"
                );
                commands_checked += 1;
            }
        }

        let first_attribute = cluster
            .attributes
            .values()
            .min_by_key(|attribute| attribute.ID);
        if let Some(attribute) = first_attribute {
            let frame = Frame::create_with_definitions(
                cluster.clone(),
                global_read.clone(),
                FrameType::GLOBAL as u8,
                Direction::CLIENT_TO_SERVER as u8,
                true,
                None,
                2,
                json!([{"attrId": attribute.ID}]),
                0,
            );
            let buffer = frame.to_buffer();
            let header = Header::from_buffer(&buffer).expect("header");
            let parsed = Frame::from_buffer(cluster.ID as u16, Some(header), &buffer, &custom);
            assert_eq!(parsed.cluster.name, *name, "{name}: read cluster");
            assert_eq!(
                parsed.payload[0]["attrId"],
                json!(attribute.ID),
                "{name}: read attrId"
            );
            reads_checked += 1;
        }
    }

    assert!(
        commands_checked > 50,
        "commands checked: {commands_checked}"
    );
    assert!(reads_checked > 100, "reads checked: {reads_checked}");
}
