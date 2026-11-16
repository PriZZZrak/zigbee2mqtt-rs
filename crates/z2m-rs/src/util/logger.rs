// Original: .upstream/zigbee2mqtt/lib/util/logger.ts

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use zigbee_apiarist::utils::datetime::{
    civil_from_days, local_offset_seconds, now_file_timestamp, now_iso8601, now_ms,
};

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

/// Upstream `winston-syslog` transport sockets (`logger.ts:111-129`).
pub enum SyslogSocket {
    Udp(std::net::UdpSocket),
    Tcp(std::sync::Mutex<std::net::TcpStream>),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixDatagram),
}

pub struct SyslogTransport {
    pub socket: SyslogSocket,
    pub app_name: String,
    pub facility: u8,
    pub hostname: String,
}

impl SyslogTransport {
    /// RFC 5424 message as produced by `winston-syslog`.
    fn format(&self, level: &str, message: &str) -> String {
        let severity = match level {
            "error" => 3,
            "warning" => 4,
            "info" => 6,
            _ => 7,
        };
        let priority = u16::from(self.facility) * 8 + severity;
        format!(
            "<{priority}>1 {} {} {} - - {}",
            now_iso8601(),
            self.hostname,
            self.app_name,
            message
        )
    }

    fn send(&self, level: &str, message: &str) {
        let payload = self.format(level, message);
        match &self.socket {
            SyslogSocket::Udp(socket) => {
                let _ = socket.send(payload.as_bytes());
            }
            SyslogSocket::Tcp(stream) => {
                use std::io::Write as _;
                if let Ok(mut stream) = stream.lock() {
                    let _ = stream.write_all(format!("{payload}\n").as_bytes());
                }
            }
            #[cfg(unix)]
            SyslogSocket::Unix(socket) => {
                let _ = socket.send(payload.as_bytes());
            }
        }
    }
}

