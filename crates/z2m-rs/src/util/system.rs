// Original: Node.js `node:os` / `node:process` usage in
// `.upstream/zigbee2mqtt/lib/extension/health.ts`.
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Minimal Linux equivalents used by the health extension.

use std::sync::OnceLock;
use std::time::Instant;

fn process_start() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

/// `os.loadavg()` (`/proc/loadavg`).
pub fn load_average() -> [f64; 3] {
    let content = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut values = content
        .split_whitespace()
        .take(3)
        .map(|value| value.parse::<f64>().unwrap_or(0.0));
    [
        values.next().unwrap_or(0.0),
        values.next().unwrap_or(0.0),
        values.next().unwrap_or(0.0),
    ]
}

/// `os.totalmem()` / `os.freemem()` in kB.
pub fn memory_kb() -> (f64, f64) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let value = |name: &str| -> f64 {
        content
            .lines()
            .find_map(|line| {
                let mut parts = line.split_whitespace();
                (parts.next() == Some(&format!("{name}:")))
                    .then(|| parts.next()?.parse::<f64>().ok())
                    .flatten()
            })
            .unwrap_or(0.0)
    };
    (value("MemTotal"), value("MemAvailable"))
}

/// `process.uptime()` in seconds.
pub fn process_uptime_secs() -> u64 {
    process_start().elapsed().as_secs()
}

/// `process.memoryUsage().rss` in kB (`/proc/self/statm`, page 2).
pub fn process_rss_kb() -> f64 {
    let content = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages = content
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    pages * 4096.0 / 1024.0
}

/// `os.version() - os.release() - os.arch()` for `bridge/info.os.version`.
pub fn os_version() -> String {
    let version = std::fs::read_to_string("/proc/version")
        .ok()
        .and_then(|content| content.split_whitespace().nth(2).map(str::to_string))
        .unwrap_or_default();
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|content| content.trim().to_string())
        .unwrap_or_default();
    format!("{version} - {release} - {}", std::env::consts::ARCH)
}

/// Unique CPU models and count (`os.cpus()` summary).
pub fn cpu_summary() -> (String, usize) {
    let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut models: Vec<String> = Vec::new();
    let mut count = 0usize;
    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if key == "processor" {
                count += 1;
            } else if key == "model name" && !models.iter().any(|model| model == value) {
                models.push(value.to_string());
            }
        }
    }
    let count = if count == 0 { 1 } else { count };
    (models.join(" | "), count)
}

/// `Math.round(os.totalmem() / 1024 / 1024)`.
pub fn total_memory_mb() -> u64 {
    (memory_kb().0 / 1024.0).round() as u64
}
