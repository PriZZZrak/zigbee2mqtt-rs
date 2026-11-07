// Original: .upstream/zigbee2mqtt/test/extensions/configure.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Extension: Configure
//   - Fail to configure via MQTT when configure fails
//   - Fail to configure via MQTT when device does not exist
//   - Fail to configure via MQTT when device has no configure
//   - Handles invalid payload for configure via MQTT
//   - Should NOT re-configure when version of the definition is 0.0.0
//   - Should allow to configure via MQTT
//   - Should configure Router on startup
//   - Should configure max 3 times when fails
//   - Should configure on zigbee message when not configured yet
//   - Should not configure EndDevice on startup
//   - Should not configure twice
//   - Should not configure when already configuring
//   - Should not configure when interview not completed
//   - Should not re-configure disabled devices
//   - Should re-configure when device rejoins
//   - Should re-configure when the version of the definition changes to a different value than the default (0.0.0)
//   - Should reconfigure reporting on reconfigure event
//   - successfully configures after couple fails
//   - successfully configures with force after fails

use serde_json::json;
use z2m_rs::extension::configure::{Configure, Endpoint};

fn setup() -> Configure {
    let mut configure = Configure::new();
    configure.coordinator_ieee = "0x00124b0000000000".to_string();
    configure.add_device("bulb", "0x000b57fffec6a5b2", "Router", &[1]);
    configure.add_device("remote", "0x0017880104e45517", "EndDevice", &[1, 2]);
    configure.add_device("coordinator", "0x00124b0000000000", "Coordinator", &[1]);
    configure.add_device("0x0017980134e45545", "0x0017980134e45545", "Router", &[1]);
    configure.device_mut("0x0017980134e45545").has_configure = false;
    configure
}

fn endpoint<'a>(configure: &'a Configure, device: &str, index: u8) -> &'a Endpoint {
    &configure.devices[device].endpoints[&index]
}

fn endpoint_mut<'a>(configure: &'a mut Configure, device: &str, index: u8) -> &'a mut Endpoint {
    configure
        .devices
        .get_mut(device)
        .unwrap()
        .endpoints
        .get_mut(&index)
        .unwrap()
}

fn clear(configure: &mut Configure, device: &str) {
    for endpoint in configure
        .devices
        .get_mut(device)
        .unwrap()
        .endpoints
        .values_mut()
    {
        *endpoint = Endpoint::default();
    }
}

fn expect_bulb_configured(configure: &Configure) {
    let endpoint = endpoint(configure, "bulb", 1);
    assert_eq!(endpoint.reads.len(), 2);
    assert!(endpoint.reads.contains(&(
        "lightingColorCtrl".to_string(),
        vec!["colorCapabilities".to_string()]
    )));
    assert!(endpoint.reads.contains(&(
        "lightingColorCtrl".to_string(),
        vec![
            "colorTempPhysicalMin".to_string(),
            "colorTempPhysicalMax".to_string()
        ]
    )));
}

fn expect_bulb_not_configured(configure: &Configure) {
    assert_eq!(endpoint(configure, "bulb", 1).reads.len(), 0);
}

fn expect_remote_not_configured(configure: &Configure) {
    assert_eq!(endpoint(configure, "remote", 1).binds.len(), 0);
}

fn expect_remote_configured(configure: &Configure) {
    let endpoint1 = endpoint(configure, "remote", 1);
    assert_eq!(endpoint1.binds.len(), 2);
    assert!(endpoint1
        .binds
        .contains(&("genOnOff".to_string(), configure.coordinator_ieee.clone())));
    assert!(endpoint1.binds.contains(&(
        "genLevelCtrl".to_string(),
        configure.coordinator_ieee.clone()
    )));

    let endpoint2 = endpoint(configure, "remote", 2);
    assert_eq!(endpoint2.writes.len(), 1);
    assert!(endpoint2.writes.contains(&(
        "genBasic".to_string(),
        json!({"49": {"type": 25, "value": 11}}),
        json!({"disableDefaultResponse": true, "manufacturerCode": 4107})
    )));

    assert_eq!(
        configure.devices["remote"].meta_configured.as_deref(),
        Some("0.0.0")
    );
}

