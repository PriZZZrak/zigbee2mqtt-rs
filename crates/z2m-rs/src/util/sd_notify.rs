// Original: .upstream/zigbee2mqtt/lib/util/sd-notify.ts
// TODO: port the systemd notification implementation.
//
// Upstream mocks `unix-dgram`, `node:os` and fake timers. The Rust port keeps
// the same observables through an injectable options struct and an explicit
// `advance` hook on the returned handle.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;

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

#[derive(Clone, Default)]
pub struct SdNotify {
    /// Successfully delivered notifications.
    pub messages: Arc<Mutex<Vec<String>>>,
    /// Attempted notifications, including ones whose send failed.
    pub attempts: Arc<Mutex<Vec<String>>>,
    pub stopped: Arc<AtomicBool>,
}

impl SdNotify {
    /// Upstream: `notifyStopping()`.
    pub fn notify_stopping(&self) {
        todo!()
    }

    /// Upstream: `stop()`.
    pub fn stop(&self) {
        todo!()
    }

    /// Fake-timer replacement: advance the watchdog clock by `milliseconds`.
    pub fn advance(&self, milliseconds: u64) {
        todo!()
    }
}

/// Upstream: `initSdNotify()` reading `NOTIFY_SOCKET`/`WATCHDOG_USEC`.
pub async fn init_sd_notify() -> Option<SdNotify> {
    todo!()
}

/// Testable variant with injected platform/socket/logger.
pub async fn init_sd_notify_with(options: SdNotifyOptions) -> Option<SdNotify> {
    todo!()
}
