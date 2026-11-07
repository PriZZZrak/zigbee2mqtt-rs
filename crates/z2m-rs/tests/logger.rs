// Port of .upstream/zigbee2mqtt/test/logger.test.ts
//
// Upstream uses winston/node streams and spies on console/fs. The Rust port
// records transports, winston calls and console writes on `Logger`, so the
// same observables can be asserted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{json, Value};
use z2m_rs::util::logger::{Logger, Transport};

const LOG_LEVELS: [&str; 4] = ["debug", "info", "warning", "error"];

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "z2m-logger-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::SeqCst)
    ))
}

fn settings(advanced: Value) -> Value {
    json!({"advanced": advanced})
}

fn setup() -> (PathBuf, Logger) {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let (dir, mut logger) = (dir.clone(), Logger::new());
    logger.advanced = settings(json!({"log_directory": format!("{}/%TIMESTAMP%", dir.display())}));
    logger.init();
    (dir, logger)
}

fn log_at(logger: &mut Logger, level: &str, message: &str, namespace: &str) {
    match level {
        "debug" => logger.debug(message, namespace),
        "info" => logger.info(message, namespace),
        "warning" => logger.warning(message, namespace),
        "error" => logger.error(message, namespace),
        other => panic!("unknown level {other}"),
    }
}

fn is_timestamp_dir_name(name: &str) -> bool {
    name.len() == 19
        && name.as_bytes()[4] == b'-'
        && name.as_bytes()[7] == b'-'
        && name.as_bytes()[10] == b'.'
        && name.as_bytes()[13] == b'-'
        && name.as_bytes()[16] == b'-'
}

fn create_numbered_dirs(dir: &Path, range: std::ops::Range<usize>) {
    for i in range {
        std::fs::create_dir_all(dir.join(format!("log_{i}"))).unwrap();
    }
}

fn clear_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[test]
fn create_log_directory() {
    let (dir, _logger) = setup();
    let dirs: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();

    assert_eq!(dirs.len(), 1);
    assert!(is_timestamp_dir_name(&dirs[0]), "invalid dir name {dirs:?}");
}

#[test]
fn should_cleanup_default_setting() {
    let (dir, mut logger) = setup();
    clear_dir(&dir);
    create_numbered_dirs(&dir, 0..20);

    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 20);
    logger.init();
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 10);
}

#[test]
fn should_handle_cleanup_error() {
    let (dir, mut logger) = setup();
    clear_dir(&dir);
    create_numbered_dirs(&dir, 0..20);

    logger.remove_error = Some("EACCES: permission denied".to_string());
    logger.winston_logs.clear();
    logger.init();

    let (level, message) = logger
        .winston_logs
        .last()
        .cloned()
        .expect("logger.error must be called");
    assert_eq!(level, "error");
    assert!(message.contains("Failed to remove old log directory"));
    assert!(message.contains("EACCES: permission denied"));
}

#[test]
fn should_cleanup_15_folders_setting() {
    let (dir, mut logger) = setup();
    clear_dir(&dir);
    create_numbered_dirs(&dir, 0..20);
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_directories_to_keep": 15,
    }));

    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 20);
    logger.init();
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 15);
}

#[test]
fn should_not_cleanup_when_there_is_no_timestamp_set() {
    let (dir, mut logger) = setup();
    clear_dir(&dir);
    create_numbered_dirs(&dir, 30..50);
    logger.advanced = settings(json!({"log_directory": format!("{}/bla", dir.display())}));

    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 20);
    logger.init();
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 21);
}

#[test]
fn set_and_get_log_level() {
    let mut logger = Logger::new();

    logger.set_level("debug");
    assert_eq!(logger.get_level(), "debug");
    logger.set_level("info");
    assert_eq!(logger.get_level(), "info");
    logger.set_level("warning");
    assert_eq!(logger.get_level(), "warning");
    logger.set_level("error");
    assert_eq!(logger.get_level(), "error");

    // winston level always stays at 'debug', logic handled by custom logger
    assert_eq!(logger.winston_level(), "debug");
    for transport in &logger.transports {
        assert!(transport.max_files.is_none() || transport.kind == "File");
    }
}