#[derive(Default)]
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
    /// Upstream winston transport hook (`bridge.ts` EventTransport).
    pub log_listener: Option<Arc<dyn Fn(&str, &str, &str) + Send + Sync>>,
    /// Upstream `winston-syslog` transport (`log_output: ["syslog"]`).
    pub syslog: Option<SyslogTransport>,
    /// Append handle for the file transport (`log_output: ["file"]`).
    pub file_log: Option<std::fs::File>,
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
            if let Some(directory) = &directory {
                let _ = std::fs::create_dir_all(directory);
            }
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
            self.file_log = self.open_log_file(&directory);

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

        if outputs.iter().any(|output| output == "syslog") {
            self.syslog = self.init_syslog();
        } else {
            self.syslog = None;
        }

        if let Some(ignore) = self
            .advanced_str("log_debug_namespace_ignore")
            .map(str::to_string)
        {
            self.set_debug_namespace_ignore(&ignore);
        }
    }

    /// Upstream `winston-syslog` options (`logger.ts:111-129`).
    fn init_syslog(&self) -> Option<SyslogTransport> {
        let options = self
            .advanced
            .get("advanced")
            .and_then(|advanced| advanced.get("log_syslog"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let app_name = options
            .get("app_name")
            .or_else(|| options.get("appName"))
            .and_then(Value::as_str)
            .unwrap_or("Zigbee2MQTT")
            .to_string();
        let hostname = options
            .get("localhost")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| std::env::var("HOSTNAME").ok())
            .unwrap_or_else(|| "localhost".to_string());
        let facility_name = options
            .get("facility")
            .and_then(Value::as_str)
            .unwrap_or("local0");
        let facility = match facility_name {
            "kern" => 0,
            "user" => 1,
            "mail" => 2,
            "daemon" => 3,
            "auth" => 4,
            "syslog" => 5,
            "lpr" => 6,
            "news" => 7,
            "uucp" => 8,
            "cron" => 9,
            "authpriv" => 10,
            "ftp" => 11,
            "local1" => 17,
            "local2" => 18,
            "local3" => 19,
            "local4" => 20,
            "local5" => 21,
            "local6" => 22,
            "local7" => 23,
            _ => 16,
        };
        let kind = options
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("udp");
        let host = options
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("localhost");
        let port = options
            .get("port")
            .and_then(Value::as_u64)
            .or_else(|| {
                options
                    .get("port")
                    .and_then(Value::as_str)
                    .and_then(|port| port.parse().ok())
            })
            .unwrap_or(514);

        let socket = match kind {
            "tcp" => {
                let stream = std::net::TcpStream::connect((host, port as u16)).ok()?;
                SyslogSocket::Tcp(std::sync::Mutex::new(stream))
            }
            #[cfg(unix)]
            "unix" => {
                let path = options.get("path").and_then(Value::as_str)?;
                let socket = std::os::unix::net::UnixDatagram::unbound().ok()?;
                socket.connect(path).ok()?;
                SyslogSocket::Unix(socket)
            }
            _ => {
                let bind = if host.contains(':') { "[::]:0" } else { "0.0.0.0:0" };
                let socket = std::net::UdpSocket::bind(bind).ok()?;
                socket.connect((host, port as u16)).ok()?;
                SyslogSocket::Udp(socket)
            }
        };

        Some(SyslogTransport {
            socket,
            app_name,
            facility,
            hostname,
        })
    }

    fn open_log_file(&self, directory: &Option<String>) -> Option<std::fs::File> {
        let directory = directory.as_ref()?;
        std::fs::create_dir_all(directory).ok()?;
        let filename = self.advanced_str("log_file").unwrap_or("log.log");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(directory).join(filename))
            .ok()
    }

    /// Upstream `advanced.timestamp_format` (winston/fecha tokens).
    fn timestamp_pattern(&self) -> String {
        self.advanced_str("timestamp_format")
            .unwrap_or("YYYY-MM-DD HH:mm:ss")
            .to_string()
    }

    fn format_timestamp(&self) -> String {
        format_timestamp(now_ms() as i64, &self.timestamp_pattern())
    }

    fn log_directory_pattern(&self) -> String {
        self.advanced_str("log_directory")
            .unwrap_or("data/log/%TIMESTAMP%")
            .to_string()
    }

    fn init_file_directory(&mut self) -> Option<String> {
        let pattern = self.log_directory_pattern();
        let directory = if pattern.contains("%TIMESTAMP%") {
            pattern.replace("%TIMESTAMP%", &now_file_timestamp())
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
        // The directory created by this run is never a removal candidate; the
        // upstream cleanup keeps `log_directories_to_keep` newest directories
        // including the active one.
        let active = {
            let pattern = self.log_directory_pattern();
            pattern
                .contains("%TIMESTAMP%")
                .then(|| PathBuf::from(pattern.replace("%TIMESTAMP%", &now_file_timestamp())))
        };
        let mut directories: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter(|path| Some(path) != active.as_ref())
            .collect();

        let keep = self.advanced_u64("log_directories_to_keep").unwrap_or(10) as usize;
        let keep_others = keep.saturating_sub(1);
        if directories.len() <= keep_others {
            return;
        }
        directories.sort();
        let remove_count = directories.len() - keep_others;
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

    pub fn set_log_listener(&mut self, listener: Arc<dyn Fn(&str, &str, &str) + Send + Sync>) {
        self.log_listener = Some(listener);
    }

    pub fn clear_log_listener(&mut self) {
        self.log_listener = None;
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

        if let Some(listener) = &self.log_listener {
            listener(level, message, namespace);
        }

        let message = format!("{namespace}: {message}");
        self.winston_logs.push((level.to_string(), message.clone()));
        if let Some(syslog) = &self.syslog {
            syslog.send(level, &message);
        }
        let timestamp = self.format_timestamp();
        if let Some(file) = self.file_log.as_mut() {
            use std::io::Write as _;
            let _ = writeln!(file, "[{timestamp}] {level}: \t{message}");
        }

        let console_silent = self
            .transports
            .iter()
            .find(|transport| transport.kind == "Console")
            .map(|transport| transport.silent)
            .unwrap_or(false);

        if self.log_console_json {
            let line =
                json!({"level": level, "message": message, "timestamp": timestamp}).to_string();
            if !console_silent {
                println!("{line}");
            }
            self.console_writes.push(line);
        } else {
            let color = match level {
                "error" => 31,
                "warning" => 33,
                "debug" => 34,
                _ => 32,
            };
            if !console_silent {
                println!("[{timestamp}] {level}: \t{message}");
            }
            self.console_writes.push(format!(
                "[{timestamp}] \u{1b}[{color}m{level}\u{1b}[39m: \t{message}"
            ));
        }
    }

    /// Upstream: `logger.end()`.
    pub fn end(&mut self) {}
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

/// Winston/fecha-compatible timestamp formatting with local time
/// (`advanced.timestamp_format`, logger.ts:38-41).
pub fn format_timestamp(now_ms: i64, pattern: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ];
    const MONTHS_SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    const WEEKDAYS: [&str; 7] = [
        "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
    ];

    let local_ms = now_ms + local_offset_seconds(now_ms) * 1000;
    let days = local_ms.div_euclid(86_400_000);
    let rem = local_ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let seconds = rem / 1000;
    let (hour, minute, second) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    let millis = rem % 1000;
    let weekday = (days + 4).rem_euclid(7);
    let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    let offset = local_offset_seconds(now_ms);

    let tokens: [(&str, String); 25] = [
        ("YYYY", format!("{year:04}")),
        ("YY", format!("{:02}", year.rem_euclid(100))),
        ("MMMM", MONTHS[(month - 1) as usize].to_string()),
        ("MMM", MONTHS_SHORT[(month - 1) as usize].to_string()),
        ("MM", format!("{month:02}")),
        ("M", format!("{month}")),
        ("dddd", WEEKDAYS[weekday as usize].to_string()),
        ("ddd", WEEKDAYS[weekday as usize][..3].to_string()),
        ("DD", format!("{day:02}")),
        ("D", format!("{day}")),
        ("HH", format!("{hour:02}")),
        ("H", format!("{hour}")),
        ("hh", format!("{hour12:02}")),
        ("h", format!("{hour12}")),
        ("mm", format!("{minute:02}")),
        ("m", format!("{minute}")),
        ("ss", format!("{second:02}")),
        ("s", format!("{second}")),
        ("SSS", format!("{millis:03}")),
        ("A", if hour < 12 { "AM".into() } else { "PM".into() }),
        ("a", if hour < 12 { "am".into() } else { "pm".into() }),
        ("ZZ", format!("{}{:02}{:02}", if offset < 0 { "-" } else { "+" }, offset.abs() / 3600, (offset.abs() % 3600) / 60)),
        ("Z", format!("{}{:02}:{:02}", if offset < 0 { "-" } else { "+" }, offset.abs() / 3600, (offset.abs() % 3600) / 60)),
        ("X", format!("{}", local_ms / 1000)),
        ("x", format!("{local_ms}")),
    ];

    let mut output = String::new();
    let mut index = 0;
    while index < pattern.len() {
        let rest = &pattern[index..];
        if let Some((token, value)) = tokens.iter().find(|(token, _)| rest.starts_with(token)) {
            output.push_str(value);
            index += token.len();
        } else {
            let character = rest.chars().next().unwrap();
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
}
