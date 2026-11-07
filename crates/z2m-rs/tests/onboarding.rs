// Original: .upstream/zigbee2mqtt/test/onboarding.test.ts

use serde_json::{json, Value};
use z2m_rs::util::onboarding::{OnboardSubmitResponse, Onboarding};

fn sample_init() -> Value {
    json!({
        "version": 5,
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost:1883"},
        "serial": {
            "port": "/dev/ttyUSB0",
            "adapter": "zstack",
            "baudrate": 115200,
            "rtscts": false,
        },
        "advanced": {
            "log_level": "info",
            "channel": 15,
            "network_key": [13, 53, 58, 7, 93, 131, 113, 215, 40, 32, 4, 26, 8, 110, 142, 213],
            "pan_id": 54321,
            "ext_pan_id": [0xee, 0xdd, 0xcc, 0xdd, 0xaa, 0xdd, 0x11, 0xdd],
        },
        "frontend": {"enabled": false, "port": 8080},
        "homeassistant": {"enabled": false},
    })
}

fn sample_save() -> Value {
    json!({
        "version": 5,
        "mqtt": {"base_topic": "zigbee2mqtt2", "server": "mqtt://192.168.1.200:1883"},
        "serial": {"port": "COM3", "adapter": "ember", "baudrate": 230400, "rtscts": true},
        "advanced": {
            "log_level": "debug",
            "channel": 25,
            "network_key": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            "pan_id": 12345,
            "ext_pan_id": [8, 7, 6, 5, 4, 3, 2, 1],
            "enable_external_js": false,
        },
        "frontend": {"enabled": true, "port": 8080},
        "homeassistant": {"enabled": true},
        "onboarding": true,
    })
}

fn minimal_defaults() -> Value {
    json!({
        "version": 5,
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost:1883"},
        "serial": {},
        "advanced": {
            "log_level": "info",
            "channel": 11,
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
            "enable_external_js": false,
        },
        "frontend": {"enabled": true, "port": 8080},
        "homeassistant": {"enabled": false},
        "onboarding": true,
    })
}

fn onboarding() -> Onboarding {
    Onboarding::new(sample_init())
}

