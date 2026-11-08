// Port of .upstream/zigbee2mqtt/test/data.test.ts

use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use z2m_rs::util::data;

// `data::get_path` caches process-wide state and the tests below mutate
// `ZIGBEE2MQTT_DATA`, so serialize them (upstream runs each test in an
// isolated environment).
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn should_return_correct_path() {
    let _guard = ENV_LOCK.lock().unwrap();
    env::remove_var("ZIGBEE2MQTT_DATA");
    data::_test_reload();
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let actual = data::get_path();
    assert_eq!(actual, expected);
}

#[test]
fn should_return_correct_path_when_zigbee2mqtt_data_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    let expected = std::env::temp_dir().join(format!("z2m-data-test-{}", std::process::id()));
    env::set_var("ZIGBEE2MQTT_DATA", &expected);
    data::_test_reload();

    let actual = data::get_path();
    assert_eq!(actual, expected);
    assert_eq!(data::join_path("test"), expected.join("test"));
    assert_eq!(data::join_path("/test"), PathBuf::from("/test"));

    env::remove_var("ZIGBEE2MQTT_DATA");
    data::_test_reload();
}

#[test]
fn should_return_data_path_when_join_path_with_empty_string() {
    let _guard = ENV_LOCK.lock().unwrap();
    let expected = data::get_path();
    assert_eq!(data::join_path(""), expected);
}
