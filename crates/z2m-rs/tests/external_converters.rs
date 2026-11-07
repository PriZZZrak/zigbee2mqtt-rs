// Original: .upstream/zigbee2mqtt/test/extensions/externalConverters.test.ts

use serde_json::{json, Value};
use z2m_rs::extension::external_converters::{ConverterFile, ExternalConverters};

const CJS_MAIN: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_converters/cjs/mock-external-converter.js"
);
const CJS_MULTIPLE: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_converters/cjs/mock-external-converter-multiple.js"
);
const MJS_MAIN: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_converters/mjs/mock-external-converter.mjs"
);
const MJS_MULTIPLE: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_converters/mjs/mock-external-converter-multiple.mjs"
);

fn cjs_assets() -> Vec<ConverterFile> {
    vec![
        ConverterFile::new("mock-external-converter-multiple.js", CJS_MULTIPLE),
        ConverterFile::new("mock-external-converter.js", CJS_MAIN),
    ]
}

fn mjs_assets() -> Vec<ConverterFile> {
    vec![
        ConverterFile::new("mock-external-converter-multiple.mjs", MJS_MULTIPLE),
        ConverterFile::new("mock-external-converter.mjs", MJS_MAIN),
    ]
}

fn external_definition(description: &str) -> Value {
    json!({
        "description": description,
        "model": "external_converter_device",
        "vendor": "external",
        "zigbeeModel": ["external_converter_device"],
    })
}

fn auto_definition() -> Value {
    json!({
        "description": "Automatically generated definition",
        "model": "external_converter_device",
        "vendor": "",
        "zigbeeModel": ["external_converter_device"],
    })
}

fn published_on<'a>(
    external_converters: &'a ExternalConverters,
    topic: &str,
) -> Vec<&'a (String, Value, Value)> {
    external_converters
        .published
        .iter()
        .filter(|(published_topic, _, _)| published_topic == topic)
        .collect()
}

fn response_payload<'a>(external_converters: &'a ExternalConverters, topic: &str) -> &'a Value {
    &published_on(external_converters, topic)
        .first()
        .unwrap_or_else(|| panic!("missing publish on {topic}"))
        .1
}

#[tokio::test]
async fn loads_nothing() {
    let mut external_converters = ExternalConverters::new();

    external_converters.start().await;

    assert_eq!(external_converters.published_converters(), vec![json!([])]);
    assert_eq!(
        published_on(&external_converters, "zigbee2mqtt/bridge/converters")[0].2,
        json!({"retain": true})
    );
}

#[tokio::test]
async fn cjs_loads_converters() {
    let mut external_converters = ExternalConverters::new();
    external_converters.folder = cjs_assets();

    external_converters.start().await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );
    assert_eq!(
        external_converters.published_converters(),
        vec![json!([
            {"name": "mock-external-converter-multiple.js", "code": CJS_MULTIPLE},
            {"name": "mock-external-converter.js", "code": CJS_MAIN},
        ])]
    );
    assert_eq!(
        external_converters.definitions_removed,
        vec![
            "mock-external-converter-multiple.js".to_string(),
            "mock-external-converter.js".to_string(),
        ]
    );
    assert_eq!(external_converters.definitions_added.len(), 3);
    assert_eq!(external_converters.definitions_added[0]["mock"], json!(1));
    assert_eq!(external_converters.definitions_added[1]["mock"], json!(2));
    assert_eq!(
        external_converters.definitions_added[2]["mock"],
        json!(true)
    );

    let bridge_devices = published_on(&external_converters, "zigbee2mqtt/bridge/devices");
    assert_eq!(bridge_devices.len(), 1);
    assert_eq!(
        bridge_devices[0].1,
        json!([{
            "model_id": "external_converter_device",
            "supported": true,
            "definition": {
                "description": "external/converter",
                "model": "external_converter_device",
            },
        }])
    );
}

