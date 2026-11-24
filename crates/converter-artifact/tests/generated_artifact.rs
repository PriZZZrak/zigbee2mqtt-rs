use converter_artifact::{DeviceProfile, DeviceType, Manifest, from_binary, from_json};

#[test]
fn generated_artifact_is_consistent_when_available() {
    let Ok(root) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let manifest = Manifest::load(&root.join("manifest.json")).expect("manifest");
    manifest
        .verify_compatibility(1, "quickjs-oxide", "0.5.0")
        .expect("compat");
    manifest.verify_package(&root).expect("package hashes");
    let json =
        from_json(&std::fs::read(root.join("index.json")).expect("json")).expect("json index");
    let binary = from_binary(&std::fs::read(root.join(&manifest.index.file)).expect("binary"))
        .expect("binary index");
    assert_eq!(json.definitions.len(), binary.definitions.len());
    assert_eq!(json.keys, binary.keys);
    if json.definitions != binary.definitions {
        let index = json
            .definitions
            .iter()
            .zip(binary.definitions.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "definition mismatch at {index}: json {:?} binary {:?}",
            json.definitions[index], binary.definitions[index]
        );
    }
    let profile = DeviceProfile {
        r#type: DeviceType::EndDevice,
        model_id: Some("TS0601".to_owned()),
        manufacturer_name: Some("_TZE200_locansqn".to_owned()),
        manufacturer_id: Some(4417),
        power_source: Some("Battery".to_owned()),
        endpoints: vec![converter_artifact::DeviceEndpoint {
            id: Some(1),
            ..Default::default()
        }],
        ..Default::default()
    };
    let resolved = json.resolve(&profile).expect("resolved");
    assert_eq!(resolved.model, "TH01Z");
    assert_eq!(resolved.vendor, "Tuya");
    assert_eq!(resolved.file.path, "devices/nous.js");
    assert!(
        manifest
            .split
            .get("tuya.js")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            == Some("split")
    );
}
