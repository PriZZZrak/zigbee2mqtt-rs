// Original: .upstream/zigbee-herdsman/src/utils/queue.ts

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};

/// Upstream `Queue`: bounded concurrency, optional per-key serialization.
#[derive(Debug, Clone)]
pub struct Queue {
    semaphore: Arc<Semaphore>,
    key_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    pending: Arc<AtomicUsize>,
}

impl Queue {
    pub fn new(concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(concurrent.max(1))),
            key_locks: Arc::new(Mutex::new(HashMap::new())),
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Upstream `execute(func, key)`.
    pub async fn execute<F, Fut, T>(&self, function: F, key: Option<String>) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        self.pending.fetch_add(1, Ordering::SeqCst);
        let permit = self
            .semaphore
            .acquire()
            .await
            .expect("queue semaphore closed");
        self.pending.fetch_sub(1, Ordering::SeqCst);

        let key_lock = match &key {
            Some(key) => {
                let mut locks = self.key_locks.lock().await;
                Some(
                    locks
                        .entry(key.clone())
                        .or_insert_with(|| Arc::new(Mutex::new(())))
                        .clone(),
                )
            }
            None => None,
        };
        let key_guard = match &key_lock {
            Some(lock) => Some(lock.lock().await),
            None => None,
        };

        let result = function().await;

        drop(key_guard);
        if let Some(key) = key {
            let mut locks = self.key_locks.lock().await;
            if let Some(lock) = locks.get(&key) {
                if Arc::strong_count(lock) <= 2 {
                    locks.remove(&key);
                }
            }
        }
        drop(permit);
        result
    }

    /// Upstream `clear()` (running jobs cannot be cancelled).
    pub fn clear(&self) {
        self.pending.store(0, Ordering::SeqCst);
    }

    /// Upstream `count()` (queued jobs).
    pub fn count(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new(1)
    }
}
