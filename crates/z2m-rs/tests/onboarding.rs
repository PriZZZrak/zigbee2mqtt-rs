//! Port of upstream `test/onboarding.test.ts`.
//!
//! Anchor: `.upstream/zigbee2mqtt/test/onboarding.test.ts` (zigbee2mqtt 2.14.3,
//! `3c841fdf`).
//!
//! Upstream mocks `node:http`/`node:fs` and drives the express handlers; the
//! Rust port exposes the same handlers as functions (`submit_settings`,
//! `submit_zip`, `handle_request`, `onboard`, `read_http_request`), so the
//! cases are asserted on handler results and on the files they write.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use z2m_rs::util::onboarding;
use z2m_rs::util::settings::Settings;
use z2m_rs::util::static_file_server::{create_static_file_server, HttpRequest};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "z2m-onboarding-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn settings_in(dir: &Path) -> Arc<Mutex<Settings>> {
    Arc::new(Mutex::new(
        Settings::read_data_dir(dir).expect("settings read"),
    ))
}

fn static_assets() -> (PathBuf, z2m_rs::util::static_file_server::StaticFileServer) {
    let dir = temp_dir("assets");
    std::fs::write(dir.join("index.html"), "<html>onboarding</html>").unwrap();
    let server = create_static_file_server(dir.clone(), Arc::new(|_: &str| {}));
    (dir, server)
}

fn get(target: &str) -> HttpRequest {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("accept".to_string(), "text/html".to_string());
    HttpRequest {
        method: "GET".to_string(),
        target: target.to_string(),
        headers,
    }
}

fn post(target: &str) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        target: target.to_string(),
        headers: Default::default(),
    }
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default();
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer.into_inner()
}

fn base64(bytes: &[u8]) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .encode(bytes)
        .into_bytes()
}

// --- cases 1-5: ZIP upload -------------------------------------------------

