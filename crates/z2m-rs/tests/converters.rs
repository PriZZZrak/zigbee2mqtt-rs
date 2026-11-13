// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/index.js
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// Shared converter host: the registry starts empty (R1 has no converter
// database), so lookups return the upstream "not found" fallback. Registered
// definitions exercise index/fingerprint/white-label lookup and the device
// definition fallbacks.

use std::sync::{Arc, Mutex};

use serde_json::json;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;

use z2m_rs::converters::{Definition, DefinitionRegistry, Fingerprint, WhiteLabel};
use z2m_rs::model::Device;
use z2m_rs::util::settings::Settings;

fn zh_device() -> Arc<ZhDevice> {
    ZhDevice::from_database_entry(&json!({
        "id": 1,
        "type": "Router",
        "ieeeAddr": "0x0017880104e45519",
        "nwkAddr": 1234,
        "manufId": 4476,
        "manufName": "Test",
        "modelId": "TEST",
        "interviewCompleted": true,
        "endpoints": {
            "1": {"epId": 1, "profId": 260, "devId": 5, "inClusterList": [0, 6], "outClusterList": []},
        },
    }))
}

#[test]
fn empty_registry_returns_not_found() {
    let registry = DefinitionRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.external_count(), 0);
    assert!(registry.find_by_device(&zh_device(), true).is_none());
}

#[test]
fn coordinator_never_has_a_definition() {
    let mut registry = DefinitionRegistry::new();
    registry
        .register(Definition::new("Coordinator", "Test", "Coordinator").with_zigbee_model("TEST"));
    let coordinator = ZhDevice::from_database_entry(&json!({
        "id": 1,
        "type": "Coordinator",
        "ieeeAddr": "0x00124b0024c16b95",
        "nwkAddr": 0,
        "modelId": "TEST",
    }));

    assert!(registry.find_by_device(&coordinator, true).is_none());
}

#[test]
fn finds_single_definition_by_zigbee_model() {
    let mut registry = DefinitionRegistry::new();
    registry.register(Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST"));

    let definition = registry.find_by_device(&zh_device(), true).expect("found");
    assert_eq!(definition.model, "TEST");
    // Model IDs are normalized like upstream `normalizeModelID`.
    let mut upper = DefinitionRegistry::new();
    upper.register(Definition::new("UPPER", "Acme", "Upper").with_zigbee_model(" test "));
    assert!(upper.find_by_device(&zh_device(), true).is_some());
}

#[test]
fn fingerprint_priority_wins_between_candidates() {
    let mut registry = DefinitionRegistry::new();
    let mut low = Definition::new("LOW", "Acme", "Low").with_zigbee_model("TEST");
    low = low.with_fingerprint(Fingerprint {
        manufacturer_name: Some("Test".to_string()),
        priority: 1,
        ..Default::default()
    });
    let mut high = Definition::new("HIGH", "Acme", "High").with_zigbee_model("TEST");
    high = high.with_fingerprint(Fingerprint {
        manufacturer_name: Some("Test".to_string()),
        priority: 5,
        ..Default::default()
    });
    registry.register(low);
    registry.register(high);

    assert_eq!(
        registry
            .find_by_device(&zh_device(), true)
            .map(|definition| definition.model.clone()),
        Some("HIGH".to_string())
    );
}

#[test]
fn white_label_substitutes_model_and_vendor() {
    let mut registry = DefinitionRegistry::new();
    let definition = Definition::new("TEST", "Acme", "Test device")
        .with_zigbee_model("TEST")
        .with_white_label(WhiteLabel {
            model: "TEST-OEM".to_string(),
            vendor: Some("OEM".to_string()),
            description: Some("OEM device".to_string()),
            fingerprint: vec![Fingerprint {
                manufacturer_name: Some("Test".to_string()),
                ..Default::default()
            }],
        });
    registry.register(definition);

    let definition = registry.find_by_device(&zh_device(), true).expect("found");
    assert_eq!(definition.model, "TEST-OEM");
    assert_eq!(definition.vendor, "OEM");
    assert_eq!(definition.description, "OEM device");
}

#[test]
fn external_definitions_are_counted() {
    let mut registry = DefinitionRegistry::new();
    registry.register_external(Definition::new("EXT", "Acme", "External").with_zigbee_model("EXT"));

    assert_eq!(registry.external_count(), 1);
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.all()[0].model, "EXT");
}

#[test]
fn device_definition_resolution_and_exposes() {
    let mut registry = DefinitionRegistry::new();
    registry.register(
        Definition::new("TEST", "Acme", "Test device")
            .with_zigbee_model("TEST")
            .with_exposes(vec![z2m_rs::converters::Expose::binary(
                "state",
                z2m_rs::converters::access::ALL,
            )]),
    );
    let settings = Arc::new(Mutex::new(Settings::from_config(
        json!({"mqtt": {"server": "localhost"}}),
    )));
    let device = Device::new(zh_device(), settings, Arc::new(Mutex::new(registry)));

    device.resolve_definition(false);

    assert!(device.is_supported());
    let definition = device.definition().expect("definition");
    assert_eq!(definition.model, "TEST");
    let exposes = device.exposes().expect("exposes");
    assert_eq!(exposes.len(), 2);
    assert_eq!(exposes[0].r#type, "binary");
    assert_eq!(exposes[1].name.as_deref(), Some("linkquality"));
    assert_eq!(exposes[1].value_min, Some(0.0));
    assert_eq!(exposes[1].value_max, Some(255.0));
}

#[test]
fn definition_less_device_reports_not_supported() {
    let settings = Arc::new(Mutex::new(Settings::from_config(
        json!({"mqtt": {"server": "localhost"}}),
    )));
    let device = Device::without_converters(zh_device(), settings);

    device.resolve_definition(false);

    assert!(!device.is_supported());
    assert!(device.definition().is_none());
    assert_eq!(
        device.exposes().unwrap_err(),
        "Cannot retreive exposes before definition is resolved"
    );
    assert_eq!(device.ota_extra_metas(), json!({}));
}
