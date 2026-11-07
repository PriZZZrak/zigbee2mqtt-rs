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
use z2m_rs::extension::frontend::{
    enable_disable_extension, Frontend, ListenTarget, DEVICE_ICONS_PATH, FRONTEND_PATH,
};

fn frontend(settings: serde_json::Value) -> Frontend {
    Frontend::from_settings(&settings)
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn authentication() {
    let auth_token = "sample-secure-token";
    let mut frontend = frontend(json!({
        "frontend": {
            "enabled": true,
            "port": 8081,
            "host": "127.0.0.1",
            "auth_token": auth_token,
        },
    }));
    frontend.start().await;

    frontend.on_upgrade("/api", 1);
    assert_eq!(frontend.handle_upgrade_calls.len(), 1);
    assert_eq!(frontend.socket_destroy_calls, 0);
    assert_eq!(
        frontend.socket_closes,
        vec![(4401, "Unauthorized".to_string())]
    );

    frontend.socket_closes.clear();
    frontend.upgrade_emits.clear();

    let url = format!("/api?token={auth_token}");
    frontend.on_upgrade(&url, 3);
    assert_eq!(frontend.handle_upgrade_calls.len(), 2);
    assert_eq!(frontend.socket_destroy_calls, 0);
    assert!(frontend.socket_closes.is_empty());
    assert_eq!(frontend.upgrade_emits, vec![(url, 3)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_serve_device_icons() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "127.0.0.1"},
    }));
    frontend.start().await;

    frontend.on_request("/device_icons/my_device.png");
    assert_eq!(
        frontend
            .static_served
            .iter()
            .filter(|(mount, _)| mount == FRONTEND_PATH)
            .count(),
        0
    );
    assert_eq!(
        frontend.static_served.last(),
        Some(&(DEVICE_ICONS_PATH.to_string(), "/my_device.png".to_string()))
    );
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn static_server() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "127.0.0.1"},
    }));
    frontend.start().await;

    assert_eq!(
        frontend.http_listen,
        Some(ListenTarget::Tcp {
            port: 8081,
            host: Some("127.0.0.1".to_string()),
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn websocket_interaction() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn works_with_non_default_complex_base_url() {
    let base_url = "/z2m-more++/c0mplex.url/";
    let mut frontend = frontend(json!({
        "frontend": {
            "enabled": true,
            "port": 8081,
            "host": "127.0.0.1",
            "base_url": base_url,
        },
    }));
    frontend.start().await;

    assert_eq!(
        frontend.ws_path,
        Some("/z2m-more++/c0mplex.url/api".to_string())
    );

    frontend.on_request("/z2m-more++/c0mplex.url");
    assert_eq!(
        frontend.redirect,
        Some((301, "/z2m-more++/c0mplex.url/".to_string()))
    );
    assert!(!frontend
        .static_served
        .iter()
        .any(|(mount, _)| mount == FRONTEND_PATH));
    assert_eq!(frontend.send_not_found_calls, 0);

    frontend.on_request("/z2m-more++/c0mplex.url/");
    assert_eq!(
        frontend.static_served.last(),
        Some(&(FRONTEND_PATH.to_string(), "/".to_string()))
    );
    assert_eq!(frontend.send_not_found_calls, 0);

    frontend.static_served.clear();
    frontend.on_request("/z2m-more++/c0mplex.url/file.txt");
    assert_eq!(
        frontend.static_served.last(),
        Some(&(FRONTEND_PATH.to_string(), "/file.txt".to_string()))
    );
    assert_eq!(frontend.send_not_found_calls, 0);

    frontend.static_served.clear();
    frontend.on_request("/z/file.txt");
    assert!(frontend.static_served.is_empty());
    assert_eq!(frontend.send_not_found_calls, 1);
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_request_on_upgrade() {
    let mut frontend = frontend(json!({
        "frontend": {"enabled": true, "port": 8081, "host": "127.0.0.1"},
    }));
    frontend.start().await;

    frontend.on_upgrade("http://localhost:8080/api", 3);
    assert_eq!(frontend.handle_upgrade_calls.len(), 1);
    assert_eq!(frontend.socket_destroy_calls, 0);
    assert_eq!(
        frontend.upgrade_emits,
        vec![("http://localhost:8080/api".to_string(), 3)]
    );

    frontend.on_request("/file.txt");
    assert_eq!(
        frontend
            .static_served
            .iter()
            .filter(|(mount, _)| mount == DEVICE_ICONS_PATH)
            .count(),
        0
    );
    assert_eq!(
        frontend
            .static_served
            .iter()
            .filter(|(mount, _)| mount == FRONTEND_PATH)
            .count(),
        1
    );
    assert_eq!(
        frontend.static_served.last(),
        Some(&(FRONTEND_PATH.to_string(), "/file.txt".to_string()))
    );
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

// Upstream: it.each(["/z2m/", "/z2m"])("Works with non-default base url %s", ...)
#[tokio::test]
#[ignore = "not implemented yet"]
async fn works_with_non_default_base_url() {
    for base_url in ["/z2m/", "/z2m"] {
        let mut frontend = frontend(json!({
            "frontend": {
                "enabled": true,
                "port": 8081,
                "host": "127.0.0.1",
                "base_url": base_url,
            },
        }));
        frontend.start().await;

        assert_eq!(frontend.ws_path, Some("/z2m/api".to_string()));

        // the base url without trailing slash points at a directory, redirect
        // so relative asset paths resolve against it
        frontend.on_request("/z2m");
        assert_eq!(frontend.redirect, Some((301, "/z2m/".to_string())));
        assert!(!frontend
            .static_served
            .iter()
            .any(|(mount, _)| mount == FRONTEND_PATH));
        assert_eq!(frontend.send_not_found_calls, 0);

        frontend.on_request("/z2m/");
        assert_eq!(
            frontend.static_served.last(),
            Some(&(FRONTEND_PATH.to_string(), "/".to_string()))
        );
        assert_eq!(frontend.send_not_found_calls, 0);

        frontend.static_served.clear();
        frontend.on_request("/z2m/file.txt");
        assert_eq!(
            frontend.static_served.last(),
            Some(&(FRONTEND_PATH.to_string(), "/file.txt".to_string()))
        );
        assert_eq!(frontend.send_not_found_calls, 0);

        frontend.static_served.clear();
        frontend.on_request("/z/file.txt");
        assert!(frontend.static_served.is_empty());
        assert_eq!(frontend.send_not_found_calls, 1);

        frontend.static_served.clear();
        frontend.on_request("/z2m/device_icons/my-device.png");
        assert_eq!(
            frontend.static_served.last(),
            Some(&(DEVICE_ICONS_PATH.to_string(), "/my-device.png".to_string()))
        );
    }
}
