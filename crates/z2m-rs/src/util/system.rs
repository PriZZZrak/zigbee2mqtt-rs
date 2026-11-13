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