#[tokio::test]
async fn mjs_loads_converters() {
    let mut external_converters = ExternalConverters::new();
    external_converters.folder = mjs_assets();

    external_converters.start().await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );
    assert_eq!(
        external_converters.published_converters(),
        vec![json!([
            {"name": "mock-external-converter-multiple.mjs", "code": MJS_MULTIPLE},
            {"name": "mock-external-converter.mjs", "code": MJS_MAIN},
        ])]
    );
    assert_eq!(
        external_converters.definitions_removed,
        vec![
            "mock-external-converter-multiple.mjs".to_string(),
            "mock-external-converter.mjs".to_string(),
        ]
    );
    assert_eq!(external_converters.definitions_added.len(), 3);
}

#[tokio::test]
async fn updates_after_edit_from_mqtt() {
    let converter_name = "mock-external-converter.js";
    let mut external_converters = ExternalConverters::new();
    external_converters.folder = cjs_assets();
    external_converters.start().await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );

    let edited_code = CJS_MAIN.replace(
        "posix.join(\"external\", \"converter\")",
        "posix.join(\"external\", \"converter\", \"edited\")",
    );
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": edited_code}),
        )
        .await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter/edited")
    );
    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([
            {"name": "mock-external-converter-multiple.js", "code": CJS_MULTIPLE},
            {"name": converter_name, "code": edited_code},
        ]))
    );
    assert_eq!(
        external_converters.definitions_added.last().unwrap()["externalConverterName"],
        json!(converter_name)
    );

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": CJS_MAIN}),
        )
        .await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );
    assert_eq!(
        external_converters.definitions_added.last().unwrap()["externalConverterName"],
        json!(converter_name)
    );
}

#[tokio::test]
async fn loads_all_valid_converters_relocates_skips_ones_with_errors() {
    let mut external_converters = ExternalConverters::new();
    external_converters.folder = mjs_assets();
    external_converters
        .folder
        .push(ConverterFile::new("invalid.mjs", "invalid js"));

    external_converters.start().await;

    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([
            {"name": "mock-external-converter-multiple.mjs", "code": MJS_MULTIPLE},
            {"name": "mock-external-converter.mjs", "code": MJS_MAIN},
        ]))
    );
    assert_eq!(
        external_converters.relocated_files,
        vec![("invalid.mjs".to_string(), "invalid.mjs.invalid".to_string())]
    );
    assert!(external_converters
        .parse_errors
        .iter()
        .any(|error| error.contains("Parse failure: Expected ';', '}' or <eof>")));
}

#[tokio::test]
async fn cjs_saves_and_removes() {
    let converter_name = "foo.js";
    let converter_code = CJS_MAIN;
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    assert_eq!(external_converters.device_definition(), auto_definition());

    // -- SAVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": converter_code}),
        )
        .await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );
    assert_eq!(
        external_converters.written_files,
        vec![(
            format!("external_converters/{converter_name}"),
            converter_code.to_string()
        )]
    );
    assert_eq!(
        external_converters.definitions_removed,
        vec![converter_name.to_string()]
    );
    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([{"name": converter_name, "code": converter_code}]))
    );

    // -- REMOVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/remove",
            json!({"name": converter_name}),
        )
        .await;

    assert_eq!(external_converters.device_definition(), auto_definition());
    assert!(external_converters
        .removed_files
        .iter()
        .any(|path| path.contains(converter_name)));
    assert_eq!(
        external_converters.definitions_removed.last(),
        Some(&converter_name.to_string())
    );
    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([]))
    );
}

#[tokio::test]
async fn mjs_saves_and_removes() {
    let converter_name = "foo.mjs";
    let converter_code = MJS_MAIN;
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    assert_eq!(external_converters.device_definition(), auto_definition());

    // -- SAVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": converter_code}),
        )
        .await;

    assert_eq!(
        external_converters.device_definition(),
        external_definition("external/converter")
    );
    assert_eq!(
        external_converters.written_files,
        vec![(
            format!("external_converters/{converter_name}"),
            converter_code.to_string()
        )]
    );
    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([{"name": converter_name, "code": converter_code}]))
    );

    // -- REMOVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/remove",
            json!({"name": converter_name}),
        )
        .await;

    assert_eq!(external_converters.device_definition(), auto_definition());
    assert!(external_converters
        .removed_files
        .iter()
        .any(|path| path.contains(converter_name)));
    assert_eq!(
        external_converters.published_converters().last(),
        Some(&json!([]))
    );
}

