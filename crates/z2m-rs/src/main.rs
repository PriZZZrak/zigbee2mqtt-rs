// Original: .upstream/zigbee2mqtt/index.js (entrypoint)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::Value;

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::util::{data, yaml};
use z2m_rs::zigbee::Zigbee;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config_path = data::join_path("configuration.yaml");
    let configuration: Value = match yaml::read(&config_path) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("Failed to read configuration.yaml: {error}");
            std::process::exit(1);
        }
    };

    let settings = Arc::new(Mutex::new(Settings::from_config(configuration)));
    let applied = settings.lock().unwrap().get();
    let mut logger = Logger::new();
    logger.advanced = applied.clone();
    logger.init();
    let logger = Arc::new(Mutex::new(logger));

    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state = Arc::new(Mutex::new(State::new(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
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

    let weak = Arc::downgrade(&controller);
    controller.set_restart_callback(Arc::new(move || {
        let weak = weak.clone();
        Box::pin(async move {
            if let Some(controller) = weak.upgrade() {
                controller.stop(true, 0).await;
            }
        })
    }));
    controller.set_exit_callback(Arc::new(|code, _restart| {
        Box::pin(async move { std::process::exit(code) })
    }));

    if let Err(error) = controller.start().await {
        controller
            .logger
            .lock()
            .unwrap()
            .error(&error, "z2m");
        controller.stop(false, 1).await;
        return;
    }

    if let Err(error) = tokio::signal::ctrl_c().await {
        controller
            .logger
            .lock()
            .unwrap()
            .error(&format!("Failed to listen for shutdown signal: {error}"), "z2m");
    }
    controller.stop(false, 0).await;
}
