// Original: .upstream/zigbee2mqtt/lib/util/logger.ts
// TODO: port the winston-based logger.
//
// Tests model winston transports and console output explicitly so the same
// observables can be asserted without node streams.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Transport {
    /// Upstream `constructor.name`: "Console" or "File".
    pub kind: String,
    pub silent: bool,
    pub dirname: Option<String>,
    pub max_files: Option<u32>,
    pub tailable: bool,
    pub maxsize: Option<u64>,
}

#[derive(Debug, Default)]
pub struct Logger {
    pub level: String,
    pub namespaced_levels: BTreeMap<String, String>,
    /// `logger.cachedNamespacedLevels` (private upstream).
    pub cached_namespaced_levels: BTreeMap<String, String>,
    pub debug_namespace_ignore: String,
    pub debug_namespace_ignore_regex: Option<String>,
    pub transports: Vec<Transport>,
    /// Everything written to `console._stdout`.
    pub console_writes: Vec<String>,
    /// Calls to `logger.winston.log(level, message)`.
    pub winston_logs: Vec<(String, String)>,
    /// Upstream `settings.advanced`. Used by `init`.
    pub advanced: serde_json::Value,
    /// Test hook mirroring `vi.spyOn(fs, "rmSync").mockImplementationOnce(throw)`.
    pub remove_error: Option<String>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: "info".to_string(),
            advanced: serde_json::json!({}),
            ..Default::default()
        }
    }

    /// Upstream: `logger.init()`.
    pub fn init(&mut self) {
        todo!()
    }

    pub fn set_level(&mut self, level: &str) {
        todo!()
    }

    pub fn get_level(&self) -> String {
        todo!()
    }

    /// Upstream: `logger.winston.level` (always "debug").
    pub fn winston_level(&self) -> String {
        todo!()
    }

    pub fn set_namespaced_levels(&mut self, levels: BTreeMap<String, String>) {
        todo!()
    }

    pub fn get_namespaced_levels(&self) -> BTreeMap<String, String> {
        todo!()
    }

    pub fn get_cached_namespaced_levels(&self) -> BTreeMap<String, String> {
        todo!()
    }

    pub fn set_debug_namespace_ignore(&mut self, ignore: &str) {
        todo!()
    }

    pub fn get_debug_namespace_ignore(&self) -> String {
        todo!()
    }

    /// Upstream: `logger.debugNamespaceIgnoreRegex.test(namespace)`.
    pub fn debug_namespace_ignore_matches(&self, namespace: &str) -> bool {
        todo!()
    }

    pub fn add_transport(&mut self, transport: Transport) {
        todo!()
    }

    /// Upstream removes by identity; tests pass the same kind/name.
    pub fn remove_transport(&mut self, kind: &str) {
        todo!()
    }

    pub fn debug(&mut self, message: &str, namespace: &str) {
        self.log("debug", message, namespace)
    }

    pub fn info(&mut self, message: &str, namespace: &str) {
        self.log("info", message, namespace)
    }

    pub fn warning(&mut self, message: &str, namespace: &str) {
        self.log("warning", message, namespace)
    }

    pub fn error(&mut self, message: &str, namespace: &str) {
        self.log("error", message, namespace)
    }

    /// Upstream: `logger[level](message, namespace)`.
    pub fn log(&mut self, _level: &str, _message: &str, _namespace: &str) {
        todo!()
    }

    /// Upstream: `logger.end()`.
    pub fn end(&mut self) {
        todo!()
    }
}
