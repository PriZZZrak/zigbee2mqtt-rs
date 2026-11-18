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

#[path = "support/mqtt_broker.rs"]
mod mqtt_broker;
use mqtt_broker::Broker;

async fn wait_for<F: Fn() -> bool>(what: &str, timeout: std::time::Duration, predicate: F) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if predicate() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("timeout waiting for {what}");
}

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

/// Upstream prevents-mismatching-setting-extension-state for Frontend
/// (frontend.test.ts).
#[tokio::test]
async fn frontend_enable_disable_requires_matching_settings() {
    let (controller, _zigbee, _mqtt) = harness("mqtt://127.0.0.1:1");
    controller.settings.lock().unwrap().configuration["frontend"] = json!({"enabled": true});
    let error = controller
        .enable_disable_extension(false, "Frontend".to_string())
        .await
        .expect_err("disable while enabled in settings");
    assert_eq!(
        error,
        "Tried to disable Frontend extension enabled in settings"
    );

    controller.settings.lock().unwrap().configuration["frontend"] = json!({"enabled": false});
    let error = controller
        .enable_disable_extension(true, "Frontend".to_string())
        .await
        .expect_err("enable while disabled in settings");
    assert_eq!(
        error,
        "Tried to enable Frontend extension disabled in settings"
    );

    controller
        .enable_disable_extension(false, "Frontend".to_string())
        .await
        .expect("disable while disabled in settings is allowed");
}

/// CP-04 regression: a failed startup must not wipe the state cache when no
/// herdsman controller exists (upstream `state.stop()` only prunes then).
#[tokio::test]
async fn failed_startup_keeps_the_state_cache() {
    let (controller, zigbee, _mqtt) = harness("mqtt://127.0.0.1:1");
    *zigbee.test_start.lock().unwrap() = Some((Err("boom".to_string()), 0));

    let error = controller.start().await.expect_err("startup failure");
    assert!(error.contains("boom"), "{error}");
    controller
        .state
        .lock()
        .unwrap()
        .test_insert("0xa4c13887bf43cc0f", json!({"state": "ON"}));
    controller.stop(false, 0).await;

    assert!(
        controller
            .state
            .lock()
            .unwrap()
            .exists(&z2m_rs::model::entity::EntityId::Ieee(
                "0xa4c13887bf43cc0f".to_string()
            )),
        "state cache was pruned without a herdsman controller"
    );
    assert_eq!(
        controller
            .state
            .lock()
            .unwrap()
            .get(&z2m_rs::model::entity::EntityId::Ieee(
                "0xa4c13887bf43cc0f".to_string()
            )),
        json!({"state": "ON"})
    );
}

