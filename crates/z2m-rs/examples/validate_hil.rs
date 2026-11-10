// Validates the already-ported pure layers against real router/adapter data.
//
// Usage:
//   Z2M_HIL_DIR=ref/hil-2026-11-09 cargo run -p z2m-rs --example validate_hil

use std::path::PathBuf;

use serde_json::{json, Value};
use z2m_rs::util::{settings::Settings, yaml};
use zigbee_apiarist::adapter::z_stack::structs::entries::nib::nib;
use zigbee_apiarist::adapter::z_stack::structs::entries::nwk_key_descriptor::nwk_key_descriptor;
use zigbee_apiarist::adapter::z_stack::utils::channel_list::unpack_channel_list;
use zigbee_apiarist::controller::database::Database;
use zigbee_apiarist::utils::backup::{from_unified_backup, to_unified_backup};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).expect("hex"))
        .collect()
}

fn nv(dump: &Value, name: &str) -> Vec<u8> {
    unhex(
        dump["nvItems"][name]["hex"]
            .as_str()
            .unwrap_or_else(|| panic!("missing NV item {name}")),
    )
}

fn main() {
    let dir = PathBuf::from(std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR"));
    let router = dir.join("router-node");

    // 1. configuration.yaml through the settings port.
    let configuration = yaml::read(router.join("configuration.yaml")).expect("configuration");
    let settings = Settings::with_config(configuration).get();
    assert_eq!(settings["serial"]["port"], "/dev/ttyUSB0");
    assert_eq!(settings["mqtt"]["base_topic"], "zigbee2mqtt");
    assert_eq!(settings["advanced"]["pan_id"], 6754);
    assert_eq!(
        settings["advanced"]["network_key"],
        json!([1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13])
    );
    assert_eq!(
        settings["devices"].as_object().expect("devices").len(),
        3,
        "router configuration should contain 3 devices"
    );
    println!("settings: ok");

    // 2. database.db through the Database port.
    let database = Database::open(router.join("database.db"));
    assert_eq!(database.all_entries().count(), 4);
    assert_eq!(
        database
            .get_entries_iterator(&[
                zigbee_apiarist::controller::database::EntityType::DeviceType(
                    zigbee_apiarist::controller::database::DeviceType::Coordinator
                )
            ])
            .count(),
        1
    );
    assert_eq!(
        database
            .get_entries_iterator(&[
                zigbee_apiarist::controller::database::EntityType::DeviceType(
                    zigbee_apiarist::controller::database::DeviceType::EndDevice
                )
            ])
            .count(),
        1
    );
    assert_eq!(
        database
            .get_entries_iterator(&[
                zigbee_apiarist::controller::database::EntityType::DeviceType(
                    zigbee_apiarist::controller::database::DeviceType::Router
                )
            ])
            .count(),
        2
    );
    println!("database: ok (4 entries)");

    // 3. coordinator_backup.json through fromUnifiedBackup/toUnifiedBackup.
    let backup_json: Value = serde_json::from_str(
        &std::fs::read_to_string(router.join("coordinator_backup.json")).expect("backup"),
    )
    .expect("backup json");
    let backup = from_unified_backup(&backup_json).expect("from unified backup");
    assert_eq!(backup.network_options.pan_id, 0x1a62);
    assert_eq!(backup.logical_channel, 11);
    assert_eq!(backup.network_options.channel_list, vec![11]);
    assert_eq!(
        hex(&backup.network_options.network_key),
        "01030507090b0d0f00020406080a0c0d"
    );
    assert_eq!(hex(&backup.coordinator_ieee_address), "00124b0024c2d400");
    assert_eq!(backup.devices.len(), 9);
    assert_eq!(backup.znp_version, Some(1));

    let unified = to_unified_backup(&backup);
    assert_eq!(unified["metadata"]["source"], "zigbee-herdsman@11.2.0");
    assert_eq!(unified["pan_id"], "1a62");
    assert_eq!(unified["channel"], 11);
    assert_eq!(unified["devices"].as_array().expect("devices").len(), 9);
    println!("backup: ok (source={})", unified["metadata"]["source"]);

    // 4. Real adapter NV dump through Struct/Nib/NwkKeyDescriptor.
    let dump: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("adapter-dump.json")).expect("dump"),
    )
    .expect("dump json");

    let nib_bytes = nv(&dump, "NIB");
    assert_eq!(nib_bytes.len(), 116, "NIB should be aligned in NV");
    let nib = nib(Some(&nib_bytes));
    assert_eq!(nib.get_u16("nwkPanId"), Some(0x1a62));
    assert_eq!(nib.get_u8("nwkLogicalChannel"), Some(11));
    assert_eq!(
        unpack_channel_list(nib.get_u32("channelList").expect("channelList")),
        vec![11]
    );
    assert_eq!(
        hex(&nib.get_bytes("extendedPANID").expect("extendedPANID")),
        "00124b0024c2d400"
    );
    assert_eq!(nib.get_u8("SecurityLevel"), Some(5));
    assert_eq!(nib.get_u8("nwkUpdateId"), Some(0));

    let active_key = nv(&dump, "NWK_ACTIVE_KEY_INFO");
    let descriptor = nwk_key_descriptor(Some(&active_key));
    assert_eq!(
        hex(&descriptor.get_bytes("key").expect("active key")),
        "01030507090b0d0f00020406080a0c0d"
    );
    assert_eq!(descriptor.get_u8("keySeqNum"), Some(0));
    assert_eq!(
        hex(&nv(&dump, "PRECFGKEY")),
        "01030507090b0d0f00020406080a0c0d"
    );
    assert_eq!(
        hex(&nv(&dump, "EXTADDR")),
        "00d4c224004b1200",
        "EXTADDR is little-endian in NV"
    );
    println!("adapter NV: ok (NIB/keys match router backup)");

    println!("all real-data checks passed");
}
