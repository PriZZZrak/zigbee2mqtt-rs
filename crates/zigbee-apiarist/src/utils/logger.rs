// Original: .upstream/zigbee-herdsman/src/utils/logger.ts
// TODO: port public API from the original file.

// pub fn setLogger() { todo!() }

pub fn set_logger() {
    todo!()
}

/// Upstream console logger used for coverage (`consoleLogger.warning/error`).
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleLogger;

impl ConsoleLogger {
    pub fn warning(&self, message: &str, namespace: &str) {
        todo!()
    }

    pub fn error(&self, message: &str, namespace: &str) {
        todo!()
    }
}

pub fn console_logger() -> ConsoleLogger {
    ConsoleLogger
}