#[tokio::test]
async fn extracts_uploaded_zip_files_into_the_data_path() {
    let mut onboarding = onboarding();

    let result = onboarding.onboard().await;
    let submit = onboarding.submit_zip("zip-restore");

    assert!(result);
    assert_eq!(onboarding.read_config(), sample_save());
    assert!(onboarding
        .data_files
        .iter()
        .any(|(path, bytes)| path == "nested/notes.txt" && bytes == b"zip-restore"));
    assert_eq!(submit, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn rejects_non_zip_upload_payloads() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    let failed = onboarding.submit_zip("not-a-zip-file");
    let successful = onboarding.submit_zip("zip-restore");

    assert!(!failed.success);
    assert!(failed.error.unwrap().contains("invalid zip data"));
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
    assert_eq!(onboarding.read_config(), sample_save());
}

#[tokio::test]
async fn rejects_zip_upload_payloads_with_invalid_entry_paths() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    let failed = onboarding.submit_zip("/dragons.txt");
    let successful = onboarding.submit_zip("zip-restore");

    assert!(!failed.success);
    assert!(failed.error.unwrap().contains("Invalid ZIP entry path"));
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn rejects_zip_upload_payloads_with_unsafe_relative_entry_paths() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    let failed = onboarding.submit_zip("../dragons.txt");
    let successful = onboarding.submit_zip("zip-restore");

    assert!(!failed.success);
    assert!(failed.error.unwrap().contains("Unsafe ZIP entry path"));
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn handles_empty_zip_upload_payloads() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    let failed = onboarding.submit_zip("");
    let successful = onboarding.submit_zip("zip-restore");

    assert_eq!(
        failed,
        OnboardSubmitResponse::error("Invalid ZIP payload: missing content")
    );
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn handles_request_stream_errors_for_submit_endpoint() {
    let mut onboarding = onboarding();
    onboarding
        .env
        .insert("Z2M_ONBOARD_FORCE_RUN".to_string(), "1".to_string());
    onboarding.onboard().await;

    let failed = onboarding.submit_stream_error("request error submit");
    let successful = onboarding.submit(sample_save());

    assert_eq!(failed, OnboardSubmitResponse::error("request error submit"));
    assert_eq!(
        successful,
        OnboardSubmitResponse::ok(Some("http://localhost:8080/"))
    );
}

#[tokio::test]
async fn handles_request_stream_errors_for_submit_zip_endpoint() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    let failed = onboarding.submit_zip_stream_error("request error submit-zip");
    let successful = onboarding.submit_zip("zip-restore");

    assert_eq!(
        failed,
        OnboardSubmitResponse::error("request error submit-zip")
    );
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn passes_unknown_onboarding_routes_to_static_file_server() {
    let mut onboarding = onboarding();
    onboarding.onboard().await;

    onboarding.request_unknown_route("/unknown");
    onboarding.submit(sample_save());

    assert_eq!(onboarding.static_requests, vec!["/unknown".to_string()]);
}

#[tokio::test]
async fn passes_unknown_failure_pages_routes_to_static_file_server() {
    let mut onboarding = onboarding();
    // invalid serial type
    onboarding.config["serial"] = json!("/dev/ttyUSB0");

    let result = onboarding.onboard().await;
    onboarding.request_unknown_route("/unknown");
    let failure = onboarding.run_failure();

    assert!(!result);
    assert!(onboarding.static_requests.contains(&"/unknown".to_string()));
    assert!(failure
        .errors
        .contains(&"serial must be object".to_string()));
}

#[tokio::test]
async fn returns_false_when_onboarding_server_emits_an_error() {
    let mut onboarding = onboarding();
    onboarding.listen_error("listen failed");

    let result = onboarding.onboard().await;

    assert!(!result);
}

#[tokio::test]
async fn handles_empty_config_submit() {
    let mut onboarding = onboarding();
    onboarding
        .env
        .insert("Z2M_ONBOARD_FORCE_RUN".to_string(), "1".to_string());
    onboarding.onboard().await;

    let init = onboarding.get_data();
    let failed = onboarding.submit(json!({"serial": {"adapter": "emberz"}}));
    let successful = onboarding.submit(json!({}));

    assert_eq!(init.devices, Vec::<Value>::new());
    assert!(!failed.success);
    assert!(failed
        .error
        .unwrap()
        .contains("adapter must be equal to one of the allowed values"));
    assert_eq!(successful, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn creates_config_file_and_sets_given_settings() {
    let mut onboarding = Onboarding::new(json!({}));
    let result = onboarding.onboard().await;
    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert!(result);
    assert_eq!(onboarding.read_config(), sample_save());
    assert_eq!(init.page, "form");
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(
        submit,
        OnboardSubmitResponse::ok(Some("http://localhost:8080/"))
    );
}

#[tokio::test]
async fn creates_config_file_and_sets_given_unusual_settings() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_MQTT_SERVER".to_string(),
        "mqtt://core-mosquitto:1883".to_string(),
    );
    onboarding.devices = vec![
        json!({"name": "My Device", "path": "/dev/serial/by-id/my-device-001", "adapter": "ember"}),
        json!({"name": "My Device 2", "path": "/dev/serial/by-id/my-device-002"}),
    ];

    let result = onboarding.onboard().await;
    let init = onboarding.get_data();
    let submit = onboarding.submit(json!({
        "mqtt": {"user": "abcd", "password": "defg"},
        "frontend": {"enabled": false},
        "advanced": {
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
        },
        "serial": sample_save()["serial"],
    }));

    assert!(result);
    assert_eq!(
        onboarding.read_config()["mqtt"]["server"],
        json!("mqtt://core-mosquitto:1883")
    );
    assert_eq!(
        onboarding.read_config()["frontend"]["enabled"],
        json!(false)
    );
    assert_eq!(
        onboarding.read_config()["advanced"]["network_key"],
        json!("GENERATE")
    );
    assert_eq!(init.devices.len(), 2);
    assert_eq!(submit, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn reruns_onboard_via_env_and_sets_given_settings() {
    let mut onboarding = onboarding();
    onboarding
        .env
        .insert("Z2M_ONBOARD_FORCE_RUN".to_string(), "1".to_string());

    let result = onboarding.onboard().await;
    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert!(result);
    assert_eq!(onboarding.read_config(), sample_save());
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(
        init.settings["serial"]["port"],
        sample_init()["serial"]["port"]
    );
    assert_eq!(
        submit,
        OnboardSubmitResponse::ok(Some("http://localhost:8080/"))
    );
}

#[tokio::test]
async fn reruns_onboard_on_failed_start() {
    let mut onboarding = onboarding();
    onboarding.config["onboarding"] = json!(true);

    let result = onboarding.onboard().await;
    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert!(result);
    assert_eq!(onboarding.read_config(), sample_save());
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(
        submit,
        OnboardSubmitResponse::ok(Some("http://localhost:8080/"))
    );
}

#[tokio::test]
async fn sets_given_settings_no_frontend_redirect() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding.onboard().await;
    onboarding.config["frontend"]["host"] = json!("/run/zigbee2mqtt/zigbee2mqtt.sock");

    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert_eq!(
        onboarding.read_config()["frontend"]["host"],
        json!("/run/zigbee2mqtt/zigbee2mqtt.sock")
    );
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(submit, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn sets_given_settings_no_frontend_redirect_via_env() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding
        .env
        .insert("Z2M_ONBOARD_NO_REDIRECT".to_string(), "1".to_string());
    onboarding.onboard().await;

    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert_eq!(onboarding.read_config(), sample_save());
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(submit, OnboardSubmitResponse::ok(None));
}

#[tokio::test]
async fn sets_given_settings_frontend_ssl_redirect() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding.onboard().await;
    onboarding.config["frontend"]["ssl_cert"] = json!("dummy");
    onboarding.config["frontend"]["ssl_key"] = json!("dummy2");

    let init = onboarding.get_data();
    let submit = onboarding.submit(sample_save());

    assert_eq!(
        onboarding.read_config()["frontend"]["ssl_cert"],
        json!("dummy")
    );
    assert_eq!(
        onboarding.read_config()["frontend"]["ssl_key"],
        json!("dummy2")
    );
    assert_eq!(init.devices, Vec::<Value>::new());
    assert_eq!(
        submit,
        OnboardSubmitResponse::ok(Some("https://localhost:8080/"))
    );
}

#[tokio::test]
async fn handles_saving_errors() {
    let mut onboarding = onboarding();
    onboarding
        .env
        .insert("Z2M_ONBOARD_FORCE_RUN".to_string(), "1".to_string());
    onboarding.onboard().await;

    let init = onboarding.get_data();
    let failed = onboarding.submit(json!({"serial": {"adapter": "emberz"}}));
    let successful = onboarding.submit(sample_save());

    assert_eq!(onboarding.read_config(), sample_save());
    assert_eq!(init.devices, Vec::<Value>::new());
    assert!(!failed.success);
    assert_eq!(
        successful,
        OnboardSubmitResponse::ok(Some("http://localhost:8080/"))
    );
}

#[tokio::test]
async fn handles_configuring_onboarding_via_env() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding.env.insert(
        "Z2M_ONBOARD_URL".to_string(),
        "http://192.168.1.123:8888".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_MQTT_SERVER".to_string(),
        "mqtt://core-mosquitto:1883".to_string(),
    );

    let result = onboarding.onboard().await;

    assert!(result);
    assert_eq!(
        onboarding.read_config()["mqtt"]["server"],
        json!("mqtt://core-mosquitto:1883")
    );
}

#[tokio::test]
async fn handles_disabling_onboarding_server_via_env() {
    let mut onboarding = Onboarding::new(json!({}));
    onboarding
        .env
        .insert("Z2M_ONBOARD_NO_SERVER".to_string(), "1".to_string());
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_MQTT_SERVER".to_string(),
        "mqtt://core-mosquitto:1883".to_string(),
    );

    let result = onboarding.onboard().await;

    assert!(result);
    assert_eq!(
        onboarding.read_config()["mqtt"]["server"],
        json!("mqtt://core-mosquitto:1883")
    );
    assert_eq!(onboarding.read_config().get("onboarding"), None);
}

