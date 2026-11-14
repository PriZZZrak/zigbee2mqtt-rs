// Original: .upstream/zigbee2mqtt/lib/controller.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::json;

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

#[tokio::test]
async fn constructs_full_extension_set_and_rejects_builtin_runtime_changes() {
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "homeassistant": {"enabled": true},
        "frontend": {"enabled": false},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state_path = std::env::temp_dir().join(format!(
        "z2m-ctrl-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let controller = Arc::new(Controller::new(
        event_bus,
        zigbee,
        state,
        mqtt,
        settings,
        logger,
    ));

    for name in [
        "OnEvent",
        "Bridge",
        "Publish",
        "Receive",
        "Configure",
        "NetworkMap",
        "Groups",
        "Bind",
        "Availability",
        "Health",
        "HomeAssistant",
        "Frontend",
    ] {
        let extension = controller
            .make_extension(name)
            .unwrap_or_else(|| panic!("missing extension {name}"));
        assert_eq!(extension.name(), name);
    }
    assert!(controller.make_extension("NotAnExtension").is_none());

    let error = controller
        .enable_disable_extension(false, "Bridge".to_string())
        .await
        .unwrap_err();
    assert_eq!(error, "Built-in extension Bridge cannot be disabled at runtime");
    let error = controller
        .enable_disable_extension(true, "OnEvent".to_string())
        .await
        .unwrap_err();
    assert!(error.contains("cannot be enabled at runtime"));

    let error = controller
        .enable_disable_extension(true, "Frontend".to_string())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        "Tried to enable Frontend extension disabled in settings"
    );

    let _ = std::fs::remove_file(state_path);
}