#[tokio::test]
async fn returns_error_on_invalid_name() {
    let converter_name = "foo1";
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": "a"}),
        )
        .await;

    assert!(response_payload(
        &external_converters,
        "zigbee2mqtt/bridge/response/converter/save"
    )
    .as_str()
    .unwrap()
    .contains("JavaScript file must have '.mjs', '.js' or '.cjs' extension"));
    assert!(external_converters.written_files.is_empty());
}

#[tokio::test]
async fn returns_error_on_invalid_code() {
    let converter_name = "foo1.js";
    let converter_code = "definetly not a correct javascript code";
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": converter_code}),
        )
        .await;

    assert!(response_payload(
        &external_converters,
        "zigbee2mqtt/bridge/response/converter/save"
    )
    .as_str()
    .unwrap()
    .contains(&format!(
        "\"error\":\"{converter_name} contains invalid code"
    )));
    assert!(external_converters
        .written_files
        .iter()
        .any(|(path, code)| path.contains(converter_name) && code == converter_code));
}

#[tokio::test]
async fn returns_error_on_invalid_removal() {
    let converter_name = "foo2.js";
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/remove",
            json!({"name": converter_name}),
        )
        .await;

    assert!(response_payload(
        &external_converters,
        "zigbee2mqtt/bridge/response/converter/remove"
    )
    .as_str()
    .unwrap()
    .contains("doesn't exists"));
    assert!(!external_converters
        .removed_files
        .iter()
        .any(|path| path.contains(converter_name)));
}

#[tokio::test]
async fn returns_error_on_invalid_definition() {
    let converter_name = "foo3.js";
    let converter_code = CJS_MAIN;
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;
    external_converters.add_definition_error = Some("Invalid definition".to_string());

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": converter_code}),
        )
        .await;

    assert!(response_payload(
        &external_converters,
        "zigbee2mqtt/bridge/response/converter/save"
    )
    .as_str()
    .unwrap()
    .contains("Invalid definition"));
    assert!(external_converters
        .written_files
        .iter()
        .any(|(path, code)| path.contains(converter_name) && code == converter_code));
}

#[tokio::test]
async fn returns_error_on_failed_removal() {
    let converter_name = "foo4.js";
    let converter_code = CJS_MAIN;
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    // -- SAVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": converter_name, "code": converter_code}),
        )
        .await;

    external_converters.remove_definition_error = Some("Failed to remove definition".to_string());

    // -- REMOVE
    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/remove",
            json!({"name": converter_name}),
        )
        .await;

    assert_eq!(
        response_payload(
            &external_converters,
            "zigbee2mqtt/bridge/response/converter/remove"
        ),
        &json!({"data": {}, "status": "error", "error": "Failed to remove definition"})
    );
    assert!(!external_converters
        .removed_files
        .iter()
        .any(|path| path.contains(converter_name)));
}

#[tokio::test]
async fn handles_invalid_payloads() {
    let mut external_converters = ExternalConverters::new();
    external_converters.start().await;

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/save",
            json!({"name": "foo5.js", "transaction": 1}),
        )
        .await;

    assert_eq!(
        response_payload(
            &external_converters,
            "zigbee2mqtt/bridge/response/converter/save"
        ),
        &json!({"data": {}, "status": "error", "error": "Invalid payload", "transaction": 1})
    );

    external_converters
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/converter/remove",
            json!({"namex": "foo5.js", "transaction": 2}),
        )
        .await;

    assert_eq!(
        response_payload(
            &external_converters,
            "zigbee2mqtt/bridge/response/converter/remove"
        ),
        &json!({"data": {}, "status": "error", "error": "Invalid payload", "transaction": 2})
    );
}

#[tokio::test]
async fn doesnt_add_extension_when_external_js_disabled() {
    let mut external_converters = ExternalConverters::new();
    external_converters.enabled = false;

    assert!(!external_converters.extension_enabled());
}
