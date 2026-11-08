// Port of .upstream/zigbee2mqtt/test/objectAssignDeep.test.ts
//
// Function under test: z2m_rs::util::object_assign_deep::object_assign_deep

use serde_json::json;
use z2m_rs::util::object_assign_deep::object_assign_deep;

fn parse(json_text: &str) -> serde_json::Value {
    serde_json::from_str(json_text).unwrap()
}

#[test]
fn mutates_and_returns_the_target() {
    let mut target = json!({"a": 1});
    let target_ptr = &target as *const serde_json::Value;
    let result = object_assign_deep(&mut target, &[json!({"b": 2})]);

    assert!(std::ptr::eq(result as *const serde_json::Value, target_ptr));
    assert_eq!(result, &json!({"a": 1, "b": 2}));
}

#[test]
fn applies_sources_in_order_later_ones_win() {
    let mut target = json!({});
    let result = object_assign_deep(
        &mut target,
        &[json!({"a": 1, "b": 1}), json!({"b": 2, "c": 2})],
    );
    assert_eq!(result, &json!({"a": 1, "b": 2, "c": 2}));
}

#[test]
fn copies_keys_missing_from_the_target() {
    let mut target = json!({});
    let source = json!({"nested": {"deep": {"value": 1}}});
    let result = object_assign_deep(&mut target, &[source]);
    assert_eq!(result, &json!({"nested": {"deep": {"value": 1}}}));
}

#[test]
fn deep_merges_nested_objects_present_in_both() {
    let mut target = json!({"mqtt": {"base_topic": "zigbee2mqtt", "server": "old"}, "advanced": {"channel": 11}});
    let result = object_assign_deep(&mut target, &[json!({"mqtt": {"server": "new"}})]);

    assert_eq!(
        result,
        &json!({"mqtt": {"base_topic": "zigbee2mqtt", "server": "new"}, "advanced": {"channel": 11}})
    );
}

#[test]
fn replaces_nested_objects_of_the_target_instead_of_mutating_them() {
    let nested = json!({"a": 1});
    let mut target = json!({"nested": nested.clone()});

    object_assign_deep(&mut target, &[json!({"nested": {"b": 2}})]);

    assert_eq!(nested, json!({"a": 1}));
    let result_ptr = &target["nested"] as *const serde_json::Value;
    assert!(!std::ptr::eq(
        result_ptr,
        &nested as *const serde_json::Value
    ));
    assert_eq!(target, json!({"nested": {"a": 1, "b": 2}}));
}

#[test]
fn replaces_an_existing_non_object_value_with_a_clone_of_the_source_object() {
    assert_eq!(
        object_assign_deep(&mut json!({"a": 5}), &[json!({"a": {"b": 1}})]),
        &json!({"a": {"b": 1}})
    );
    assert_eq!(
        object_assign_deep(&mut json!({"a": "str"}), &[json!({"a": {"b": 1}})]),
        &json!({"a": {"b": 1}})
    );
    assert_eq!(
        object_assign_deep(&mut json!({"a": [1, 2]}), &[json!({"a": {"b": 1}})]),
        &json!({"a": {"b": 1}})
    );
    // `null` is not an object, so it takes the "existing value" path but is not merged into.
    assert_eq!(
        object_assign_deep(&mut json!({"a": null}), &[json!({"a": {"b": 1}})]),
        &json!({"a": {"b": 1}})
    );
}

#[test]
fn overwrites_with_null() {
    // Upstream also covers `undefined`, which has no JSON representation in Rust.
    let mut target = json!({"a": {"b": 1}, "c": 1});
    let result = object_assign_deep(&mut target, &[json!({"a": null, "c": null})]);
    assert_eq!(result, &json!({"a": null, "c": null}));
}

#[test]
fn replaces_arrays_instead_of_concatenating_them() {
    assert_eq!(
        object_assign_deep(&mut json!({"a": [1, 2, 3]}), &[json!({"a": [4]})]),
        &json!({"a": [4]})
    );
    assert_eq!(
        object_assign_deep(&mut json!({"a": [1, 2, 3]}), &[json!({"a": []})]),
        &json!({"a": []})
    );
    assert_eq!(
        object_assign_deep(&mut json!({"a": 1}), &[json!({"a": [4]})]),
        &json!({"a": [4]})
    );
    assert_eq!(
        object_assign_deep(&mut json!({}), &[json!({"a": [4]})]),
        &json!({"a": [4]})
    );
}

#[test]
fn clones_arrays_and_the_objects_nested_inside_them() {
    let source = json!({"a": [{"b": 1}, [{"c": 2}]]});
    let mut target = json!({});
    let result = object_assign_deep(&mut target, &[source.clone()]);

    assert_eq!(result, &source);
    assert!(!std::ptr::eq(
        &result["a"] as *const _,
        &source["a"] as *const _
    ));
    assert!(!std::ptr::eq(
        &result["a"][0] as *const _,
        &source["a"][0] as *const _
    ));
    assert!(!std::ptr::eq(
        &result["a"][1][0] as *const _,
        &source["a"][1][0] as *const _
    ));
}