#[test]
fn add_remove_transport() {
    let (_dir, mut logger) = setup();

    assert_eq!(logger.transports.len(), 2);
    logger.add_transport(Transport {
        kind: "DummyTransport".to_string(),
        ..Default::default()
    });
    assert_eq!(logger.transports.len(), 3);
    logger.remove_transport("DummyTransport");
    assert_eq!(logger.transports.len(), 2);
}

#[test]
fn logger_should_be_console_and_file_by_default() {
    let (dir, logger) = setup();
    let pipes = &logger.transports;

    assert_eq!(pipes.len(), 2);
    assert_eq!(pipes[0].kind, "Console");
    assert!(!pipes[0].silent);
    assert_eq!(pipes[1].kind, "File");
    assert!(pipes[1]
        .dirname
        .as_deref()
        .is_some_and(|dirname| dirname.starts_with(&*dir.to_string_lossy())));
}

#[test]
fn logger_can_be_file_only() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_output": ["file"],
    }));
    logger.init();

    let pipes = &logger.transports;
    assert_eq!(pipes.len(), 2);
    assert_eq!(pipes[0].kind, "Console");
    assert!(pipes[0].silent);
    assert_eq!(pipes[1].kind, "File");
    assert!(pipes[1]
        .dirname
        .as_deref()
        .is_some_and(|dirname| dirname.starts_with(&*dir.to_string_lossy())));
}

#[test]
fn logger_can_be_console_only() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_output": ["console"],
    }));
    logger.init();

    assert_eq!(logger.transports.len(), 1);
    assert_eq!(logger.transports[0].kind, "Console");
    assert!(!logger.transports[0].silent);
}

#[test]
fn logger_can_be_nothing() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_output": [],
    }));
    logger.init();

    assert_eq!(logger.transports.len(), 1);
    assert_eq!(logger.transports[0].kind, "Console");
    assert!(logger.transports[0].silent);
}

#[test]
fn should_allow_to_disable_log_rotation() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_rotation": false,
    }));
    logger.init();

    let file = &logger.transports[1];
    assert_eq!(file.kind, "File");
    assert_eq!(file.max_files, None);
    assert!(!file.tailable);
    assert_eq!(file.maxsize, None);
}

#[test]
fn should_allow_to_symlink_logs_to_current_directory() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_symlink_current": true,
    }));
    logger.init();

    let entries: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(entries.contains(&"current".to_string()));
}

// Upstream: it.each([...])("Logs relevant levels for %s", ...)
#[test]
fn logs_relevant_levels_for() {
    let cases: [(&str, &[&str], &[&str]); 4] = [
        ("debug", &["info", "warning", "error"], &[]),
        ("info", &["warning", "error"], &["debug"]),
        ("warning", &["error"], &["debug", "info"]),
        ("error", &[], &["debug", "info", "warning"]),
    ];

    for (level, higher, lower) in cases {
        let (_dir, mut logger) = setup();
        logger.set_level(level);
        logger.winston_logs.clear();
        logger.console_writes.clear();

        let mut calls = 0;
        log_at(&mut logger, level, "msg", "z2m");
        calls += 1;
        assert_eq!(
            logger.winston_logs.last(),
            Some(&(level.to_string(), "z2m: msg".to_string()))
        );
        assert_eq!(logger.console_writes.len(), calls);

        log_at(&mut logger, level, "msg", "abcd");
        calls += 1;
        assert_eq!(
            logger.winston_logs.last(),
            Some(&(level.to_string(), "abcd: msg".to_string()))
        );
        assert_eq!(logger.console_writes.len(), calls);

        // Upstream also passes a lazily evaluated function message.
        log_at(&mut logger, level, "func msg", "abcd");
        calls += 1;
        assert_eq!(
            logger.winston_logs.last(),
            Some(&(level.to_string(), "abcd: func msg".to_string()))
        );
        assert_eq!(logger.console_writes.len(), calls);

        for higher_level in higher {
            log_at(&mut logger, higher_level, "higher msg", "z2m");
            calls += 1;
            assert_eq!(
                logger.winston_logs.last(),
                Some(&((*higher_level).to_string(), "z2m: higher msg".to_string()))
            );
            assert_eq!(logger.console_writes.len(), calls);

            log_at(&mut logger, higher_level, "higher msg", "abcd");
            calls += 1;
            assert_eq!(
                logger.winston_logs.last(),
                Some(&((*higher_level).to_string(), "abcd: higher msg".to_string()))
            );
            assert_eq!(logger.console_writes.len(), calls);
        }

        logger.winston_logs.clear();
        logger.console_writes.clear();

        for lower_level in lower {
            log_at(&mut logger, lower_level, "lower msg", "z2m");
            assert!(logger.winston_logs.is_empty());
            assert!(logger.console_writes.is_empty());

            log_at(&mut logger, lower_level, "lower msg", "abcd");
            assert!(logger.winston_logs.is_empty());
            assert!(logger.console_writes.is_empty());
        }
    }
}

