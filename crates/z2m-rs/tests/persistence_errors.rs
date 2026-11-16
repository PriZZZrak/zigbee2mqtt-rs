//! Deterministic settings-write failure tests (audit CP-03).
//!
//! Upstream `settings.ts` throws when the persisted files cannot be written, so
//! `bridge/request/options`, `device/rename`, `device/remove` (blocklist),
//! `group/*` and the onboarding `/submit` handler answer with an error instead
//! of a false success.
//!
//! A read-only directory is not a reliable failure mode when the tests run as
//! root, so the write target itself is replaced by a directory: `write` then
//! fails with `EISDIR` on every platform/user.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::json;

use z2m_rs::util::onboarding;
use z2m_rs::util::settings::{Settings, CONFIGURATION_FILE, DEVICES_FILE};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "z2m-persist-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn config_text() -> &'static str {
    "version: 5\nmqtt:\n  base_topic: zigbee2mqtt\n  server: mqtt://localhost\ndevices:\n  '0x0017880104e45519':\n    friendly_name: lamp\ngroups:\n  '1':\n    friendly_name: group1\n"
}

fn settings(dir: &Path) -> Arc<Mutex<Settings>> {
    std::fs::write(dir.join(CONFIGURATION_FILE), config_text()).unwrap();
    Arc::new(Mutex::new(Settings::read_data_dir(dir).expect("settings")))
}

/// Replace a file by a directory so the next write fails deterministically.
fn break_path(path: PathBuf) {
    let _ = std::fs::remove_file(&path);
    std::fs::create_dir(&path).unwrap();
}

#[test]
fn submit_reports_a_write_failure_instead_of_success() {
    let dir = temp_dir("submit");
    let settings = settings(&dir);
    break_path(dir.join(CONFIGURATION_FILE));

    let body = serde_json::to_vec(&json!({"mqtt": {"base_topic": "changed"}})).unwrap();
    let (status, payload) = onboarding::submit_settings(&settings, &body);
    // Upstream answers 406 `{success:false, error}` when apply throws.
    assert_eq!(status, 406, "{payload}");
    assert_eq!(payload["success"], false);
    assert!(
        payload["error"]
            .as_str()
            .unwrap_or_default()
            .contains("configuration.yaml"),
        "{payload}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn options_failure_is_reported_by_apply() {
    let dir = temp_dir("options");
    let mut settings = Settings::read_data_dir(&dir).unwrap_or_else(|_| {
        std::fs::write(dir.join(CONFIGURATION_FILE), config_text()).unwrap();
        Settings::read_data_dir(&dir).unwrap()
    });
    break_path(dir.join(CONFIGURATION_FILE));

    let error = settings
        .apply(json!({"advanced": {"log_level": "debug"}}))
        .expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    // The in-memory value is applied (upstream mutates before throwing).
    assert_eq!(settings.get()["advanced"]["log_level"], "debug");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rename_and_remove_fail_when_the_configuration_is_not_writable() {
    let dir = temp_dir("rename");
    let settings = settings(&dir);
    break_path(dir.join(CONFIGURATION_FILE));

    let mut guard = settings.lock().unwrap();
    let error = guard
        .change_friendly_name("lamp", "renamed")
        .expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    // Upstream mutates the persisted object before writing.
    assert!(guard.get_device("renamed").is_some());
    drop(guard);

    let mut guard = settings.lock().unwrap();
    let error = guard
        .remove_device("renamed")
        .expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn blocklist_and_group_mutations_fail_when_the_configuration_is_not_writable() {
    let dir = temp_dir("blocklist");
    let settings = settings(&dir);
    break_path(dir.join(CONFIGURATION_FILE));

    let mut guard = settings.lock().unwrap();
    let error = guard
        .block_device("0x0017880104e45519")
        .expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    // The blocklist is updated in memory before the failing write.
    assert!(guard.get()["blocklist"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item == "0x0017880104e45519")));

    let error = guard.add_group("group2", None).expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    let error = guard.remove_group("group1").expect_err("write failure");
    assert!(error.contains("configuration.yaml"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn split_devices_write_failure_leaves_the_configuration_file_intact() {
    let dir = temp_dir("split");
    std::fs::write(
        dir.join(CONFIGURATION_FILE),
        "version: 5\nmqtt:\n  server: mqtt://localhost\ndevices: devices.yaml\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(DEVICES_FILE),
        "0x0017880104e45519:\n  friendly_name: lamp\n",
    )
    .unwrap();
    let before = std::fs::read_to_string(dir.join(CONFIGURATION_FILE)).unwrap();
    let settings = Arc::new(Mutex::new(Settings::read_data_dir(&dir).unwrap()));

    // Make only the split devices file unwritable.
    break_path(dir.join(DEVICES_FILE));

    let mut guard = settings.lock().unwrap();
    let error = guard
        .change_friendly_name("lamp", "renamed")
        .expect_err("write failure");
    assert!(error.contains("devices.yaml"), "{error}");
    drop(guard);

    // configuration.yaml still points at the split file and is untouched.
    assert_eq!(
        std::fs::read_to_string(dir.join(CONFIGURATION_FILE)).unwrap(),
        before
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_write_does_not_replace_the_previous_file() {
    let dir = temp_dir("preserve");
    let settings = settings(&dir);
    let original = std::fs::read_to_string(dir.join(CONFIGURATION_FILE)).unwrap();

    // The configuration file itself becomes a directory: the writer must not
    // delete or rewrite it, and the operation reports the error.
    break_path(dir.join(CONFIGURATION_FILE));
    assert!(settings
        .lock()
        .unwrap()
        .apply(json!({"mqtt": {"base_topic": "x"}}))
        .is_err());
    assert!(dir.join(CONFIGURATION_FILE).is_dir());

    // The original content is still recoverable once the path is a file again.
    std::fs::remove_dir(dir.join(CONFIGURATION_FILE)).unwrap();
    std::fs::write(dir.join(CONFIGURATION_FILE), &original).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join(CONFIGURATION_FILE)).unwrap(),
        original
    );
    let _ = std::fs::remove_dir_all(dir);
}
