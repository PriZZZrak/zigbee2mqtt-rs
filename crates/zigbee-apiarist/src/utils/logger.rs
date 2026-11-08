// Original: .upstream/zigbee-herdsman/src/utils/logger.ts
// TODO: port public API from the original file.

/// Upstream console logger used for coverage (`consoleLogger.warning/error`).
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleLogger;

impl ConsoleLogger {
    pub fn warning(&self, message: &str, namespace: &str) {
        eprintln!("[WARNING] ({namespace}) {message}");
    }

    pub fn error(&self, message: &str, namespace: &str) {
        eprintln!("[ERROR] ({namespace}) {message}");
    }
}

pub fn console_logger() -> ConsoleLogger {
    ConsoleLogger
}
