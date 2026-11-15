// Original: .upstream/zigbee2mqtt/lib/util/settings.ts + lib/controller.ts database loading
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Loads the configuration and database captured from the real router
// (`ref/hil-2026-11-09`) through the Rust port and checks that they round-trip
// without errors.

use std::path::PathBuf;

use serde_json::Value;
use zigbee_apiarist::controller::database::Database;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;
use zigbee_apiarist::controller::model::test_support::serial;

use z2m_rs::util::settings::Settings;
use z2m_rs::util::yaml;

fn hil_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ref/hil-2026-11-09")
}

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "z2m-router-artifacts-{name}-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn loads_the_router_configuration_without_validation_errors() {
    let _guard = serial();
    let path = hil_dir().join("z2m-run/configuration.yaml");
    let configuration = yaml::read(&path).expect("router configuration.yaml parses");

    let mut settings = Settings::from_config(configuration.clone());
    let applied = settings.get();

    assert_eq!(applied["mqtt"]["server"], "mqtt://192.168.0.1");
    assert_eq!(applied["mqtt"]["base_topic"], "zigbee2mqtt-hil");
    assert_eq!(applied["serial"]["port"], "/dev/ttyUSB0");
    assert_eq!(applied["serial"]["adapter"], "zstack");
    assert_eq!(applied["advanced"]["channel"], 11);
    assert_eq!(applied["frontend"]["enabled"], false);
    assert_eq!(applied["homeassistant"]["enabled"], false);

    let errors = settings.validate();
    assert!(errors.is_empty(), "validation errors: {errors:?}");

    // Applying the same configuration over the loaded settings must not fail.
    settings.apply(configuration).expect("configuration re-applies");
}

#[test]
fn loads_the_router_database_and_round_trips_it() {
    let _guard = serial();
    let source = hil_dir().join("router-node/database.db");
    let database = Database::open(&source);
    let entries: Vec<Value> = database.all_entries().cloned().collect();
    assert_eq!(entries.len(), 4, "coordinator + three devices expected");

    ZhDevice::reset_cache();
    let mut coordinator = 0;
    let mut devices = 0;
    for entry in &entries {
        if entry["type"] == "Group" {
            continue;
        }
        let device = ZhDevice::from_database_entry(entry);
        if device.r#type() == zigbee_apiarist::controller::database::DeviceType::Coordinator {
            coordinator += 1;
            assert_eq!(device.ieee_addr(), "0x00124b0024c2d400");
            assert_eq!(device.endpoints().len(), 15);
        } else {
            devices += 1;
            assert!(!device.ieee_addr().is_empty());
            assert!(device.model_id().is_some());
            assert!(!device.endpoints().is_empty());
        }
    }
    assert_eq!(coordinator, 1);
    assert_eq!(devices, 3);

    // Write the entries into a fresh database and re-read them.
    let target = temp("roundtrip");
    let mut copy = Database::open(&target);
    for entry in &entries {
        copy.insert(entry.clone()).expect("entry inserts");
    }
    copy.write().expect("database writes");
    drop(copy);

    let reloaded = Database::open(&target);
    let reloaded_entries: Vec<Value> = reloaded.all_entries().cloned().collect();
    assert_eq!(reloaded_entries.len(), entries.len());
    let mut source_ids: Vec<u64> = entries.iter().filter_map(|entry| entry["id"].as_u64()).collect();
    let mut reloaded_ids: Vec<u64> = reloaded_entries
        .iter()
        .filter_map(|entry| entry["id"].as_u64())
        .collect();
    source_ids.sort_unstable();
    reloaded_ids.sort_unstable();
    assert_eq!(reloaded_ids, source_ids);

    for entry in &entries {
        let id = entry["id"].as_u64().unwrap();
        let reloaded = reloaded_entries
            .iter()
            .find(|candidate| candidate["id"].as_u64() == Some(id))
            .expect("entry survives the round-trip");
        assert_eq!(reloaded["ieeeAddr"], entry["ieeeAddr"]);
        assert_eq!(reloaded["type"], entry["type"]);
        assert_eq!(reloaded["endpoints"], entry["endpoints"]);
    }

    let _ = std::fs::remove_file(&target);
}