#[tokio::test]
async fn extracts_uploaded_zip_files_into_the_data_path() {
    let dir = temp_dir("zip");
    let body = base64(&zip_with(&[
        ("configuration.yaml", b"version: 5\nmqtt:\n  base_url: x\n"),
        ("sub/dir/file.txt", b"hello"),
    ]));

    let (status, payload) = onboarding::submit_zip(&dir, &body);
    assert_eq!(status, 200, "{payload}");
    assert_eq!(payload["success"], true);
    assert_eq!(
        std::fs::read_to_string(dir.join("configuration.yaml")).unwrap(),
        "version: 5\nmqtt:\n  base_url: x\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("sub/dir/file.txt")).unwrap(),
        "hello"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn rejects_non_zip_upload_payloads() {
    let dir = temp_dir("notzip");
    let (status, payload) = onboarding::submit_zip(&dir, &base64(b"this is not a zip"));
    assert_eq!(status, 406, "{payload}");
    assert_eq!(payload["success"], false);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn rejects_zip_upload_payloads_with_invalid_entry_paths() {
    let dir = temp_dir("abszip");
    let (status, payload) = onboarding::submit_zip(&dir, &base64(&zip_with(&[("/etc/passwd", b"x")])));
    assert_eq!(status, 406, "{payload}");
    assert_eq!(payload["success"], false);
    assert!(payload["error"].as_str().unwrap().contains("Invalid ZIP entry path"));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn rejects_zip_upload_payloads_with_unsafe_relative_entry_paths() {
    let dir = temp_dir("travzip");
    let (status, payload) = onboarding::submit_zip(&dir, &base64(&zip_with(&[("../evil.txt", b"x")])));
    assert_eq!(status, 406, "{payload}");
    assert_eq!(payload["success"], false);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn handles_empty_zip_upload_payloads() {
    let dir = temp_dir("emptyzip");
    let (status, payload) = onboarding::submit_zip(&dir, b"");
    assert_eq!(status, 406, "{payload}");
    assert!(payload["error"]
        .as_str()
        .unwrap()
        .contains("missing content"));
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 6-7: request stream errors --------------------------------------

#[tokio::test]
async fn handles_request_stream_errors_for_submit_endpoints() {
    // A truncated request (headers announce a body that never arrives).
    let mut stream = std::io::Cursor::new(b"POST /submit HTTP/1.1\r\nContent-Length: 10\r\n\r\nabc".to_vec());
    assert!(onboarding::read_http_request(&mut stream).is_err());

    let mut stream = std::io::Cursor::new(b"POST /submit-zip HTTP/1.1\r\nContent-Length: 10\r\n\r\n".to_vec());
    assert!(onboarding::read_http_request(&mut stream).is_err());
}

// --- cases 8-9: unknown routes go to the static file server ----------------

#[tokio::test]
async fn passes_unknown_onboarding_and_failure_routes_to_the_static_file_server() {
    let dir = temp_dir("routes");
    let settings = settings_in(&dir);
    let (assets, server) = static_assets();

    for target in ["/devices", "/some/spa/route"] {
        let response = onboarding::handle_request(&settings, &dir, &get(target), b"", &server);
        assert_eq!(response.status, 200, "{target}");
        assert_eq!(
            String::from_utf8_lossy(&response.body),
            "<html>onboarding</html>",
            "{target}"
        );
    }

    // The failure page itself is a JSON payload served by the same routing.
    assert_eq!(
        onboarding::failure_payload(&["boom".to_string()]),
        json!({"page": "failure", "errors": ["boom"]})
    );

    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(assets);
}

// --- case 10: onboarding server error --------------------------------------

#[tokio::test]
async fn returns_an_error_when_the_onboarding_server_cannot_listen() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (assets, _) = static_assets();
    let dir = temp_dir("listen");
    let settings = settings_in(&dir);
    std::env::set_var("Z2M_ONBOARD_URL", format!("http://127.0.0.1:{}", addr.port()));

    let error = onboarding::run_server(settings, dir.clone(), assets.clone())
        .await
        .expect_err("port already in use");
    assert!(
        error.contains("Address already in use") || error.contains("in use"),
        "{error}"
    );

    std::env::remove_var("Z2M_ONBOARD_URL");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(assets);
    drop(listener);
}

// --- cases 11-13: config submit --------------------------------------------

#[tokio::test]
async fn handles_empty_config_submit() {
    let dir = temp_dir("emptysubmit");
    let settings = settings_in(&dir);

    let (status, payload) = onboarding::submit_settings(&settings, b"");
    assert_eq!(status, 200, "{payload}");
    assert_eq!(payload["success"], true);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn creates_config_file_and_sets_given_settings() {
    let dir = temp_dir("submit");
    let settings = settings_in(&dir);
    let body = serde_json::to_vec(&json!({
        "mqtt": {"base_topic": "custom", "server": "mqtt://example:1883"},
        "serial": {"port": "/dev/ttyACM0"},
        "advanced": {"log_level": "debug"},
        "frontend": {"enabled": false},
    }))
    .unwrap();

    let (status, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(status, 200, "{payload}");
    assert_eq!(payload["success"], true);

    let written = z2m_rs::util::yaml::read(dir.join("configuration.yaml")).unwrap();
    assert_eq!(written["mqtt"]["base_topic"], "custom");
    assert_eq!(written["mqtt"]["server"], "mqtt://example:1883");
    assert_eq!(written["serial"]["port"], "/dev/ttyACM0");
    assert_eq!(written["advanced"]["log_level"], "debug");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn creates_config_file_and_sets_unusual_settings() {
    let dir = temp_dir("unusual");
    let settings = settings_in(&dir);
    // Nested/unknown-but-valid shapes must be persisted verbatim.
    let body = serde_json::to_vec(&json!({
        "mqtt": {"base_topic": "z2m", "server": "mqtt://localhost"},
        "advanced": {"log_namespaced_levels": {"zh:zstack": "debug"}},
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
    }))
    .unwrap();

    let (status, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(status, 200, "{payload}");
    let written = z2m_rs::util::yaml::read(dir.join("configuration.yaml")).unwrap();
    assert_eq!(written["advanced"]["log_namespaced_levels"]["zh:zstack"], "debug");
    assert_eq!(
        written["devices"]["0x0017880104e45519"]["friendly_name"],
        "lamp"
    );
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 14-15: rerun onboard (ENV / failed start) -----------------------

#[tokio::test]
async fn reruns_onboard_via_env_and_on_failed_start() {
    let dir = temp_dir("rerun");
    std::fs::write(
        dir.join("configuration.yaml"),
        "version: 5\nmqtt:\n  base_topic: zigbee2mqtt\n  server: mqtt://localhost\n",
    )
    .unwrap();
    std::env::set_var("Z2M_ONBOARD_FORCE_RUN", "1");

    let settings = settings_in(&dir);
    let ran = onboarding::onboard(&settings, &dir, false).await.expect("onboard");
    assert!(ran);
    assert!(settings.lock().unwrap().configuration["onboarding"]
        .as_bool()
        .unwrap_or(false));

    // A failed start can force the server even without the ENV.
    std::env::remove_var("Z2M_ONBOARD_FORCE_RUN");
    let settings = settings_in(&dir);
    let ran = onboarding::onboard(&settings, &dir, true).await.expect("onboard");
    assert!(ran);
    assert!(settings.lock().unwrap().configuration["onboarding"]
        .as_bool()
        .unwrap_or(false));
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 16-18: frontend redirect ---------------------------------------

#[tokio::test]
async fn sets_given_settings_without_and_with_frontend_redirect() {
    let dir = temp_dir("redirect");
    let settings = settings_in(&dir);

    // frontend disabled -> no redirect
    let body = serde_json::to_vec(&json!({"frontend": {"enabled": false}})).unwrap();
    let (_, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(payload["frontendUrl"], Value::Null);

    // frontend enabled -> redirect to the configured host/port/base_url
    let body = serde_json::to_vec(&json!({
        "frontend": {"enabled": true, "host": "localhost", "port": 8080, "base_url": "/"},
        "homeassistant": {"enabled": false},
    }))
    .unwrap();
    let (_, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(payload["frontendUrl"], "http://localhost:8080/");

    // SSL certificate + key -> https
    let body = serde_json::to_vec(&json!({
        "frontend": {
            "enabled": true,
            "host": "localhost",
            "port": 8080,
            "base_url": "/",
            "ssl_cert": "/tmp/cert.pem",
            "ssl_key": "/tmp/key.pem",
        },
    }))
    .unwrap();
    let (_, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(payload["frontendUrl"], "https://localhost:8080/");

    // ENV disables the redirect
    std::env::set_var("Z2M_ONBOARD_NO_REDIRECT", "1");
    let (_, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(payload["frontendUrl"], Value::Null);
    std::env::remove_var("Z2M_ONBOARD_NO_REDIRECT");
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 20-22: onboarding via ENV ---------------------------------------

#[tokio::test]
async fn handles_configuring_and_disabling_onboarding_via_env() {
    std::env::set_var("Z2M_ONBOARD_URL", "https://0.0.0.0:8099");
    assert_eq!(onboarding::server_url(), ("0.0.0.0".to_string(), 8099));
    std::env::set_var("Z2M_ONBOARD_URL", "http://127.0.0.1");
    assert_eq!(onboarding::server_url(), ("127.0.0.1".to_string(), 8080));
    std::env::remove_var("Z2M_ONBOARD_URL");

    let dir = temp_dir("env-onboard");
    std::fs::write(dir.join("configuration.yaml"), "version: 5\n").unwrap();
    std::env::set_var("Z2M_ONBOARD_FORCE_RUN", "1");
    std::env::set_var("Z2M_ONBOARD_NO_SERVER", "1");
    std::env::set_var("ZIGBEE2MQTT_CONFIG_MQTT_SERVER", "mqtt://envhost:1883");

    let settings = settings_in(&dir);
    let ran = onboarding::onboard(&settings, &dir, false).await.expect("onboard");
    assert!(ran);
    // The server is disabled, but the config ENV override is visible.
    assert!(!settings.lock().unwrap().get()["onboarding"]
        .as_bool()
        .unwrap_or(false));
    assert_eq!(
        settings.lock().unwrap().get()["mqtt"]["server"],
        "mqtt://envhost:1883"
    );

    std::env::remove_var("Z2M_ONBOARD_FORCE_RUN");
    std::env::remove_var("Z2M_ONBOARD_NO_SERVER");
    std::env::remove_var("ZIGBEE2MQTT_CONFIG_MQTT_SERVER");
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 23-24: migrations ----------------------------------------------

#[tokio::test]
async fn runs_migrations() {
    let dir = temp_dir("migrate");
    // Version 4 configuration with a pre-2.x `advanced` layout.
    std::fs::write(
        dir.join("configuration.yaml"),
        "version: 4\nmqtt:\n  base_topic: zigbee2mqtt\n  server: mqtt://localhost\nadvanced:\n  log_level: info\n  network_key: GENERATE\n  pan_id: GENERATE\n  ext_pan_id: GENERATE\n",
    )
    .unwrap();

    let settings = settings_in(&dir);
    let ran = onboarding::onboard(&settings, &dir, true).await.expect("onboard");
    assert!(ran);
    let written = z2m_rs::util::yaml::read(dir.join("configuration.yaml")).unwrap();
    assert_eq!(
        written["version"],
        z2m_rs::util::settings::CURRENT_VERSION
    );
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 25-26: validation ----------------------------------------------

#[tokio::test]
async fn handles_validation_failure() {
    let dir = temp_dir("invalid");
    let settings = settings_in(&dir);

    // `mqtt.server` must be a string; a number fails the schema.
    let body = serde_json::to_vec(&json!({"advanced": {"log_level": "not-a-level"}})).unwrap();
    let (status, payload) = onboarding::submit_settings(&settings, &body);
    assert_eq!(status, 406, "{payload}");
    assert_eq!(payload["success"], false);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn handles_non_required_validation_failure_before_applying_envs() {
    let dir = temp_dir("nonrequired");
    // A partially valid configuration (no mqtt) still boots R1 (non-required
    // validation only).
    std::fs::write(
        dir.join("configuration.yaml"),
        "version: 5\nmqtt:\n  base_topic: zigbee2mqtt\n  server: mqtt://localhost\nserial:\n  port: /dev/ttyACM0\n",
    )
    .unwrap();
    let settings = settings_in(&dir);
    assert!(onboarding::onboard(&settings, &dir, false).await.is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

// --- cases 27-29: yaml errors and data path -------------------------------

#[tokio::test]
async fn handles_invalid_yaml_file_and_load_errors() {
    let dir = temp_dir("badyaml");
    std::fs::write(dir.join("configuration.yaml"), "mqtt: [unclosed\n").unwrap();
    assert!(Settings::read_data_dir(&dir).is_err(), "invalid yaml is fatal");
    let _ = std::fs::remove_dir_all(dir);

    // Unreadable file (a directory) surfaces a load error too.
    let dir = temp_dir("loaderror");
    std::fs::create_dir_all(dir.join("configuration.yaml")).unwrap();
    assert!(Settings::read_data_dir(&dir).is_err(), "load error is fatal");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn handles_creating_data_path() {
    let base = temp_dir("datapath");
    let dir = base.join("nested/data");

    let settings = Arc::new(Mutex::new(
        Settings::read_data_dir(&dir).expect("missing configuration is allowed"),
    ));
    let ran = onboarding::onboard(&settings, &dir, true).await.expect("onboard");
    assert!(ran);
    assert!(dir.is_dir());
    assert!(dir.join("configuration.yaml").is_file());
    let _ = std::fs::remove_dir_all(base);
}
