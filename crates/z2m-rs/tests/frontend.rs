// Original: .upstream/zigbee2mqtt/test/extensions/frontend.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Authentication
//   - Extension: Frontend
//   - Should serve device icons
//   - Start/stop HTTPS invalid : missing config
//   - Start/stop HTTPS invalid : missing file
//   - Start/stop HTTPS valid
//   - Start/stop unix socket
//   - Start/stop with defaults
//   - Start/stop without host
//   - Static server
//   - Websocket interaction
//   - Works with non-default complex base url
//   - disables serving
//   - onRequest/onUpgrade
//   - prevents mismatching setting/extension state

use serde_json::json;
use z2m_rs::extension::frontend::{enable_disable_extension, Frontend, ListenTarget};

fn frontend(settings: serde_json::Value) -> Frontend {
    Frontend::from_settings(&settings)
}

#[test]
#[ignore = "not implemented yet"]
fn authentication() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_serve_device_icons() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_https_invalid_missing_config() {
    let mut frontend = frontend(json!({
        "frontend": {
            "enabled": true,
            "port": 8081,
            "host": "127.0.0.1",
            "ssl_cert": ".upstream/zigbee2mqtt/test/assets/certs/dummy.crt",
        },
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );
    assert_ne!(
        frontend.https_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );

    frontend.stop().await;
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_https_invalid_missing_file() {
    let mut frontend = frontend(json!({
        "frontend": {
            "enabled": true,
            "port": 8081,
            "host": "127.0.0.1",
            "ssl_cert": "filesNotExists.crt",
            "ssl_key": ".upstream/zigbee2mqtt/test/assets/certs/dummy.key",
        },
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );
    assert_ne!(
        frontend.https_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );

    frontend.stop().await;
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_https_valid() {
    let mut frontend = frontend(json!({
        "frontend": {
            "enabled": true,
            "port": 8081,
            "host": "127.0.0.1",
            "ssl_cert": ".upstream/zigbee2mqtt/test/assets/certs/dummy.crt",
            "ssl_key": ".upstream/zigbee2mqtt/test/assets/certs/dummy.key",
        },
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_ne!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );
    assert_eq!(
        frontend.https_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );

    frontend.stop().await;
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_unix_socket() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "/tmp/zigbee2mqtt.sock"},
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(frontend.static_mounts.len(), 2);
    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Unix("/tmp/zigbee2mqtt.sock".to_string()))
    );

    frontend.ws_clients += 1;
    frontend.stop().await;

    assert_eq!(frontend.ws_clients_terminated, 1);
    assert_eq!(frontend.http_close_calls, 1);
    assert_eq!(frontend.ws_close_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_with_defaults() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "127.0.0.1"},
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(frontend.static_mounts.len(), 2);
    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );

    frontend.ws_clients += 1;
    frontend.stop().await;

    assert_eq!(frontend.ws_clients_terminated, 1);
    assert_eq!(frontend.http_close_calls, 1);
    assert_eq!(frontend.ws_close_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_stop_without_host() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081},
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(frontend.static_mounts.len(), 2);
    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: None,
        })
    );

    frontend.ws_clients += 1;
    frontend.stop().await;

    assert_eq!(frontend.ws_clients_terminated, 1);
    assert_eq!(frontend.http_close_calls, 1);
    assert_eq!(frontend.ws_close_calls, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn static_server() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn websocket_interaction() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn works_with_non_default_complex_base_url() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn disables_serving() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "127.0.0.1", "disable_ui_serving": true},
        "homeassistant": {"enabled": true},
    }));
    frontend.start().await;

    assert_eq!(frontend.http_listen, None);
    frontend.ws_clients += 1;
    frontend.stop().await;

    assert_eq!(frontend.ws_clients_terminated, 1);
    assert_eq!(frontend.http_close_calls, 0);
    assert_eq!(frontend.ws_close_calls, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn on_request_on_upgrade() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn prevents_mismatching_setting_extension_state() {
    let mut settings_enabled = false;

    let error = enable_disable_extension(settings_enabled, true).unwrap_err();
    assert_eq!(
        error,
        "Tried to enable Frontend extension disabled in settings"
    );

    settings_enabled = true;

    let error = enable_disable_extension(settings_enabled, false).unwrap_err();
    assert_eq!(
        error,
        "Tried to disable Frontend extension enabled in settings"
    );

    assert!(enable_disable_extension(settings_enabled, true).is_ok());

    settings_enabled = false;
    assert!(enable_disable_extension(settings_enabled, false).is_ok());
}
