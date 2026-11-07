// Original: .upstream/zigbee2mqtt/test/extensions/externalExtensions.test.ts

use serde_json::{json, Value};
use z2m_rs::extension::external_extensions::{ExtensionFile, ExternalExtensions};

const CJS_EXAMPLE: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_extensions/cjs/exampleExtension.js"
);
const CJS_EXAMPLE2: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_extensions/cjs/example2Extension.js"
);
const MJS_EXAMPLE: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_extensions/mjs/exampleExtension.mjs"
);
const MJS_EXAMPLE2: &str = include_str!(
    "../../../.upstream/zigbee2mqtt/test/assets/external_extensions/mjs/example2Extension.mjs"
);

fn cjs_assets() -> Vec<ExtensionFile> {
    vec![
        ExtensionFile::new("example2Extension.js", CJS_EXAMPLE2),
        ExtensionFile::new("exampleExtension.js", CJS_EXAMPLE),
    ]
}

fn mjs_assets() -> Vec<ExtensionFile> {
    vec![
        ExtensionFile::new("example2Extension.mjs", MJS_EXAMPLE2),
        ExtensionFile::new("exampleExtension.mjs", MJS_EXAMPLE),
    ]
}

fn published_on<'a>(
    external_extensions: &'a ExternalExtensions,
    topic: &str,
) -> Vec<&'a (String, Value, Value)> {
    external_extensions
        .published
        .iter()
        .filter(|(published_topic, _, _)| published_topic == topic)
        .collect()
}

fn response_payload<'a>(external_extensions: &'a ExternalExtensions, topic: &str) -> &'a Value {
    &published_on(external_extensions, topic)
        .first()
        .unwrap_or_else(|| panic!("missing publish on {topic}"))
        .1
}

#[tokio::test]
async fn loads_nothing() {
    let mut external_extensions = ExternalExtensions::new();

    external_extensions.start().await;

    assert_eq!(external_extensions.published_extensions(), vec![json!([])]);
    assert_eq!(
        published_on(&external_extensions, "zigbee2mqtt/bridge/extensions")[0].2,
        json!({"retain": true})
    );
}

#[tokio::test]
async fn cjs_loads_extensions() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.folder = cjs_assets();

    external_extensions.start().await;

    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example2/extension",
        json!("call2 from constructor")
    ));
    assert!(external_extensions
        .has_publish("zigbee2mqtt/example2/extension", json!("call2 from start")));
    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from constructor")
    ));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from start"))
    );
    assert_eq!(
        external_extensions.published_extensions(),
        vec![json!([
            {"name": "example2Extension.js", "code": CJS_EXAMPLE2},
            {"name": "exampleExtension.js", "code": CJS_EXAMPLE},
        ])]
    );
}

#[tokio::test]
async fn mjs_loads_extensions() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.folder = mjs_assets();

    external_extensions.start().await;

    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example2/extension",
        json!("call2 from constructor")
    ));
    assert!(external_extensions
        .has_publish("zigbee2mqtt/example2/extension", json!("call2 from start")));
    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from constructor")
    ));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from start"))
    );
    assert_eq!(
        external_extensions.published_extensions(),
        vec![json!([
            {"name": "example2Extension.mjs", "code": MJS_EXAMPLE2},
            {"name": "exampleExtension.mjs", "code": MJS_EXAMPLE},
        ])]
    );
}

#[tokio::test]
async fn starts_extensions_only_once() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.folder = mjs_assets();

    external_extensions.start().await;

    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension/counter", json!("start 0"))
    );
    assert!(
        !external_extensions.has_publish("zigbee2mqtt/example/extension/counter", json!("start 1"))
    );
    assert_eq!(
        external_extensions.started,
        vec![
            "example2Extension.mjs".to_string(),
            "exampleExtension.mjs".to_string(),
        ]
    );
}

#[tokio::test]
async fn loads_all_valid_extensions_relocates_skips_ones_with_errors() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.folder = mjs_assets();
    external_extensions
        .folder
        .push(ExtensionFile::new("invalid.mjs", "invalid js"));

    external_extensions.start().await;

    assert_eq!(
        external_extensions.published_extensions(),
        vec![json!([
            {"name": "example2Extension.mjs", "code": MJS_EXAMPLE2},
            {"name": "exampleExtension.mjs", "code": MJS_EXAMPLE},
        ])]
    );
    assert_eq!(
        external_extensions.relocated_files,
        vec![("invalid.mjs".to_string(), "invalid.mjs.invalid".to_string())]
    );
    assert!(external_extensions
        .parse_errors
        .iter()
        .any(|error| error.contains("Parse failure")));
}

