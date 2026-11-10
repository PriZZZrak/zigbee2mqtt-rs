// Test support: serialize tests that mutate the process-wide Entity injection.

use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn serial() -> MutexGuard<'static, ()> {
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