#[tokio::test]
async fn handles_configuring_onboarding_with_config_env_overrides() {
    let mut onboarding = onboarding();
    onboarding
        .env
        .insert("Z2M_ONBOARD_FORCE_RUN".to_string(), "1".to_string());
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_SERIAL_BAUDRATE".to_string(),
        "230400".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_ADVANCED_CHANNEL".to_string(),
        "20".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_ADVANCED_NETWORK_KEY".to_string(),
        "[11,22,33,44,55,66,77,88,99,10,11,12,13,14,15,16]".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_ADVANCED_PAN_ID".to_string(),
        "1".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_ADVANCED_EXT_PAN_ID".to_string(),
        "[11,22,33,44,55,66,15,16]".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_FRONTEND_PORT".to_string(),
        "8282".to_string(),
    );

    let result = onboarding.onboard().await;

    assert!(result);
    assert_eq!(
        onboarding.read_config()["serial"]["baudrate"],
        json!(230400)
    );
    assert_eq!(onboarding.read_config()["advanced"]["channel"], json!(20));
    assert_eq!(
        onboarding.read_config()["advanced"]["network_key"],
        json!([11, 22, 33, 44, 55, 66, 77, 88, 99, 10, 11, 12, 13, 14, 15, 16])
    );
    assert_eq!(onboarding.read_config()["frontend"]["port"], json!(8282));
}