#[tokio::test]
async fn updates_after_edit_from_mqtt() {
    let extension_name = "exampleExtension.js";
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.folder = cjs_assets();
    external_extensions.start().await;

    let edited_code = CJS_EXAMPLE
        .replace("\"call from start\"", "\"call from start - edited\"")
        .replace("\"call from stop\"", "\"call from stop - edited\"");

    external_extensions.published.clear();
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": edited_code}),
        )
        .await;

    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from stop"))
    );
    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from start - edited")
    ));
    assert_eq!(
        external_extensions.published_extensions().last(),
        Some(&json!([
            {"name": "example2Extension.js", "code": CJS_EXAMPLE2},
            {"name": extension_name, "code": edited_code},
        ]))
    );

    external_extensions.published.clear();
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": CJS_EXAMPLE}),
        )
        .await;

    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from stop - edited")
    ));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from start"))
    );
    assert!(!external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from start - edited")
    ));
}

#[tokio::test]
async fn cjs_saves_and_removes() {
    let extension_name = "foo.js";
    let extension_code = CJS_EXAMPLE;
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    // -- SAVE
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": extension_code}),
        )
        .await;

    assert_eq!(
        external_extensions.written_files,
        vec![(
            format!("external_extensions/{extension_name}"),
            extension_code.to_string()
        )]
    );
    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from constructor")
    ));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from start"))
    );
    assert_eq!(
        external_extensions.published_extensions().last(),
        Some(&json!([{"name": extension_name, "code": extension_code}]))
    );

    // -- REMOVE
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/remove",
            json!({"name": extension_name}),
        )
        .await;

    assert!(external_extensions
        .removed_files
        .iter()
        .any(|path| path.contains(extension_name)));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from stop"))
    );
    assert_eq!(
        external_extensions.published_extensions().last(),
        Some(&json!([]))
    );
}

#[tokio::test]
async fn mjs_saves_and_removes() {
    let extension_name = "foo.mjs";
    let extension_code = MJS_EXAMPLE;
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    // -- SAVE
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": extension_code}),
        )
        .await;

    assert_eq!(
        external_extensions.written_files,
        vec![(
            format!("external_extensions/{extension_name}"),
            extension_code.to_string()
        )]
    );
    assert!(external_extensions.has_publish(
        "zigbee2mqtt/example/extension",
        json!("call from constructor")
    ));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from start"))
    );
    assert_eq!(
        external_extensions.published_extensions().last(),
        Some(&json!([{"name": extension_name, "code": extension_code}]))
    );

    // -- REMOVE
    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/remove",
            json!({"name": extension_name}),
        )
        .await;

    assert!(external_extensions
        .removed_files
        .iter()
        .any(|path| path.contains(extension_name)));
    assert!(
        external_extensions.has_publish("zigbee2mqtt/example/extension", json!("call from stop"))
    );
    assert_eq!(
        external_extensions.published_extensions().last(),
        Some(&json!([]))
    );
}

#[tokio::test]
async fn returns_error_on_invalid_name() {
    let extension_name = "foo1";
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": "a"}),
        )
        .await;

    assert!(response_payload(
        &external_extensions,
        "zigbee2mqtt/bridge/response/extension/save"
    )
    .as_str()
    .unwrap()
    .contains("JavaScript file must have '.mjs', '.js' or '.cjs' extension"));
    assert!(external_extensions.written_files.is_empty());
}

#[tokio::test]
async fn returns_error_on_invalid_code() {
    let extension_name = "foo1.js";
    let extension_code = "definetly not a correct javascript code";
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": extension_name, "code": extension_code}),
        )
        .await;

    assert!(response_payload(
        &external_extensions,
        "zigbee2mqtt/bridge/response/extension/save"
    )
    .as_str()
    .unwrap()
    .contains(&format!(
        "\"error\":\"{extension_name} contains invalid code"
    )));
    assert!(external_extensions
        .written_files
        .iter()
        .any(|(path, code)| path.contains(extension_name) && code == extension_code));
}

#[tokio::test]
async fn returns_error_on_invalid_removal() {
    let extension_name = "foo2.js";
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/remove",
            json!({"name": extension_name}),
        )
        .await;

    assert!(response_payload(
        &external_extensions,
        "zigbee2mqtt/bridge/response/extension/remove"
    )
    .as_str()
    .unwrap()
    .contains("doesn't exists"));
    assert!(!external_extensions
        .removed_files
        .iter()
        .any(|path| path.contains(extension_name)));
}

#[tokio::test]
async fn handles_invalid_payloads() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.start().await;

    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/save",
            json!({"name": "foo3.js", "transaction": 1}),
        )
        .await;

    assert_eq!(
        response_payload(
            &external_extensions,
            "zigbee2mqtt/bridge/response/extension/save"
        ),
        &json!({"data": {}, "status": "error", "error": "Invalid payload", "transaction": 1})
    );

    external_extensions
        .on_mqtt_message(
            "zigbee2mqtt/bridge/request/extension/remove",
            json!({"namex": "foo3.js", "transaction": 2}),
        )
        .await;

    assert_eq!(
        response_payload(
            &external_extensions,
            "zigbee2mqtt/bridge/response/extension/remove"
        ),
        &json!({"data": {}, "status": "error", "error": "Invalid payload", "transaction": 2})
    );
}

#[tokio::test]
async fn doesnt_add_extension_when_external_js_disabled() {
    let mut external_extensions = ExternalExtensions::new();
    external_extensions.enabled = false;

    assert!(!external_extensions.extension_enabled());
}
