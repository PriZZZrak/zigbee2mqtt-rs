//! Read a `database.db`, load devices/groups through the model and write it
//! back with the canonical Node-compatible encoder.
//!
//! Usage: cargo run -p zigbee-apiarist --example db_roundtrip -- <in> <out>

use std::sync::{Arc, Mutex};

use serde_json::Value;

use zigbee_apiarist::controller::database::Database;
use zigbee_apiarist::controller::model::device::Device;
use zigbee_apiarist::controller::model::entity::{inject_database, Entity};
use zigbee_apiarist::controller::model::group::Group;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("input database.db");
    let output = args.next().expect("output database.db");

    std::fs::copy(&input, &output).expect("copy database");
    Device::reset_cache();
    Group::reset_cache();
    inject_database(Arc::new(Mutex::new(Database::open(&output))));

    let content = std::fs::read_to_string(&input).expect("read database");
    for line in content.split('\n') {
        if line.is_empty() {
            continue;
        }
        let entry: Value = serde_json::from_str(line).expect("database line");
        if entry.get("type").and_then(Value::as_str) == Some("Group") {
            Group::test_from_database_entry(&entry).save(false);
        } else {
            Device::from_database_entry(&entry).save(false);
        }
    }
    Entity::database()
        .lock()
        .unwrap()
        .write()
        .expect("write database");
}
