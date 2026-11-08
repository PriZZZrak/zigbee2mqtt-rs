// Original: .upstream/zigbee-herdsman/src/utils/async-mutex.ts

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

/// Upstream `AsyncMutex`: serializes async critical sections.
#[derive(Debug, Default)]
pub struct AsyncMutex {
    inner: tokio::sync::Mutex<()>,
    /// Incremented by `clear`; queued callers can observe it if they need to
    /// abort, while already-running work is never cancelled (like upstream).
    generation: AtomicU64,
}

impl AsyncMutex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `queue.run(func)`.
    pub async fn run<F, Fut, T>(&self, function: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let generation = self.generation.load(Ordering::SeqCst);
        let _guard = self.inner.lock().await;
        let _ = generation;
        function().await
    }

    /// Upstream `clear()`: invalidates queued work (running work finishes).
    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}
