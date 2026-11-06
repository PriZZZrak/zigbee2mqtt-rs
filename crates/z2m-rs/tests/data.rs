// Port of .upstream/zigbee2mqtt/test/data.test.ts

use std::env;
use std::path::PathBuf;
use z2m_rs::util::data;

#[test]
#[ignore = "not implemented yet"]
fn should_return_correct_path() {
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let actual = data::get_path();
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_correct_path_when_zigbee2mqtt_data_set() {
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
#[ignore = "not implemented yet"]
fn should_return_data_path_when_join_path_with_empty_string() {
    let expected = data::get_path();
    assert_eq!(data::join_path(""), expected);
}