fn response_ok(configure: &Configure, id: &str) -> bool {
    configure.published.contains(&(
        "zigbee2mqtt/bridge/response/device/configure".to_string(),
        json!({"data": {"id": id}, "status": "ok"}),
        json!({}),
    ))
}

fn response_error(configure: &Configure, payload: serde_json::Value) -> bool {
    configure.published.contains(&(
        "zigbee2mqtt/bridge/response/device/configure".to_string(),
        payload,
        json!({}),
    ))
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn fail_to_configure_via_mqtt_when_configure_fails() {
    let mut configure = setup();
    configure.device_mut("remote").configure_error = Some("Bind timeout after 10s".to_string());

    configure.on_mqtt_configure(&json!({"id": "remote"}));

    assert!(response_error(
        &configure,
        json!({
            "data": {},
            "status": "error",
            "error": "Failed to configure (Bind timeout after 10s)",
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn fail_to_configure_via_mqtt_when_device_does_not_exist() {
    let mut configure = setup();
    configure.on_mqtt_configure(&json!({"id": "not_existing_device"}));

    assert!(response_error(
        &configure,
        json!({
            "data": {},
            "status": "error",
            "error": "Device 'not_existing_device' does not exist",
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn fail_to_configure_via_mqtt_when_device_has_no_configure() {
    let mut configure = setup();
    configure.on_mqtt_configure(&json!({"id": "0x0017980134e45545", "transaction": 20}));

    assert!(response_error(
        &configure,
        json!({
            "data": {},
            "status": "error",
            "error": "Device '0x0017980134e45545' cannot be configured",
            "transaction": 20,
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handles_invalid_payload_for_configure_via_mqtt() {
    let mut configure = setup();
    configure.on_mqtt_configure(&json!({"idx": "0x0017882104a44559"}));

    assert!(response_error(
        &configure,
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_re_configure_when_version_of_the_definition_is_0_0_0() {
    let mut configure = setup();
    configure.start();
    expect_bulb_configured(&configure);
    configure.device_mut("bulb").meta_configured = Some("1321".to_string());

    // Nothing happens when receiving a Zigbee message
    clear(&mut configure, "bulb");
    configure.on_last_seen_changed("bulb");
    expect_bulb_not_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_configure_via_mqtt() {
    let mut configure = setup();
    configure.start();
    configure.published.clear();
    configure.devices_changed = 0;
    configure.exposes_changed.clear();

    configure.on_mqtt_configure(&json!("remote"));

    expect_remote_configured(&configure);
    assert_eq!(configure.devices_changed, 1);
    assert!(configure
        .exposes_changed
        .iter()
        .any(|device| device == "remote"));
    assert!(response_ok(&configure, "remote"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_configure_router_on_startup() {
    let mut configure = setup();
    configure.start();

    expect_bulb_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_configure_max_3_times_when_fails() {
    let mut configure = setup();
    configure.device_mut("remote").meta_configured = None;
    configure.device_mut("remote").configure_error = Some("BLA".to_string());
    clear(&mut configure, "remote");

    for _ in 0..5 {
        configure.on_last_seen_changed("remote");
    }

    assert_eq!(endpoint(&configure, "remote", 1).binds.len(), 3);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_configure_on_zigbee_message_when_not_configured_yet() {
    let mut configure = setup();
    configure.device_mut("bulb").meta_configured = None;
    clear(&mut configure, "bulb");

    configure.on_last_seen_changed("bulb");

    expect_bulb_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_configure_end_device_on_startup() {
    let mut configure = setup();
    configure.start();

    expect_remote_not_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_configure_twice() {
    let mut configure = setup();
    configure.start();
    expect_bulb_configured(&configure);

    clear(&mut configure, "bulb");
    configure.on_device_interview("bulb");

    expect_bulb_not_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_configure_when_already_configuring() {
    let mut configure = setup();
    configure.device_mut("remote").meta_configured = None;
    clear(&mut configure, "remote");
    configure.configuring.insert("remote".to_string());

    configure.on_last_seen_changed("remote");
    configure.on_last_seen_changed("remote");

    assert_eq!(endpoint(&configure, "remote", 1).binds.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_configure_when_interview_not_completed() {
    let mut configure = setup();
    configure.device_mut("remote").meta_configured = None;
    configure.device_mut("remote").interview_state = "Pending".to_string();
    clear(&mut configure, "remote");

    configure.on_last_seen_changed("remote");

    expect_remote_not_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_re_configure_disabled_devices() {
    let mut configure = setup();
    configure.start();
    expect_bulb_configured(&configure);

    clear(&mut configure, "bulb");
    configure.disable("bulb");
    configure.on_device_joined("bulb");

    expect_bulb_not_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_re_configure_when_device_rejoins() {
    let mut configure = setup();
    configure.start();
    expect_bulb_configured(&configure);

    clear(&mut configure, "bulb");
    configure.on_device_joined("bulb");

    expect_bulb_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_re_configure_when_the_version_of_the_definition_changes_to_a_different_value_than_the_default_0_0_0(
) {
    let mut configure = setup();
    configure.start();
    // Device is initially configured (definition uses default version 0.0.0)
    expect_bulb_configured(&configure);

    // Nothing happens when receiving a Zigbee message
    clear(&mut configure, "bulb");
    configure.on_last_seen_changed("bulb");
    expect_bulb_not_configured(&configure);

    // Simulate that the definition version changes
    assert_eq!(configure.devices["bulb"].definition_version, "0.0.0");
    configure.device_mut("bulb").definition_version = "0.0.1".to_string();

    // Now it should re-configure upon receiving a Zigbee message
    clear(&mut configure, "bulb");
    configure.on_last_seen_changed("bulb");
    expect_bulb_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_reconfigure_reporting_on_reconfigure_event() {
    let mut configure = setup();
    configure.start();
    expect_bulb_configured(&configure);

    clear(&mut configure, "bulb");
    expect_bulb_not_configured(&configure);

    configure.on_reconfigure("bulb");

    expect_bulb_configured(&configure);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn successfully_configures_after_couple_fails() {
    let mut configure = setup();
    configure.device_mut("remote").meta_configured = None;
    clear(&mut configure, "remote");

    configure.device_mut("remote").configure_error = Some("BLA1".to_string());
    configure.on_last_seen_changed("remote");
    configure.device_mut("remote").configure_error = Some("BLA2".to_string());
    configure.on_last_seen_changed("remote");
    configure.device_mut("remote").configure_error = None;
    configure.on_last_seen_changed("remote");

    assert_eq!(endpoint(&configure, "remote", 1).binds.len(), 4);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn successfully_configures_with_force_after_fails() {
    let mut configure = setup();
    configure.device_mut("remote").meta_configured = None;
    clear(&mut configure, "remote");

    configure.device_mut("remote").configure_error = Some("BLA1".to_string());
    configure.on_last_seen_changed("remote");
    configure.device_mut("remote").configure_error = Some("BLA2".to_string());
    configure.on_last_seen_changed("remote");
    configure.device_mut("remote").configure_error = Some("BLA3".to_string());
    configure.on_last_seen_changed("remote");
    configure.on_last_seen_changed("remote");
    configure.on_last_seen_changed("remote");

    assert_eq!(endpoint(&configure, "remote", 1).binds.len(), 3);

    let ieee = configure.devices["remote"].ieee_address.clone();
    configure.device_mut("remote").configure_error = Some("BLA4".to_string());
    configure.on_mqtt_configure(&json!({"id": ieee}));
    configure.on_mqtt_configure(&json!({"id": ieee}));

    assert_eq!(endpoint(&configure, "remote", 1).binds.len(), 6);
}