// Upstream: it.each([...])("Sets namespace ignore for debug level %s", ...)
#[test]
fn sets_namespace_ignore_for_debug_level() {
    let cases: [(&str, &[(&str, bool)]); 3] = [
        (
            "^zhc:legacy:fz:(tuya|moes)",
            &[
                ("zhc:legacy:fz:tuya_device12", true),
                ("zhc:legacy:fz:moes_dimmer", true),
                ("zhc:legacy:fz:not_moes", false),
                ("zhc:legacy:fz", false),
                ("zhc:legacy:fz:", false),
                ("1zhc:legacy:fz:tuya_device12", false),
            ],
        ),
        (
            "^zhc:legacy:fz:(tuya|moes)|^zh:ember:uart:|^zh:controller",
            &[
                ("zh:ember:uart:ash", true),
                ("zh:ember:uart", false),
                ("zh:controller", true),
                ("zh:controller:", true),
                ("azh:controller:", false),
            ],
        ),
        (
            "",
            &[
                ("zhc:legacy:fz:tuya_device12", false),
                ("zhc:legacy:fz:moes_dimmer", false),
                ("zhc:legacy:fz:not_moes", false),
                ("zhc:legacy:fz", false),
                ("zhc:legacy:fz:", false),
                ("1zhc:legacy:fz:tuya_device12", false),
                ("zh:ember:uart:ash", false),
                ("zh:ember:uart", false),
                ("zh:controller", false),
                ("zh:controller:", false),
                ("azh:controller:", false),
            ],
        ),
    ];

    for (ignore, tests) in cases {
        let (_dir, mut logger) = setup();
        logger.set_level("debug");
        logger.winston_logs.clear();
        logger.set_debug_namespace_ignore(ignore);

        assert_eq!(logger.get_debug_namespace_ignore(), ignore);
        assert_eq!(
            logger.debug_namespace_ignore_regex.is_some(),
            !ignore.is_empty()
        );

        for (namespace, matches) in tests {
            logger.debug("Test message", namespace);

            if *matches {
                assert!(logger.winston_logs.is_empty(), "ns={namespace}");
            } else {
                assert_eq!(
                    logger.winston_logs.last(),
                    Some(&("debug".to_string(), format!("{namespace}: Test message"))),
                    "ns={namespace}"
                );
            }

            logger.winston_logs.clear();
        }
    }
}