#[test]
fn breaks_all_references_to_the_sources() {
    let mut source = json!({"a": {"b": {"c": 1}}});
    let mut target = json!({});
    let result = object_assign_deep(&mut target, &[source.clone()]);

    source["a"]["b"]["c"] = json!(99);

    assert_eq!(result["a"]["b"]["c"], json!(1));
}

#[test]
fn does_not_mutate_the_sources() {
    let source = json!({"a": {"b": 1}});

    object_assign_deep(&mut json!({"a": {"c": 2}}), &[source.clone()]);

    assert_eq!(source, json!({"a": {"b": 1}}));
}

#[test]
fn copies_primitives_by_value() {
    // Upstream also covers functions, symbols and BigInt, which have no JSON
    // representation in Rust/serde_json and are therefore not portable.
    let mut target = json!({});
    let result = object_assign_deep(
        &mut target,
        &[json!({"n": 1, "s": "x", "b": true, "z": null})],
    );

    assert_eq!(result["n"], json!(1));
    assert_eq!(result["s"], json!("x"));
    assert_eq!(result["b"], json!(true));
    assert_eq!(result["z"], json!(null));
}

#[test]
fn reduces_non_plain_objects_to_their_own_enumerable_properties() {
    // serde_json only has plain objects/arrays, so non-plain inputs are
    // represented by their own enumerable properties already.
    assert_eq!(
        object_assign_deep(&mut json!({}), &[json!({"date": {}})]),
        &json!({"date": {}})
    );
    assert_eq!(
        object_assign_deep(&mut json!({}), &[json!({"device": {"id": 1}})]),
        &json!({"device": {"id": 1}})
    );
}

#[test]
fn merges_deeply_nested_objects_coming_from_multiple_sources() {
    let mut target = json!({});
    let result = object_assign_deep(
        &mut target,
        &[
            json!({"a": {"b": {"c": 1}}}),
            json!({"a": {"b": {"d": 2}, "e": 3}}),
        ],
    );

    assert_eq!(result, &json!({"a": {"b": {"c": 1, "d": 2}, "e": 3}}));
}

#[test]
fn never_copies_keys_that_could_tamper_with_the_prototype_chain() {
    let mut target = json!({});
    let result = object_assign_deep(
        &mut target,
        &[parse(
            r#"{"__proto__": {"polluted": "yes"}, "constructor": {"x": 1}, "prototype": {"y": 2}, "safe": 1}"#,
        )],
    );

    assert_eq!(result, &json!({"safe": 1}));
    assert!(result.get("__proto__").is_none());
    assert!(result.get("constructor").is_none());
    assert!(result.get("prototype").is_none());
}

#[test]
fn never_copies_unsafe_keys_nested_inside_cloned_objects() {
    let mut target = json!({});
    let result = object_assign_deep(
        &mut target,
        &[
            json!({"nested": parse(r#"{"__proto__": {"polluted": "yes"}, "constructor": 1, "prototype": 2, "safe": 1}"#)}),
        ],
    );

    assert_eq!(result, &json!({"nested": {"safe": 1}}));
    assert!(result["nested"].get("__proto__").is_none());
    assert!(result["nested"].get("constructor").is_none());
    assert!(result["nested"].get("prototype").is_none());
}

#[test]
fn never_copies_unsafe_keys_when_merging_into_an_existing_object() {
    let mut target = json!({"nested": {"safe": 1}});
    let result = object_assign_deep(
        &mut target,
        &[parse(
            r#"{"nested": {"__proto__": {"polluted": "yes"}, "other": 2}}"#,
        )],
    );

    assert_eq!(result, &json!({"nested": {"safe": 1, "other": 2}}));
    assert!(result["nested"].get("__proto__").is_none());
}

#[test]
fn leaves_every_source_untouched_when_given_an_empty_target() {
    let first = json!({"a": {"b": 1}});
    let second = json!({"a": {"c": 2}});
    let mut target = json!({});
    let result = object_assign_deep(&mut target, &[first.clone(), second.clone()]);

    assert_eq!(result, &json!({"a": {"b": 1, "c": 2}}));
    assert!(!std::ptr::eq(
        result as *const serde_json::Value,
        &first as *const serde_json::Value
    ));
    assert!(!std::ptr::eq(
        &result["a"] as *const serde_json::Value,
        &first["a"] as *const serde_json::Value
    ));
    assert_eq!(first, json!({"a": {"b": 1}}));
    assert_eq!(second, json!({"a": {"c": 2}}));
}
