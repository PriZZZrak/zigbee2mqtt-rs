// Original: .upstream/zigbee-herdsman-converters/test/index.test.ts

use serde_json::json;
use zigbee_apiarist_converters::index::{Definition, Index};

#[test]
#[ignore = "not implemented yet"]
fn zhc() {
    let mut index = Index::new();
    index.record("constructor");
    assert!(index.called("constructor"));
}

#[test]
#[ignore = "not implemented yet"]
fn adds_removes_external_converter() {
    let mut index = Index::new();
    index.add_external_definition(Definition::new("external"));
    assert!(index.called("add_external_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn adds_removes_override_external_converter() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn adds_external_converter_with_same_model_built_in() {
    let mut index = Index::new();
    index.add_external_definition(Definition::new("external"));
    assert!(index.called("add_external_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn allows_definition_with_both_modern_extend_and_exposes_as_function() {
    let mut index = Index::new();
    index.prepare_definition(Definition::new("model"));
    assert!(index.called("prepare_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn cannot_find_definition_without_enough_data_points() {
    let mut index = Index::new();
    let _ = index.find_definition(json!({}));
    assert!(index.called("find_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn check_if_all_definitions_are_imported_in_devices_index_ts() {
    let mut index = Index::new();
    index.check_imports();
    assert!(index.called("check_imports"));
}

#[test]
#[ignore = "not implemented yet"]
fn checks_if_white_label_of_references_point_to_existing_models() {
    let mut index = Index::new();
    index.check_white_labels();
    assert!(index.called("check_white_labels"));
}

#[test]
#[ignore = "not implemented yet"]
fn computes_calibration_precision() {
    let mut index = Index::new();
    index.compute_precision();
    assert!(index.called("compute_precision"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_throw_when_exposes_function_throws() {
    let mut index = Index::new();
    index.prepare_definition(Definition::new("model"));
    assert!(index.called("prepare_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_light_with_endpoint() {
    let mut index = Index::new();
    let _ = index.find_definition(json!({}));
    assert!(index.called("find_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_nul_padded_model_id() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_nul_padded_model_id_when_an_external_converter_adds_a_second_candidate() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_gp() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_gp_ptm_215_z_friends_of_hue_range() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_index_of_size_10() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_index_of_size_250() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_index_of_size_35() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_for_mismatching_zigbee_model_in_firmware() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_fingerprint_over_zigbee_model() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_model_id() {
    let mut index = Index::new();
    let _ = index.find_by_model("model");
    assert!(index.called("find_by_model"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_by_prioritized_fingerprints() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_definition_with_white_label_by_fingerprint() {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn finds_the_paulmann_291_52_white_label_for_piic5800_firmware() {
    let mut index = Index::new();
    let _ = index.find_definition(json!({}));
    assert!(index.called("find_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn generates_definition_gp_with_no_matching_fingerprint_from_candidates() {
    let mut index = Index::new();
    let _ = index.generate_definition(json!([]));
    assert!(index.called("generate_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn generates_definition_for_unknown() {
    let mut index = Index::new();
    let _ = index.generate_definition(json!([]));
    assert!(index.called("generate_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn instantiates_list_expose_of_composite_type() {
    let mut index = Index::new();
    index.instantiate_list_expose();
    assert!(index.called("instantiate_list_expose"));
}

#[test]
#[ignore = "not implemented yet"]
fn instantiates_list_expose_of_number_type() {
    let mut index = Index::new();
    index.instantiate_list_expose();
    assert!(index.called("instantiate_list_expose"));
}

#[test]
#[ignore = "not implemented yet"]
fn verifies_options_filter() {
    let mut index = Index::new();
    let _ = index.options_filter(json!([]));
    assert!(index.called("options_filter"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_add_to_zigbee_converters_options_multiple_times_if_find_by_device_is_called_multiple_times_for_the_same_device(
) {
    let mut index = Index::new();
    let _ = index.find_by_device(json!({"modelID": "model"}));
    let _ = index.find_by_device(json!({"modelID": "model"}));
    assert!(index.called("find_by_device"));
}