#[test]
fn logs_with_namespaced_levels_or_default_higher() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_namespaced_levels": {"z2m:mqtt": "warning"},
    }));
    logger.init();
    logger.set_level("debug");

    assert_eq!(
        logger.get_namespaced_levels(),
        BTreeMap::from([("z2m:mqtt".to_string(), "warning".to_string())])
    );
    assert_eq!(logger.get_level(), "debug");

    logger.winston_logs.clear();
    logger.console_writes.clear();
    logger.info(
        "MQTT publish: topic 'abcd/efgh', payload '{\"my\": {\"payload\": \"injson\"}}'",
        "z2m:mqtt",
    );
    assert!(logger.winston_logs.is_empty());
    assert!(logger.console_writes.is_empty());

    logger.error("Not connected to MQTT server!", "z2m:mqtt");
    assert_eq!(logger.winston_logs.len(), 1);
    assert_eq!(logger.console_writes.len(), 1);

    logger.info("Just another info message", "z2m:notmqtt");
    assert_eq!(logger.winston_logs.len(), 2);
    assert_eq!(logger.console_writes.len(), 2);
}

#[test]
fn logs_with_namespaced_levels_or_default_lower() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({"log_directory": format!("{}/%TIMESTAMP%", dir.display())}));

    assert_eq!(logger.get_namespaced_levels(), BTreeMap::new());
    logger.set_namespaced_levels(BTreeMap::from([(
        "z2m:mqtt".to_string(),
        "info".to_string(),
    )]));
    logger.set_level("warning");
    assert_eq!(
        logger.get_namespaced_levels(),
        BTreeMap::from([("z2m:mqtt".to_string(), "info".to_string())])
    );
    assert_eq!(logger.get_level(), "warning");

    logger.winston_logs.clear();
    logger.console_writes.clear();
    logger.info(
        "MQTT publish: topic 'abcd/efgh', payload '{\"my\": {\"payload\": \"injson\"}}'",
        "z2m:mqtt",
    );
    assert_eq!(logger.winston_logs.len(), 1);
    assert_eq!(logger.console_writes.len(), 1);

    logger.error("Not connected to MQTT server!", "z2m:mqtt");
    assert_eq!(logger.winston_logs.len(), 2);
    assert_eq!(logger.console_writes.len(), 2);

    logger.info("Just another info message", "z2m:notmqtt");
    assert_eq!(logger.winston_logs.len(), 2);
    assert_eq!(logger.console_writes.len(), 2);

    logger.warning("Just another warning message", "z2m:notmqtt");
    assert_eq!(logger.winston_logs.len(), 3);
    assert_eq!(logger.console_writes.len(), 3);
}

