//! Controller startup/abort/error paths (audit CP-05).
//!
//! Upstream `test/controller.test.ts` "aborts during startup", "Start controller
//! fails ..." and "EventBus should handle sync/async errors". The Rust port
//! cancels a startup by dropping the future and wraps EventBus handlers with
//! `catch_unwind`; `Zigbee::test_start` / `test_stop_count` make the coordinator
//! phase deterministic without hardware.

use std::sync::{Arc, Mutex};

use serde_json::json;

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

fn harness(server: &str) -> (Arc<Controller>, Arc<Zigbee>, Arc<tokio::sync::Mutex<Mqtt>>) {
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"base_topic": "z2m", "server": server},
        "advanced": {"cache_state": false, "cache_state_send_on_startup": false},
        "homeassistant": {"enabled": false},
        "frontend": {"enabled": false},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state_path = std::env::temp_dir().join(format!(
        "z2m-startup-{}-{}.json",
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
        state_path,
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let controller = Arc::new(Controller::new(
        event_bus,
        zigbee.clone(),
        state,
        mqtt.clone(),
        settings,
        logger,
    ));
    (controller, zigbee, mqtt)
}

#[tokio::test]
async fn start_fails_when_zigbee_start_fails() {
    let (controller, zigbee, mqtt) = harness("mqtt://127.0.0.1:1");
    *zigbee.test_start.lock().unwrap() = Some((Err("boom".to_string()), 0));

    let error = controller.start().await.expect_err("zigbee failure");
    assert!(error.contains("boom"), "{error}");
    assert!(!mqtt.lock().await.is_connected());
    assert_eq!(controller.extensions_len(), 0, "no extension was started");
}

#[tokio::test]
async fn start_returns_early_when_zigbee_reports_not_started() {
    let (controller, zigbee, mqtt) = harness("mqtt://127.0.0.1:1");
    *zigbee.test_start.lock().unwrap() = Some((Ok(false), 0));

    controller.start().await.expect("start returns early");
    assert!(!mqtt.lock().await.is_connected());
    assert_eq!(controller.extensions_len(), 0);
}

#[tokio::test]
async fn aborting_start_during_the_coordinator_phase_skips_extensions() {
    let (controller, zigbee, mqtt) = harness("mqtt://127.0.0.1:1");
    *zigbee.test_start.lock().unwrap() = Some((Ok(true), 500));

    // Poll the start future for 100 ms, then drop it: dropping the future is the
    // Rust equivalent of aborting the upstream AbortSignal.
    let start = controller.start();
    tokio::pin!(start);
    let _ = tokio::time::timeout(std::time::Duration::from_millis(100), &mut start).await;
    drop(start);

    assert!(
        !mqtt.lock().await.is_connected(),
        "MQTT was never connected"
    );
    assert_eq!(controller.extensions_len(), 0, "no extension was started");
}

#[tokio::test]
async fn mqtt_connect_failure_returns_error_and_stops_zigbee() {
    let (controller, zigbee, _mqtt) = harness("mqtt://127.0.0.1:1");
    *zigbee.test_start.lock().unwrap() = Some((Ok(true), 0));

    let error = controller.start().await.expect_err("mqtt failure");
    assert!(
        error.contains("MQTT failed to connect"),
        "unexpected error: {error}"
    );
    assert_eq!(
        zigbee
            .test_stop_count
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "zigbee-herdsman must be stopped when the MQTT connect fails"
    );
}

/// Upstream `EventBus should handle sync errors`: one throwing handler must not
/// stop the others nor escape the emitter.
#[tokio::test]
async fn event_bus_contains_handler_panics() {
    let mut bus = EventBus::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let first = seen.clone();
    bus.on_permit_join_changed(
        1,
        Arc::new(move |_| {
            first.lock().unwrap().push("first");
            panic!("handler failure");
        }),
    );
    let second = seen.clone();
    bus.on_permit_join_changed(
        2,
        Arc::new(move |_| {
            second.lock().unwrap().push("second");
        }),
    );

    bus.emit_permit_join_changed(json!({"state": true}));
    bus.emit_permit_join_changed(json!({"state": false}));

    assert_eq!(
        *seen.lock().unwrap(),
        vec!["first", "second", "first", "second"],
        "the second handler still runs and later emits keep working"
    );
}

/// A test extension whose `start` succeeds and whose `stop` fails.
struct FailingExtension {
    context: z2m_rs::extension::extension::ExtensionContext,
    stopped: Arc<Mutex<bool>>,
}

impl z2m_rs::extension::extension::Extension for FailingExtension {
    fn name(&self) -> &'static str {
        "FailingTest"
    }

    fn context(&self) -> &z2m_rs::extension::extension::ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> z2m_rs::extension::extension::ExtensionFuture {
        Box::pin(async move { Ok(()) })
    }

    fn stop(self: Arc<Self>) -> z2m_rs::extension::extension::ExtensionFuture {
        let stopped = self.stopped.clone();
        Box::pin(async move {
            *stopped.lock().unwrap() = true;
            Err("stop failed".to_string())
        })
    }
}

/// Upstream `does not throw when extension stop throws`: a failing `stop` is
/// logged, the remaining extensions are still stopped and the controller keeps
/// tearing down MQTT/zigbee-herdsman.
#[tokio::test]
async fn stop_continues_when_an_extension_fails_to_stop() {
    let (controller, zigbee, _mqtt) = harness("mqtt://127.0.0.1:1");
    let failing_stopped = Arc::new(Mutex::new(false));
    let failing_context = failing_extension_context(&controller);
    controller
        .add_extension(Arc::new(FailingExtension {
            context: failing_context,
            stopped: failing_stopped.clone(),
        }))
        .expect("add failing extension");

    let bridge = controller.make_extension("Bridge").expect("bridge");
    controller
        .add_extension(bridge.clone())
        .expect("add bridge");

    controller.stop(false, 0).await;

    assert!(
        *failing_stopped.lock().unwrap(),
        "the failing stop still ran"
    );
    assert_eq!(
        zigbee
            .test_stop_count
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "teardown continues after the failing extension"
    );
}

/// Reuse the controller's own extension context for the test extension.
fn failing_extension_context(
    controller: &Arc<Controller>,
) -> z2m_rs::extension::extension::ExtensionContext {
    let bridge = controller.make_extension("Bridge").expect("bridge");
    let mut context = bridge.context().clone();
    context.key = 101;
    context
}

/// Upstream `prevents mismatching setting/extension state` (homeassistant.test.ts):
/// the settings flag and the enable/disable request must agree.
#[tokio::test]
async fn homeassistant_enable_disable_requires_matching_settings() {
    let (controller, _zigbee, _mqtt) = harness("mqtt://127.0.0.1:1");
    controller.settings.lock().unwrap().configuration["homeassistant"] = json!({"enabled": true});

    let error = controller
        .enable_disable_extension(false, "HomeAssistant".to_string())
        .await
        .expect_err("disable while enabled in settings");
    assert_eq!(
        error,
        "Tried to disable HomeAssistant extension enabled in settings"
    );

    controller.settings.lock().unwrap().configuration["homeassistant"] = json!({"enabled": false});
    let error = controller
        .enable_disable_extension(true, "HomeAssistant".to_string())
        .await
        .expect_err("enable while disabled in settings");
    assert_eq!(
        error,
        "Tried to enable HomeAssistant extension disabled in settings"
    );

    controller
        .enable_disable_extension(false, "HomeAssistant".to_string())
        .await
        .expect("disable while disabled in settings is allowed");
}
