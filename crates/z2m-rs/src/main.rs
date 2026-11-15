// Original: .upstream/zigbee2mqtt/index.js (entrypoint)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::onboarding;
use z2m_rs::util::settings::Settings;
use z2m_rs::util::{data, yaml};
use z2m_rs::zigbee::Zigbee;

fn read_configuration() -> Value {
    let config_path = data::join_path("configuration.yaml");
    if !config_path.exists() {
        return json!({});
    }
    match yaml::read(&config_path) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("Failed to read configuration.yaml: {error}");
            std::process::exit(1);
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let data_path = data::get_path();
    let settings = Arc::new(Mutex::new(Settings::from_config(read_configuration())));

    // Upstream `onboard()`: brand-new installs and `Z2M_ONBOARD_FORCE_RUN` start
    // the onboarding server before the bridge comes up.
    let conf_exists = data_path.join("configuration.yaml").exists();
    let needs_server = std::env::var("Z2M_ONBOARD_NO_SERVER").is_err()
        && (std::env::var("Z2M_ONBOARD_FORCE_RUN").is_ok()
            || !conf_exists
            || settings
                .lock()
                .unwrap()
                .get()
                .get("onboarding")
                .and_then(Value::as_bool)
                .unwrap_or(false));
    if needs_server {
        settings.lock().unwrap().set_onboarding(true);
        // Windfront onboarding assets are deferred to OpenWrt packaging; the
        // handlers serve `/data`, `/submit` and `/submit-zip` and 404 the rest.
        let static_dir = data_path.join("onboarding");
        if let Err(error) = onboarding::run_server(
            settings.clone(),
            data_path.clone(),
            static_dir,
        )
        .await
        {
            eprintln!("Failed to start onboarding server: {error}");
            std::process::exit(1);
        }
        *settings.lock().unwrap() = Settings::from_config(read_configuration());
    }

    let errors = settings.lock().unwrap().validate();
    if !errors.is_empty() {
        eprintln!("Refusing to start because configuration is not valid, found the following errors:");
        for error in &errors {
            eprintln!("- {error}");
        }
        let _ = onboarding::run_failure_server(errors, data_path.join("onboarding")).await;
        return;
    }

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
