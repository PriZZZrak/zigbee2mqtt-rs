// Port of .upstream/zigbee2mqtt/test/sd-notify.test.ts
//
// Upstream mocks `unix-dgram`, `node:os` platform and vitest fake timers.
// The Rust port injects the same controls through `SdNotifyOptions` and the
// `advance` hook.

use std::sync::{Arc, Mutex};

use z2m_rs::util::sd_notify::{init_sd_notify_with, SdNotify, SdNotifyOptions};

fn logs() -> (Arc<Mutex<Vec<String>>>, Arc<dyn Fn(&str) + Send + Sync>) {
    let sink = Arc::new(Mutex::new(Vec::<String>::new()));
    let handle = Arc::clone(&sink);
    (
        sink,
        Arc::new(move |message: &str| handle.lock().unwrap().push(message.to_string())),
    )
}

fn options() -> SdNotifyOptions {
    SdNotifyOptions::default()
}

fn sent(notify: &SdNotify) -> Vec<String> {
    notify.messages.lock().unwrap().clone()
}

fn attempts(notify: &SdNotify) -> Vec<String> {
    notify.attempts.lock().unwrap().clone()
}

#[tokio::test]
async fn no_socket() {
    let res = init_sd_notify_with(options()).await;

    assert!(res.is_none());
}

#[tokio::test]
async fn error_on_unsupported_platform() {
    let (warnings, log_warning) = logs();
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        platform: "win32".to_string(),
        log_warning,
        ..options()
    })
    .await;

    assert!(res.is_none());
    assert_eq!(
        warnings.lock().unwrap().as_slice(),
        ["NOTIFY_SOCKET env is set: Unix datagrams not available on this platform"]
    );
}

#[tokio::test]
async fn error_on_supported_platform() {
    let (errors, log_error) = logs();
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        create_socket_error: Some("Error create socket".to_string()),
        log_error,
        ..options()
    })
    .await;

    assert!(res.is_none());
    assert_eq!(
        errors.lock().unwrap().as_slice(),
        ["Could not init sd_notify: Error create socket"]
    );
}

#[tokio::test]
async fn socket_only() {
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        ..options()
    })
    .await
    .expect("notifier");

    assert_eq!(sent(&res), ["READY=1"]);

    res.advance(7500);

    res.notify_stopping();
    assert_eq!(sent(&res), ["READY=1", "STOPPING=1"]);

    res.stop();
    assert_eq!(sent(&res), ["READY=1", "STOPPING=1"]);
}

#[tokio::test]
async fn invalid_watchdog_timeout_socket_only() {
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        watchdog_usec: Some("mocked".to_string()),
        ..options()
    })
    .await
    .expect("notifier");

    assert_eq!(sent(&res), ["READY=1"]);

    res.advance(7500);

    res.notify_stopping();
    assert_eq!(sent(&res), ["READY=1", "STOPPING=1"]);

    res.stop();
    assert_eq!(sent(&res), ["READY=1", "STOPPING=1"]);
}

#[tokio::test]
async fn socket_and_watchdog() {
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        watchdog_usec: Some("10000000".to_string()),
        ..options()
    })
    .await
    .expect("notifier");

    assert_eq!(sent(&res), ["READY=1"]);

    res.advance(7500);
    assert_eq!(sent(&res), ["READY=1", "WATCHDOG=1"]);

    res.notify_stopping();
    assert_eq!(sent(&res), ["READY=1", "WATCHDOG=1", "STOPPING=1"]);

    res.advance(6000);
    assert_eq!(
        sent(&res),
        ["READY=1", "WATCHDOG=1", "STOPPING=1", "WATCHDOG=1"]
    );

    res.stop();
    assert_eq!(
        sent(&res),
        ["READY=1", "WATCHDOG=1", "STOPPING=1", "WATCHDOG=1"]
    );

    res.advance(10000);
    assert_eq!(
        sent(&res),
        ["READY=1", "WATCHDOG=1", "STOPPING=1", "WATCHDOG=1"]
    );
}

#[tokio::test]
async fn fails_to_send() {
    let (warnings, log_warning) = logs();
    let res = init_sd_notify_with(SdNotifyOptions {
        notify_socket: Some("mocked".to_string()),
        send_error: Some("Failure".to_string()),
        log_warning,
        ..options()
    })
    .await
    .expect("notifier");

    assert_eq!(attempts(&res), ["READY=1"]);
    assert_eq!(
        warnings.lock().unwrap().as_slice(),
        ["Failed to send \"READY=1\" to systemd: Failure"]
    );
}
