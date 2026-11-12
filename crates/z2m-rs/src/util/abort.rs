// Original: .upstream/zigbee2mqtt/lib/controller.ts (`#startAbortController`)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Minimal `AbortSignal` equivalent: upstream passes the signal to
// `zigbee.start(abortSignal)` and checks `aborted` between startup phases.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct AbortSignal {
    aborted: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `AbortController.abort(reason)`.
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
        self.notify.notify_one();
    }

    /// Upstream `signal.aborted`.
    pub fn aborted(&self) -> bool {
        self.aborted.load(Ordering::SeqCst)
    }

    /// Future that resolves once the signal is aborted.
    pub async fn wait(&self) {
        if self.aborted() {
            return;
        }

        let notified = self.notify.notified();
        tokio::pin!(notified);
        // Re-check after registration to close the race with `abort()`.
        if self.aborted() {
            return;
        }
        notified.await;
    }
}