#[test]
fn logs_with_namespaced_levels_hierarchy() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({"log_directory": format!("{}/%TIMESTAMP%", dir.display())}));

    let ns_levels = BTreeMap::from([
        ("zh:zstack".to_string(), "debug".to_string()),
        ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
    ]);
    logger.set_namespaced_levels(ns_levels.clone());
    logger.set_level("warning");

    logger.console_writes.clear();
    logger.debug(
        "--- parseNext [] debug picked from hierarchy",
        "zh:zstack:unpi:parser",
    );
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "debug".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
            ("zh:zstack:unpi:parser".to_string(), "debug".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 1);

    logger.warning(
        "--> frame [36,15] warning explicitely supressed",
        "zh:zstack:unpi:writer",
    );
    assert_eq!(logger.get_cached_namespaced_levels(), ns_levels);
    assert_eq!(logger.console_writes.len(), 1);

    logger.warning(
        "Another supressed warning message in a sub namespace",
        "zh:zstack:unpi:writer:sub:ns",
    );
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "debug".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
            (
                "zh:zstack:unpi:writer:sub:ns".to_string(),
                "error".to_string()
            ),
        ])
    );
    assert_eq!(logger.console_writes.len(), 1);

    logger.error(
        "but error should go through",
        "zh:zstack:unpi:writer:another:sub:ns",
    );
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "debug".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
            (
                "zh:zstack:unpi:writer:another:sub:ns".to_string(),
                "error".to_string()
            ),
        ])
    );
    assert_eq!(logger.console_writes.len(), 2);

    logger.warning("new unconfigured namespace warning", "z2m:mqtt");
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "debug".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
            ("z2m:mqtt".to_string(), "warning".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 3);

    logger.info(
        "cached unconfigured namespace info should be supressed",
        "z2m:mqtt",
    );
    assert_eq!(logger.get_cached_namespaced_levels(), ns_levels);
    assert_eq!(logger.console_writes.len(), 3);

    logger.set_level("info");
    assert_eq!(logger.get_cached_namespaced_levels(), ns_levels);
    logger.info(
        "unconfigured namespace info should now pass after default level change and cache reset",
        "z2m:mqtt",
    );
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "debug".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "error".to_string()),
            ("z2m:mqtt".to_string(), "info".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 4);

    logger.error(
        "configured namespace hierachy should still work after the cache reset",
        "zh:zstack:unpi:writer:another:sub:ns",
    );
    assert_eq!(logger.console_writes.len(), 5);

    logger.set_namespaced_levels(BTreeMap::from([(
        "zh:zstack".to_string(),
        "warning".to_string(),
    )]));
    let cached_ns_levels = BTreeMap::from([("zh:zstack".to_string(), "warning".to_string())]);
    assert_eq!(logger.get_cached_namespaced_levels(), cached_ns_levels);

    logger.error("error logged", "zh:zstack:unpi:writer");
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "warning".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "warning".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 6);

    logger.debug("debug suppressed", "zh:zstack:unpi");
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "warning".to_string()),
            ("zh:zstack:unpi:writer".to_string(), "warning".to_string()),
            ("zh:zstack:unpi".to_string(), "warning".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 6);

    logger.warning("warning logged", "zh:zstack");
    assert_eq!(logger.get_cached_namespaced_levels(), cached_ns_levels);
    assert_eq!(logger.console_writes.len(), 7);

    logger.info("unconfigured namespace", "z2m:mqtt");
    assert_eq!(
        logger.get_cached_namespaced_levels(),
        BTreeMap::from([
            ("zh:zstack".to_string(), "warning".to_string()),
            ("z2m:mqtt".to_string(), "info".to_string()),
        ])
    );
    assert_eq!(logger.console_writes.len(), 8);
}

#[test]
fn ignores_splat_chars() {
    let (_dir, mut logger) = setup();
    logger.set_level("debug");
    logger.winston_logs.clear();
    logger.console_writes.clear();

    let splat_chars = "%d";
    logger.debug(splat_chars, "z2m:mqtt");
    assert_eq!(
        logger.winston_logs.last(),
        Some(&("debug".to_string(), format!("z2m:mqtt: {splat_chars}")))
    );
    assert!(logger.console_writes[0].ends_with(&format!("z2m:mqtt: {splat_chars}")));

    let splat_chars = "anything %s goes here";
    logger.debug(splat_chars, "z2m:test");
    assert_eq!(
        logger.winston_logs.last(),
        Some(&("debug".to_string(), format!("z2m:test: {splat_chars}")))
    );
    assert!(logger.console_writes[1].ends_with(&format!("z2m:test: {splat_chars}")));
}

#[test]
fn logs_to_console_in_json_when_configured() {
    let (dir, mut logger) = setup();
    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_console_json": true,
    }));
    logger.init();

    logger.console_writes.clear();
    logger.info("Test JSON message", "z2m");

    let output_json: Value = serde_json::from_str(&logger.console_writes[0]).unwrap();
    assert_eq!(output_json["level"], json!("info"));
    assert_eq!(output_json["message"], json!("z2m: Test JSON message"));
    assert!(output_json["timestamp"].is_string());

    logger.advanced = settings(json!({
        "log_directory": format!("{}/%TIMESTAMP%", dir.display()),
        "log_console_json": false,
    }));
    logger.init();

    logger.console_writes.clear();
    logger.info("Test JSON message", "z2m");

    assert!(logger.console_writes[0]
        .trim_end()
        .ends_with("\u{1b}[32minfo\u{1b}[39m: \tz2m: Test JSON message"));
}