/// CP-05: abort after MQTT is connected and three extensions are running.
///
/// Upstream `controller.test.ts` "during extension start": `stop()` aborts the
/// in-flight start, all started extensions are stopped, MQTT is disconnected
/// and the controller can start again.
#[tokio::test]
async fn aborting_with_active_mqtt_stops_runtime_and_restarts_cleanly() {
    let broker = Broker::start().await;
    let (controller, zigbee, mqtt) = harness(&broker.server());
    *zigbee.test_start.lock().unwrap() = Some((Ok(true), 0));

    let release = Arc::new(tokio::sync::Notify::new());
    *controller.test_start_pause.lock().unwrap() = Some((3, release));

    let start = tokio::spawn({
        let controller = controller.clone();
        async move { controller.start().await }
    });

    wait_for("MQTT connect", std::time::Duration::from_secs(5), || {
        mqtt.try_lock()
            .map(|guard| guard.is_connected())
            .unwrap_or(false)
    })
    .await;
    wait_for(
        "three extensions",
        std::time::Duration::from_secs(5),
        || controller.extensions_len() >= 3,
    )
    .await;
    assert_eq!(
        controller.extensions_len(),
        3,
        "start must pause before the fourth extension"
    );
    assert_eq!(
        broker.active_connections(),
        1,
        "the client holds one MQTT connection"
    );
    assert!(
        broker
            .subscriptions()
            .iter()
            .any(|filter| filter == "z2m/#"),
        "the bridge subscribed on connect"
    );
    let subscriptions_before = broker.subscriptions().len();
    assert!(
        controller.event_bus.lock().unwrap().listener_count() > 0,
        "the started extensions registered event-bus listeners"
    );
    assert!(
        controller.state.lock().unwrap().has_save_timer(),
        "the state save timer is running"
    );

    controller.stop(false, 0).await;
    start
        .await
        .expect("start task")
        .expect("an aborted start returns Ok");

    assert!(!mqtt.lock().await.is_connected(), "MQTT was disconnected");
    // Upstream keeps the extension instances registered after stop.
    assert_eq!(controller.extensions_len(), 3);
    assert_eq!(
        controller.event_bus.lock().unwrap().listener_count(),
        0,
        "every started extension removed its listeners in stop()"
    );
    assert!(
        !controller.state.lock().unwrap().has_save_timer(),
        "the state save timer was stopped"
    );
    wait_for(
        "broker sees the closed connection",
        std::time::Duration::from_secs(5),
        || broker.active_connections() == 0,
    )
    .await;

    // A fresh controller over the same shared state is the in-process
    // equivalent of the process restart: it must reconnect, resubscribe and
    // start the full built-in extension set.
    let restarted = Arc::new(Controller::new(
        controller.event_bus.clone(),
        controller.zigbee.clone(),
        controller.state.clone(),
        controller.mqtt.clone(),
        controller.settings.clone(),
        controller.logger.clone(),
    ));
    restarted.start().await.expect("second start");
    assert!(mqtt.lock().await.is_connected());
    assert_eq!(restarted.extensions_len(), 10);
    assert!(restarted.event_bus.lock().unwrap().listener_count() > 0);
    assert!(restarted.state.lock().unwrap().has_save_timer());
    wait_for("resubscribed", std::time::Duration::from_secs(5), || {
        broker.subscriptions().len() > subscriptions_before
    })
    .await;
    assert_eq!(broker.active_connections(), 1);

    restarted.stop(false, 0).await;
    assert!(!mqtt.lock().await.is_connected());
    assert_eq!(restarted.event_bus.lock().unwrap().listener_count(), 0);
    wait_for(
        "final disconnect",
        std::time::Duration::from_secs(5),
        || broker.active_connections() == 0,
    )
    .await;
}

/// CP-05: abort while `mqtt.connect()` is in flight. The connect is bounded
/// (5 s CONNACK timeout); `stop()` sets the abort flag first, so the failure is
/// reported as an aborted start (upstream `controller.ts:126-128,146-150`)
/// and no extension is started.
#[tokio::test]
async fn aborting_during_a_slow_mqtt_connect_returns_ok_and_tears_down() {
    // Accept TCP but never answer CONNECT.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("blackhole listener");
    let port = listener.local_addr().unwrap().port();
    let blackhole = tokio::spawn(async move {
        let mut sockets = Vec::new();
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            sockets.push(socket);
        }
    });

    let (controller, zigbee, mqtt) = harness(&format!("mqtt://127.0.0.1:{port}"));
    *zigbee.test_start.lock().unwrap() = Some((Ok(true), 0));

    let start = tokio::spawn({
        let controller = controller.clone();
        async move { controller.start().await }
    });
    wait_for(
        "MQTT connect in flight",
        std::time::Duration::from_secs(2),
        || mqtt.try_lock().is_err(),
    )
    .await;

    controller.stop(false, 0).await;
    let result = start.await.expect("start task");
    blackhole.abort();

    assert!(
        result.is_ok(),
        "an aborted connect is not a startup failure: {result:?}"
    );
    assert!(!mqtt.lock().await.is_connected());
    assert_eq!(controller.extensions_len(), 0, "no extension was started");
    assert!(!controller.state.lock().unwrap().has_save_timer());
}
