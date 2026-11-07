// Original: .upstream/zigbee2mqtt/lib/util/sd-notify.ts
// TODO: port the real unix-dgram sender (currently modelled in-memory).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SdNotifyOptions {
    pub notify_socket: Option<String>,
    pub watchdog_usec: Option<String>,
    /// `node:os` `platform()` replacement.
    pub platform: String,
    pub create_socket_error: Option<String>,
    pub send_error: Option<String>,
    pub log_warning: Arc<dyn Fn(&str) + Send + Sync>,
    pub log_error: Arc<dyn Fn(&str) + Send + Sync>,
}

impl Default for SdNotifyOptions {
    fn default() -> Self {
        Self {
            notify_socket: None,
            watchdog_usec: None,
            platform: "linux".to_string(),
            create_socket_error: None,
            send_error: None,
            log_warning: Arc::new(|_message: &str| {}),
            log_error: Arc::new(|_message: &str| {}),
        }
    }
}

#[derive(Clone)]
pub struct SdNotify {
    /// Successfully delivered notifications.
    pub messages: Arc<Mutex<Vec<String>>>,
    /// Attempted notifications, including ones whose send failed.
    pub attempts: Arc<Mutex<Vec<String>>>,
    pub stopped: Arc<AtomicBool>,
    /// Watchdog ping interval in milliseconds (upstream `wdUSec / 1000 / 2`).
    pub watchdog_interval_ms: Option<u64>,
    pub elapsed_ms: Arc<Mutex<u64>>,
    pub sent_watchdog_count: Arc<Mutex<u64>>,
    pub send_error: Option<String>,
    pub log_warning: Arc<dyn Fn(&str) + Send + Sync>,
}

impl SdNotify {
    fn send(&self, message: &str) {
        self.attempts.lock().unwrap().push(message.to_string());
        if let Some(error) = &self.send_error {
            (self.log_warning)(&format!("Failed to send \"{message}\" to systemd: {error}"));
        } else {
            self.messages.lock().unwrap().push(message.to_string());
        }
    }

    /// Upstream: `notifyStopping()`.
    pub fn notify_stopping(&self) {
        self.send("STOPPING=1");
    }

    /// Upstream: `stop()`.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    /// Fake-timer replacement: advance the watchdog clock by `milliseconds`.
    pub fn advance(&self, milliseconds: u64) {
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }
        let Some(interval) = self.watchdog_interval_ms else {
            return;
        };
        if interval == 0 {
            return;
        }

        let mut elapsed = self.elapsed_ms.lock().unwrap();
        *elapsed += milliseconds;
        let due = *elapsed / interval;
        let mut sent = self.sent_watchdog_count.lock().unwrap();
        while *sent < due {
            self.send("WATCHDOG=1");
            *sent += 1;
        }
    }
}

/// Upstream: `initSdNotify()` reading `NOTIFY_SOCKET`/`WATCHDOG_USEC`.
pub async fn init_sd_notify() -> Option<SdNotify> {
    init_sd_notify_with(SdNotifyOptions {
        notify_socket: std::env::var("NOTIFY_SOCKET").ok(),
        watchdog_usec: std::env::var("WATCHDOG_USEC").ok(),
        platform: std::env::consts::OS.to_string(),
        ..SdNotifyOptions::default()
    })
    .await
}

/// Testable variant with injected platform/socket/logger.
pub async fn init_sd_notify_with(options: SdNotifyOptions) -> Option<SdNotify> {
    let socket = options.notify_socket.as_ref()?;

    if options.platform != "linux" {
        (options.log_warning)(&format!(
            "NOTIFY_SOCKET env is set: Unix datagrams not available on this platform"
        ));
        return None;
    }

    if let Some(error) = &options.create_socket_error {
        (options.log_error)(&format!("Could not init sd_notify: {error}"));
        return None;
    }

    let _ = socket;
    let watchdog_interval_ms = options
        .watchdog_usec
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(|value| value / 1000 / 2);

    let notify = SdNotify {
        messages: Arc::new(Mutex::new(Vec::new())),
        attempts: Arc::new(Mutex::new(Vec::new())),
        stopped: Arc::new(AtomicBool::new(false)),
        watchdog_interval_ms,
        elapsed_ms: Arc::new(Mutex::new(0)),
        sent_watchdog_count: Arc::new(Mutex::new(0)),
        send_error: options.send_error.clone(),
        log_warning: Arc::clone(&options.log_warning),
    };

    notify.send("READY=1");
    Some(notify)
}