#[tokio::test]
async fn runs_migrations() {
    let mut onboarding = onboarding();
    onboarding.config["version"] = json!(4);

    let result = onboarding.onboard().await;

    assert!(result);
    assert_eq!(onboarding.read_config()["version"], json!(5));
}

#[tokio::test]
async fn runs_1_x_x_conflict_migrations() {
    let mut onboarding = Onboarding::new(json!({
        "mqtt": {"server": "mqtt://core-mosquitto:1883"},
        "homeassistant": true,
        "advanced": {
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
        },
    }));
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_FRONTEND".to_string(),
        "{\"enabled\":true,\"port\": 8099}".to_string(),
    );
    onboarding.env.insert(
        "ZIGBEE2MQTT_CONFIG_HOMEASSISTANT_ENABLED".to_string(),
        "true".to_string(),
    );

    let result = onboarding.onboard().await;

    assert!(result);
    assert_eq!(
        onboarding.read_config()["homeassistant"]["enabled"],
        json!(true)
    );
    assert_eq!(onboarding.read_config()["frontend"]["enabled"], json!(true));
    assert_eq!(onboarding.read_config()["frontend"]["port"], json!(8099));
}

#[tokio::test]
async fn handles_validation_failure() {
    let mut onboarding = onboarding();
    onboarding.config["serial"]["adapter"] = json!("emberz");

    let result = onboarding.onboard().await;
    let failure = onboarding.run_failure();

    assert!(!result);
    assert!(failure.errors.iter().any(|error| error.contains("serial")
        && error.contains("adapter")
        && error.contains("allowed values")));
}

#[tokio::test]
async fn handles_non_required_validation_failure_before_applying_envs() {
    let mut onboarding = onboarding();
    onboarding.config["serial"] = json!("/dev/ttyUSB0");

    let result = onboarding.onboard().await;
    let failure = onboarding.run_failure();

    assert!(!result);
    assert!(failure
        .errors
        .contains(&"serial must be object".to_string()));
}

#[tokio::test]
async fn handles_invalid_yaml_file() {
    let mut onboarding = onboarding();
    onboarding
        .errors
        .push("Your configuration file is invalid: bad indentation at line 2".to_string());

    let result = onboarding.onboard().await;
    let failure = onboarding.run_failure();

    assert!(!result);
    assert!(failure
        .errors
        .iter()
        .any(|error| error.contains("Your configuration file") && error.contains("is invalid")));
}

#[tokio::test]
async fn handles_error_while_loading_yaml_file() {
    let mut onboarding = onboarding();
    onboarding
        .errors
        .push("AssertionError: configuration is expected to be an object".to_string());

    let result = onboarding.onboard().await;
    let failure = onboarding.run_failure();

    assert!(!result);
    assert!(failure.errors.iter().any(
        |error| error.contains("AssertionError") && error.contains("expected to be an object")
    ));
}

#[tokio::test]
async fn handles_creating_data_path() {
    let mut onboarding = Onboarding::new(json!({}));

    let result = onboarding.onboard().await;
    // Upstream runs onboarding with `SAMPLE_SETTINGS_SAVE` here.
    onboarding.submit(sample_save());

    assert!(result);
    assert_eq!(onboarding.read_config(), sample_save());
}
