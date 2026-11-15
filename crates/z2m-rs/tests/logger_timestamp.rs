// Original: .upstream/zigbee2mqtt/lib/util/logger.ts (timestamp_format, file transport)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::path::PathBuf;

use serde_json::json;
use z2m_rs::util::logger::{format_timestamp, Logger};

const FIXED_MS: i64 = 1_700_000_000_000; // 2023-11-14T22:13:20Z

#[test]
fn formats_winston_fecha_tokens() {
    // Pinned to UTC so the assertions are deterministic inside this process.
    std::env::set_var("TZ", "UTC");

    assert_eq!(
        format_timestamp(FIXED_MS, "YYYY-MM-DD HH:mm:ss"),
        "2023-11-14 22:13:20"
    );
    assert_eq!(
        format_timestamp(FIXED_MS, "YYYY-MM-DDTHH:mm:ss.SSSZ"),
        "2023-11-14T22:13:20.000+00:00"
    );
    assert_eq!(
        format_timestamp(FIXED_MS, "YYYY-MM-DDTHH:mm:ss.SSSZZ"),
        "2023-11-14T22:13:20.000+0000"
    );
    assert_eq!(
        format_timestamp(FIXED_MS, "dddd D MMMM YYYY, h:mm A"),
        "Tuesday 14 November 2023, 10:13 PM"
    );
    assert_eq!(
        format_timestamp(FIXED_MS, "YY MMM ddd hh:mm:ss a"),
        "23 Nov Tue 10:13:20 pm"
    );
    // Unknown characters are copied verbatim.
    assert_eq!(
        format_timestamp(FIXED_MS, "[YYYY] / x"),
        "[2023] / 1700000000000"
    );
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "z2m-logger-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn file_transport_writes_timestamped_lines() {
    let dir = temp_dir("file-transport");
    let mut logger = Logger::new();
    logger.advanced = json!({"advanced": {
        "log_output": ["file"],
        "log_directory": dir.display().to_string(),
        "log_file": "log.log",
        "timestamp_format": "SSS",
    }});
    logger.init();
    logger.info("hello", "z2m");
    logger.error("boom", "z2m");

    let contents = std::fs::read_to_string(dir.join("log.log")).expect("log file written");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 2, "{contents}");
    assert_eq!(&lines[0][5..], " info: \tz2m: hello", "{}", lines[0]);
    assert!(lines[0].starts_with('[') && lines[0][1..4].chars().all(|c| c.is_ascii_digit()));
    assert!(lines[1].contains("] error: \tz2m: boom"), "{}", lines[1]);

    // Console output keeps the upstream `[timestamp] level: \tmessage` shape.
    assert!(logger.console_writes[0].starts_with('['));
    assert!(logger.console_writes[0].contains("info"));

    let _ = std::fs::remove_dir_all(dir);
}
