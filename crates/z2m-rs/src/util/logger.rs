// Original: .upstream/zigbee2mqtt/lib/util/logger.ts
// TODO: port real winston transports (currently modelled in-memory).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

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
    pub advanced: Value,
    /// Test hook mirroring `vi.spyOn(fs, "rmSync").mockImplementationOnce(throw)`.
    pub remove_error: Option<String>,
    pub log_console_json: bool,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: "info".to_string(),
            advanced: json!({}),
            ..Default::default()
        }
    }

    fn advanced_str(&self, key: &str) -> Option<&str> {
        self.advanced.get("advanced")?.get(key)?.as_str()
    }

    fn advanced_u64(&self, key: &str) -> Option<u64> {
        self.advanced.get("advanced")?.get(key)?.as_u64()
    }

    fn advanced_bool(&self, key: &str) -> Option<bool> {
        self.advanced.get("advanced")?.get(key)?.as_bool()
    }

    /// Upstream: `logger.init()`.
    pub fn init(&mut self) {
        self.log_console_json = self.advanced_bool("log_console_json").unwrap_or(false);

        let outputs: Vec<String> = self
            .advanced
            .get("advanced")
            .and_then(|advanced| advanced.get("log_output"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_else(|| vec!["console".to_string(), "file".to_string()]);

        self.transports.clear();
        self.transports.push(Transport {
            kind: "Console".to_string(),
            silent: !outputs.iter().any(|output| output == "console"),
            ..Transport::default()
        });

        if outputs.iter().any(|output| output == "file") {
            let directory = self.init_file_directory();
            self.cleanup_log_directories();
            let rotation = self.advanced_bool("log_rotation").unwrap_or(true);
            let transport = Transport {
                kind: "File".to_string(),
                silent: false,
                dirname: directory.clone(),
                max_files: rotation.then_some(10),
                tailable: rotation,
                maxsize: rotation.then_some(10 * 1024 * 1024),
            };
            self.transports.push(transport);

            if self.advanced_bool("log_symlink_current").unwrap_or(false) {
                if let (Some(directory), Some(parent)) =
                    (directory, Path::new(&self.log_directory_pattern()).parent())
                {
                    let link = parent.join("current");
                    let _ = std::fs::remove_file(&link);
                    let _ = std::os::unix::fs::symlink(&directory, &link);
                }
            }
        }

        self.namespaced_levels = self
            .advanced
            .get("advanced")
            .and_then(|advanced| advanced.get("log_namespaced_levels"))
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.cached_namespaced_levels = self.namespaced_levels.clone();

        if let Some(ignore) = self
            .advanced_str("log_debug_namespace_ignore")
            .map(str::to_string)
        {
            self.set_debug_namespace_ignore(&ignore);
        }
    }

    fn log_directory_pattern(&self) -> String {
        self.advanced_str("log_directory")
            .unwrap_or("data/log/%TIMESTAMP%")
            .to_string()
    }

    fn init_file_directory(&mut self) -> Option<String> {
        let pattern = self.log_directory_pattern();
        let directory = if pattern.contains("%TIMESTAMP%") {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let (year, month, day) = civil_from_days(now.div_euclid(86_400));
            let seconds = now.rem_euclid(86_400);
            let stamp = format!(
                "{year:04}-{month:02}-{day:02}.{:02}-{:02}-{:02}",
                seconds / 3600,
                (seconds % 3600) / 60,
                seconds % 60
            );
            pattern.replace("%TIMESTAMP%", &stamp)
        } else {
            pattern
        };

        let _ = std::fs::create_dir_all(&directory);
        Some(directory)
    }

    fn cleanup_log_directories(&mut self) {
        let pattern = self.log_directory_pattern();
        if !pattern.contains("%TIMESTAMP%") {
            return;
        }

        let Some(parent) = Path::new(&pattern).parent() else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(parent) else {
            return;
        };
        let mut directories: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();

        let keep = self.advanced_u64("log_directories_to_keep").unwrap_or(10) as usize;
        if directories.len() <= keep {
            return;
        }
        directories.sort();
        let remove_count = directories.len() - keep;
        for directory in directories.into_iter().take(remove_count) {
            if let Some(error) = self.remove_error.clone() {
                self.winston_logs.push((
                    "error".to_string(),
                    format!(
                        "Failed to remove old log directory '{}': {error}",
                        directory.display()
                    ),
                ));
            } else {
                let _ = std::fs::remove_dir_all(directory);
            }
        }
    }

    pub fn set_level(&mut self, level: &str) {
        self.level = level.to_string();
        self.cached_namespaced_levels = self.namespaced_levels.clone();
    }

    pub fn get_level(&self) -> String {
        self.level.clone()
    }

    /// Upstream: `logger.winston.level` (always "debug").
    pub fn winston_level(&self) -> String {
        "debug".to_string()
    }

    pub fn set_namespaced_levels(&mut self, levels: BTreeMap<String, String>) {
        self.namespaced_levels = levels;
        self.cached_namespaced_levels = self.namespaced_levels.clone();
    }

    pub fn get_namespaced_levels(&self) -> BTreeMap<String, String> {
        self.namespaced_levels.clone()
    }

    pub fn get_cached_namespaced_levels(&self) -> BTreeMap<String, String> {
        self.cached_namespaced_levels.clone()
    }

    pub fn set_debug_namespace_ignore(&mut self, ignore: &str) {
        self.debug_namespace_ignore = ignore.to_string();
        self.debug_namespace_ignore_regex = (!ignore.is_empty()).then(|| ignore.to_string());
    }

    pub fn get_debug_namespace_ignore(&self) -> String {
        self.debug_namespace_ignore.clone()
    }

    /// Upstream: `logger.debugNamespaceIgnoreRegex.test(namespace)`.
    pub fn debug_namespace_ignore_matches(&self, namespace: &str) -> bool {
        let Some(pattern) = &self.debug_namespace_ignore_regex else {
            return false;
        };

        split_alternatives(pattern).iter().any(|alternative| {
            let mut prefix = alternative.trim_start_matches('^');
            let mut alternatives: Vec<String> = vec![prefix.to_string()];
            if let (Some(open), Some(close)) = (prefix.find('('), prefix.find(')')) {
                if close > open {
                    let before = &prefix[..open];
                    let after = &prefix[close + 1..];
                    let group = &prefix[open + 1..close];
                    alternatives = group
                        .split('|')
                        .map(|item| format!("{before}{item}{after}"))
                        .collect();
                }
            }
            alternatives
                .iter()
                .any(|prefix| namespace.starts_with(prefix.as_str()))
        })
    }

    pub fn add_transport(&mut self, transport: Transport) {
        self.transports.push(transport);
    }

    /// Upstream removes by identity; tests pass the same kind/name.
    pub fn remove_transport(&mut self, kind: &str) {
        self.transports.retain(|transport| transport.kind != kind);
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

    fn severity(level: &str) -> u8 {
        match level {
            "debug" => 0,
            "info" => 1,
            "warning" => 2,
            "error" => 3,
            _ => 1,
        }
    }

    fn resolve_level(&self, namespace: &str) -> String {
        let mut best: Option<(usize, String)> = None;
        for (prefix, level) in &self.namespaced_levels {
            if namespace == prefix || namespace.starts_with(&format!("{prefix}:")) {
                if best
                    .as_ref()
                    .is_none_or(|(length, _)| prefix.len() > *length)
                {
                    best = Some((prefix.len(), level.clone()));
                }
            }
        }

        best.map(|(_, level)| level)
            .unwrap_or_else(|| self.level.clone())
    }

    /// Upstream: `logger[level](message, namespace)`.
    pub fn log(&mut self, level: &str, message: &str, namespace: &str) {
        if level == "debug" && self.debug_namespace_ignore_matches(namespace) {
            return;
        }

        let effective = self.resolve_level(namespace);
        let accepted = Self::severity(level) >= Self::severity(&effective);
        let was_cached = self.cached_namespaced_levels.contains_key(namespace);

        if accepted {
            self.cached_namespaced_levels = self.namespaced_levels.clone();
            if !self.namespaced_levels.contains_key(namespace) {
                self.cached_namespaced_levels
                    .insert(namespace.to_string(), effective);
            }
        } else if was_cached {
            self.cached_namespaced_levels = self.namespaced_levels.clone();
        } else {
            self.cached_namespaced_levels
                .insert(namespace.to_string(), effective);
        }

        if !accepted {
            return;
        }

        let message = format!("{namespace}: {message}");
        self.winston_logs.push((level.to_string(), message.clone()));

        if self.log_console_json {
            self.console_writes.push(
                json!({"level": level, "message": message, "timestamp": timestamp()}).to_string(),
            );
        } else {
            let color = match level {
                "error" => 31,
                "warning" => 33,
                "debug" => 34,
                _ => 32,
            };
            self.console_writes
                .push(format!("\u{1b}[{color}m{level}\u{1b}[39m: \t{message}"));
        }
    }

    /// Upstream: `logger.end()`.
    pub fn end(&mut self) {}
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m, d)
}

fn split_alternatives(pattern: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in pattern.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            '|' if depth == 0 => {
                result.push(std::mem::take(&mut current));
            }
            _ => current.push(character),
        }
    }
    result.push(current);
    result
}
