// Original: .upstream/zigbee-herdsman/src/utils/logger.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::sync::{Arc, OnceLock, RwLock};

use crate::utils::datetime::now_iso8601;

/// Upstream `Logger` interface.
pub trait Logger: Send + Sync {
    /// Upstream `debug(messageOrLambda, namespace)`.
    fn debug(&self, message: &str, namespace: &str);
    /// Upstream `info(messageOrLambda, namespace)`.
    fn info(&self, message: &str, namespace: &str);
    /// Upstream `warning(messageOrLambda, namespace)`.
    fn warning(&self, message: &str, namespace: &str);
    /// Upstream `error(message, namespace)`.
    fn error(&self, message: &str, namespace: &str);
}

/// Upstream default console logger.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleLogger;

impl ConsoleLogger {
    fn format(&self, message: &str, namespace: &str) -> String {
        format!("[{}] {}: {}", now_iso8601(), namespace, message)
    }
}

impl Logger for ConsoleLogger {
    fn debug(&self, message: &str, namespace: &str) {
        println!("{}", self.format(message, namespace));
    }

    fn info(&self, message: &str, namespace: &str) {
        println!("{}", self.format(message, namespace));
    }

    fn warning(&self, message: &str, namespace: &str) {
        eprintln!("{}", self.format(message, namespace));
    }

    fn error(&self, message: &str, namespace: &str) {
        eprintln!("{}", self.format(message, namespace));
    }
}

static LOGGER: OnceLock<RwLock<Arc<dyn Logger>>> = OnceLock::new();

fn cell() -> &'static RwLock<Arc<dyn Logger>> {
    LOGGER.get_or_init(|| RwLock::new(Arc::new(ConsoleLogger)))
}

/// Upstream mutable `logger` export.
pub fn logger() -> Arc<dyn Logger> {
    cell()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Upstream `setLogger(l)`.
pub fn set_logger(next: Arc<dyn Logger>) {
    *cell()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = next;
}

/// Default `ConsoleLogger` instance.
pub fn console_logger() -> Arc<ConsoleLogger> {
    Arc::new(ConsoleLogger)
}
