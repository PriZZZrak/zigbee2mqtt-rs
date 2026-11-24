use converter_artifact::{
    ArtifactIndex, Definition, DeviceEndpoint, DeviceProfile, DeviceType, EndpointFingerprint,
    FileRecord, Fingerprint, RegexSpec, WhiteLabel,
};

fn file() -> FileRecord {
    FileRecord {
        path: "devices/test.js".to_owned(),
        bytes: 1,
        sha256: "00".repeat(32),
    }
}
fn definition(
    model: &str,
    fingerprints: Vec<Fingerprint>,
    white_label: Vec<WhiteLabel>,
) -> Definition {
    Definition {
        file: 0,
        index: 0,
        model: model.to_owned(),
        vendor: "Vendor".to_owned(),
        description: format!("{model} description"),
        version: "0.0.0".to_owned(),
        zigbee_model: vec![model.to_owned()],
        fingerprints,
        white_label,
    }
}
fn fingerprint(model: &str, manufacturer: &str, priority: i32) -> Fingerprint {
    Fingerprint {
        model_id: Some(model.to_owned()),
        manufacturer_name: Some(manufacturer.to_owned()),
        priority: Some(priority),
        ..Default::default()
    }
}
fn index(definitions: Vec<Definition>, keys: &[(&str, Vec<u32>)]) -> ArtifactIndex {
    ArtifactIndex {
        files: vec![file()],
        definitions,
        keys: keys
            .iter()
            .map(|(key, ids)| ((*key).to_owned(), ids.clone()))
            .collect(),
    }
}
fn profile(model: &str) -> DeviceProfile {
    DeviceProfile {
        r#type: DeviceType::EndDevice,
        model_id: Some(model.to_owned()),
        ..Default::default()
    }
}

#[test]
fn highest_fingerprint_priority_wins() {
    let index = index(
        vec![
            definition("a", vec![fingerprint("k", "m", 0)], vec![]),
            definition("b", vec![fingerprint("k", "m", 9)], vec![]),
        ],
        &[("k", vec![0, 1])],
    );
    let mut device = profile("k");
    device.manufacturer_name = Some("m".to_owned());
    assert_eq!(index.resolve(&device).unwrap().model, "b");
}

#[test]
fn priority_tie_keeps_first_candidate() {
    let index = index(
        vec![
            definition("a", vec![fingerprint("k", "m", 3)], vec![]),
            definition("b", vec![fingerprint("k", "m", 3)], vec![]),
        ],
        &[("k", vec![0, 1])],
    );
    let mut device = profile("k");
    device.manufacturer_name = Some("m".to_owned());
    assert_eq!(index.resolve(&device).unwrap().model, "a");
}

#[test]
fn empty_fingerprint_strings_are_ignored_like_upstream_truthiness() {
    let fp = Fingerprint {
        model_id: Some(String::new()),
        manufacturer_name: Some(String::new()),
        r#type: Some(String::new()),
        ..Default::default()
    };
    let index = index(
        vec![definition("a", vec![fp], vec![])],
        &[("null", vec![0])],
    );
    let device = DeviceProfile {
        r#type: DeviceType::Router,
        model_id: Some(String::new()),
        manufacturer_name: Some("other".to_owned()),
        ..Default::default()
    };
    assert_eq!(index.resolve(&device).unwrap().model, "a");
}

#[test]
fn zigbee_model_fallback_matches_exact_and_normalized() {
    let index = index(
        vec![definition("Model", vec![], vec![])],
        &[("model", vec![0])],
    );
    assert_eq!(index.resolve(&profile("Model")).unwrap().model, "Model");
    assert_eq!(
        index.resolve(&profile("Model\0junk")).unwrap().model,
        "Model"
    );
    // A different key is not present in this index.
    assert!(index.resolve(&profile("other")).is_none());
}

#[test]
fn white_label_overlays_model_and_vendor() {
    let white = WhiteLabel {
        model: "WhiteModel".to_owned(),
        vendor: Some("WhiteVendor".to_owned()),
        description: Some("White description".to_owned()),
        fingerprints: vec![fingerprint("k", "wm", 0)],
    };
    let index = index(
        vec![definition(
            "base",
            vec![fingerprint("k", "m", 0)],
            vec![white],
        )],
        &[("k", vec![0])],
    );
    let mut device = profile("k");
    device.manufacturer_name = Some("wm".to_owned());
    let resolved = index.resolve(&device).unwrap();
    assert_eq!(resolved.model, "WhiteModel");
    assert_eq!(resolved.vendor, "WhiteVendor");
    assert!(resolved.white_label);
}

#[test]
fn endpoint_fields_and_ieee_regex_are_checked() {
    let mut fp = fingerprint("k", "m", 0);
    fp.ieee_addr = Some(RegexSpec {
        source: "^0x00124b".to_owned(),
        flags: "i".to_owned(),
    });
    fp.endpoints = Some(vec![EndpointFingerprint {
        id: Some(1),
        profile_id: Some(0x0104),
        device_id: Some(0x0100),
        input_clusters: Some(vec![0, 6]),
        output_clusters: Some(vec![25]),
    }]);
    let mut def = definition("a", vec![fp], vec![]);
    def.zigbee_model.clear();
    let index = index(vec![def], &[("k", vec![0])]);
    let mut device = profile("k");
    device.manufacturer_name = Some("m".to_owned());
    device.ieee_addr = Some("0x00124B0024C16B95".to_owned());
    device.endpoints = vec![DeviceEndpoint {
        id: Some(1),
        profile_id: Some(0x0104),
        device_id: Some(0x0100),
        input_clusters: vec![0, 6],
        output_clusters: vec![25],
    }];
    assert!(index.resolve(&device).is_some());
    device.endpoints[0].input_clusters = vec![0];
    assert!(index.resolve(&device).is_none());
    device.endpoints[0].input_clusters = vec![0, 6];
    device.ieee_addr = Some("0xffff".to_owned());
    assert!(index.resolve(&device).is_none());
}

#[test]
fn coordinator_and_missing_key_return_none() {
    let index = index(vec![definition("a", vec![], vec![])], &[("a", vec![0])]);
    let mut coordinator = profile("a");
    coordinator.r#type = DeviceType::Coordinator;
    assert!(index.resolve(&coordinator).is_none());
    assert!(index.resolve(&profile("missing")).is_none());
}
