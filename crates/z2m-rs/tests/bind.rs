// Original: .upstream/zigbee2mqtt/test/extensions/bind.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Error bind fails when source device
//   - Error bind fails when source device does not exist
//   - Error bind fails when target device
//   - Error bind fails when target device or group does not exist
//   - Extension: Bind
//   - Filters out reporting setup based on bind status
//   - Filters out unsupported clusters for reporting setup
//   - Should allow to bind to coordinator by ieeeAddr
//   - Should bind from non default endpoint IDs
//   - Should bind from non default endpoint names
//   - Should bind only specified clusters
//   - Should bind server clusters to client clusters
//   - Should bind to default endpoint returned by endpoints()
//   - Should bind to device and configure reporting
//   - Should bind to group by number
//   - Should bind to groups
//   - Should log error when there is nothing to bind
//   - Should log when bind fails
//   - Should poll bounded Hue bulb when receiving message from Hue dimmer
//   - Should poll bounded Hue bulb when receiving message from scene controller
//   - Should poll grouped Hue bulb when receiving message from TRADFRI remote
//   - Should throw error on invalid payload
//   - Should unbind
//   - Should unbind coordinator
//   - Should unbind from default_bind_group
//   - Should unbind from group
//   - Should unbind from group with skip_disable_reporting=false
//   - Should unbind from group with skip_disable_reporting=true
//   - clears all bindings
//   - clears targeted bindings
//   - throw on invalid clears bindings payload
//   - throw on invalid clears bindings target

use std::collections::BTreeSet;

use serde_json::json;
use z2m_rs::extension::bind::Bind;

fn setup() -> Bind {
    let mut bind = Bind::new();
    bind.add_device("remote", "0x0017880104e45517", &["default", "1", "2"]);
    bind.add_device("bulb_color", "0x000b57fffec6a5b3", &["default", "1"]);
    bind.add_device("bulb", "0x000b57fffec6a5b2", &["default", "1"]);
    bind.add_device(
        "wall_switch_double",
        "0x0017880104e45542",
        &["default", "1", "2", "3", "right"],
    );
    bind.add_device("wall_switch", "0x0017880104e45543", &["default", "2"]);
    bind.add_device(
        "temperature_sensor",
        "0x00158d0001f4b5b2",
        &["default", "1"],
    );
    bind.add_device("heating_actuator", "0x00158d0001f4b5c2", &["default", "1"]);
    bind.add_device(
        "bj_scene_switch",
        "0x00158d0001f4b5d2",
        &["default", "1", "10"],
    );
    bind.add_device("tradfri_remote", "0x000d6ffffe1c2b2a", &["default", "1"]);
    bind.add_device("bulb_2", "0x000b57fffec6a5b7", &["default", "1"]);
    bind.add_device("bulb_color_2", "0x000b57fffec6a5b4", &["default", "1"]);
    bind.add_device("coordinator", "0x00124b00120144ae", &["default", "1"]);
    bind.add_device("button", "0x00158d0001f4b5a2", &["default", "1"]);
    bind.add_group("group_1");
    bind
}

fn has(bind: &Bind, topic: &str, payload: serde_json::Value) -> bool {
    bind.published
        .contains(&(topic.to_string(), payload, json!({})))
}

#[tokio::test]
async fn error_bind_fails_when_source_device() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "from_endpoint": "not_existing_endpoint", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Source device 'remote' does not have endpoint 'not_existing_endpoint'",
        })
    ));
}

#[tokio::test]
async fn error_bind_fails_when_source_device_does_not_exist() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote_not_existing", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Source device 'remote_not_existing' does not exist",
        })
    ));
}

#[tokio::test]
async fn error_bind_fails_when_target_device() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color", "to_endpoint": "not_existing_endpoint"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Target device 'bulb_color' does not have endpoint 'not_existing_endpoint'",
        })
    ));
}

#[tokio::test]
async fn error_bind_fails_when_target_device_or_group_does_not_exist() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color_not_existing"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Target device or group 'bulb_color_not_existing' does not exist",
        })
    ));
}

#[tokio::test]
async fn filters_out_reporting_setup_based_on_bind_status() {
    let mut bind = setup();
    bind.endpoint("bulb_color", "default").binds =
        vec![("genLevelCtrl".to_string(), "coordinator".to_string())];
    bind.endpoint("bulb_color", "default").configured_reportings =
        BTreeSet::from(["genLevelCtrl".to_string()]);

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"transaction": "1234", "from": "remote", "to": "bulb_color"}),
    );

    let target = bind.endpoint("bulb_color", "default");
    assert_eq!(target.configure_reporting.len(), 2);
    assert!(target
        .configure_reporting
        .iter()
        .any(|(cluster, _)| cluster == "genOnOff"));
    assert!(target
        .configure_reporting
        .iter()
        .any(|(cluster, _)| cluster == "lightingColorCtrl"));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
async fn filters_out_unsupported_clusters_for_reporting_setup() {
    let mut bind = setup();
    bind.endpoint("bulb_color", "default").input_clusters = BTreeSet::from([
        "genOnOff".to_string(),
        "genScenes".to_string(),
        "lightingColorCtrl".to_string(),
    ]);
    bind.endpoint("bulb_color", "default").binds =
        vec![("genLevelCtrl".to_string(), "coordinator".to_string())];

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"transaction": "1234", "from": "remote", "to": "bulb_color"}),
    );

    let target = bind.endpoint("bulb_color", "default");
    assert_eq!(target.configure_reporting.len(), 2);
    assert!(target
        .configure_reporting
        .iter()
        .any(|(cluster, _)| cluster == "genOnOff"));
    assert!(target
        .configure_reporting
        .iter()
        .any(|(cluster, _)| cluster == "lightingColorCtrl"));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
async fn should_allow_to_bind_to_coordinator_by_ieee_addr() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "0x00124b00120144ae", "clusters": ["genOnOff"]}),
    );

    assert_eq!(
        bind.endpoint("remote", "default").bind_calls,
        vec![("genOnOff".to_string(), "0x00124b00120144ae".to_string())]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "0x00124b00120144ae",
                "clusters": ["genOnOff"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_from_non_default_endpoint_ids() {
    let mut bind = setup();

    bind.on_message(
            "zigbee2mqtt/bridge/request/device/bind",
            &json!({"from": "remote", "from_endpoint": 2, "to": "wall_switch_double", "to_endpoint": 3}),
        );

    assert_eq!(
        bind.endpoint("remote", "2").bind_calls,
        vec![("genOnOff".to_string(), "wall_switch_double".to_string())]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": 2,
                "to": "wall_switch_double",
                "to_endpoint": 3,
                "clusters": ["genOnOff"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_from_non_default_endpoint_names() {
    let mut bind = setup();

    bind.on_message(
            "zigbee2mqtt/bridge/request/device/bind",
            &json!({"from": "remote", "from_endpoint": "ep2", "to": "wall_switch_double", "to_endpoint": "right"}),
        );

    assert_eq!(
        bind.endpoint("remote", "ep2").bind_calls,
        vec![("genOnOff".to_string(), "wall_switch_double".to_string())]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "ep2",
                "to": "wall_switch_double",
                "to_endpoint": "right",
                "clusters": ["genOnOff"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_only_specified_clusters() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color", "clusters": ["genOnOff"]}),
    );

    assert_eq!(
        bind.endpoint("remote", "default").bind_calls,
        vec![("genOnOff".to_string(), "bulb_color".to_string())]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "bulb_color",
                "clusters": ["genOnOff"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_server_clusters_to_client_clusters() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "temperature_sensor", "to": "heating_actuator"}),
    );

    assert_eq!(
        bind.endpoint("temperature_sensor", "default").bind_calls,
        vec![(
            "msTemperatureMeasurement".to_string(),
            "heating_actuator".to_string()
        )]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "temperature_sensor",
                "from_endpoint": "default",
                "to": "heating_actuator",
                "clusters": ["msTemperatureMeasurement"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_to_default_endpoint_returned_by_endpoints() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "from_endpoint": "ep2", "to": "wall_switch"}),
    );

    assert_eq!(
        bind.endpoint("remote", "ep2").bind_calls,
        vec![("genOnOff".to_string(), "wall_switch".to_string())]
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "ep2",
                "to": "wall_switch",
                "clusters": ["genOnOff"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_to_device_and_configure_reporting() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"transaction": "1234", "from": "remote", "to": "bulb_color"}),
    );

    assert_eq!(bind.endpoint("bulb_color", "default").reads.len(), 1);
    assert_eq!(
        bind.endpoint("bulb_color", "default").reads[0],
        (
            "lightingColorCtrl".to_string(),
            vec!["colorCapabilities".to_string()]
        )
    );
    assert_eq!(bind.endpoint("remote", "1").bind_calls.len(), 4);
    assert_eq!(
        bind.endpoint("bulb_color", "default")
            .configure_reporting
            .len(),
        3
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "transaction": "1234",
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "bulb_color",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl", "lightingColorCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
async fn should_bind_to_group_by_number() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "1"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.bind_calls.len(), 3);
    assert!(endpoint
        .bind_calls
        .contains(&("genOnOff".to_string(), "1".to_string())));
    assert!(endpoint
        .bind_calls
        .contains(&("genLevelCtrl".to_string(), "1".to_string())));
    assert!(endpoint
        .bind_calls
        .contains(&("genScenes".to_string(), "1".to_string())));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "1",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_bind_to_groups() {
    let mut bind = setup();
    bind.add_group_member("group_1", "bulb", "1");

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "group_1"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.bind_calls.len(), 3);
    assert!(endpoint
        .bind_calls
        .contains(&("genOnOff".to_string(), "group_1".to_string())));
    assert!(endpoint
        .bind_calls
        .contains(&("genLevelCtrl".to_string(), "group_1".to_string())));
    assert!(endpoint
        .bind_calls
        .contains(&("genScenes".to_string(), "group_1".to_string())));

    let member = bind.endpoint("bulb", "1");
    assert_eq!(member.configure_reporting.len(), 2);
    assert!(member.configure_reporting.contains(&(
            "genOnOff".to_string(),
            json!([{"attribute": "onOff", "maximumReportInterval": 3600, "minimumReportInterval": 0, "reportableChange": 0}])
        )));
    assert!(member.configure_reporting.contains(&(
            "genLevelCtrl".to_string(),
            json!([{"attribute": "currentLevel", "maximumReportInterval": 3600, "minimumReportInterval": 5, "reportableChange": 1}])
        )));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "group_1",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_log_error_when_there_is_nothing_to_bind() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "button"}),
    );

    assert!(bind.endpoint("remote", "default").bind_calls.is_empty());
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({"data": {}, "status": "error", "error": "Nothing to bind"})
    ));
}

#[tokio::test]
async fn should_log_when_bind_fails() {
    let mut bind = setup();
    bind.bind_error = true;

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color"}),
    );

    assert_eq!(bind.endpoint("remote", "default").bind_calls.len(), 3);
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({"data": {}, "status": "error", "error": "Failed to bind"})
    ));
}

#[tokio::test]
async fn should_poll_bounded_hue_bulb_when_receiving_message_from_hue_dimmer() {
    let mut bind = setup();

    bind.on_zigbee_message(
        "remote",
        "2",
        "manuSpecificPhilips",
        &json!({"button": 3, "unknown1": 3145728, "type": 2, "unknown2": 0, "time": 1}),
        0,
    );

    assert_eq!(bind.debounce_calls, 1);
    assert!(bind.poll_reads.contains(&(
        "bulb_color".to_string(),
        "genLevelCtrl".to_string(),
        vec!["currentLevel".to_string()]
    )));
}

#[tokio::test]
async fn should_poll_bounded_hue_bulb_when_receiving_message_from_scene_controller() {
    let mut bind = setup();

    bind.on_zigbee_message(
        "bj_scene_switch",
        "10",
        "genScenes",
        &json!({"action": "recall_2_row_1"}),
        0,
    );

    assert_eq!(bind.debounce_calls, 3);
    for (cluster, attributes) in [
        ("genOnOff", vec!["onOff".to_string()]),
        ("genLevelCtrl", vec!["currentLevel".to_string()]),
        (
            "lightingColorCtrl",
            vec![
                "currentX".to_string(),
                "currentY".to_string(),
                "colorTemperature".to_string(),
            ],
        ),
    ] {
        assert!(bind.poll_reads.contains(&(
            "bulb_color_2".to_string(),
            cluster.to_string(),
            attributes
        )));
    }
}

#[tokio::test]
async fn should_poll_grouped_hue_bulb_when_receiving_message_from_tradfri_remote() {
    let mut bind = setup();

    bind.on_zigbee_message(
        "tradfri_remote",
        "1",
        "genLevelCtrl",
        &json!({"stepmode": 0, "stepsize": 43, "transtime": 5}),
        15071,
    );

    assert_eq!(bind.debounce_calls, 2);
    assert_eq!(
        bind.poll_reads
            .iter()
            .filter(|(device, _, _)| device == "bulb_color_2")
            .count(),
        2
    );
    assert!(bind.poll_reads.contains(&(
        "bulb_color_2".to_string(),
        "genLevelCtrl".to_string(),
        vec!["currentLevel".to_string()]
    )));
    assert!(bind.poll_reads.contains(&(
        "bulb_color_2".to_string(),
        "genOnOff".to_string(),
        vec!["onOff".to_string()]
    )));
    assert_eq!(
        bind.poll_reads
            .iter()
            .filter(|(device, _, _)| device == "bulb_2")
            .count(),
        0
    );
}

#[tokio::test]
async fn should_throw_error_on_invalid_payload() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"fromz": "remote", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
async fn should_unbind() {
    let mut bind = setup();
    bind.endpoint("bulb_color", "1").binds = vec![
        ("genOnOff".to_string(), "coordinator".to_string()),
        ("genLevelCtrl".to_string(), "coordinator".to_string()),
        ("lightingColorCtrl".to_string(), "coordinator".to_string()),
    ];

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "bulb_color"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.unbind_calls.len(), 3);
    assert!(endpoint
        .unbind_calls
        .contains(&("genOnOff".to_string(), "bulb_color".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genLevelCtrl".to_string(), "bulb_color".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genScenes".to_string(), "bulb_color".to_string())));

    let target = bind.endpoint("bulb_color", "default");
    assert_eq!(target.configure_reporting.len(), 3);
    assert!(target.configure_reporting.contains(&(
            "genOnOff".to_string(),
            json!([{"attribute": "onOff", "maximumReportInterval": 0xffff, "minimumReportInterval": 0, "reportableChange": 0}])
        )));
    assert!(target.configure_reporting.contains(&(
            "genLevelCtrl".to_string(),
            json!([{"attribute": "currentLevel", "maximumReportInterval": 0xffff, "minimumReportInterval": 5, "reportableChange": 1}])
        )));
    assert!(target.configure_reporting.contains(&(
            "lightingColorCtrl".to_string(),
            json!([
                {"attribute": "colorTemperature", "minimumReportInterval": 5, "maximumReportInterval": 0xffff, "reportableChange": 1},
                {"attribute": "currentX", "minimumReportInterval": 5, "maximumReportInterval": 0xffff, "reportableChange": 1},
                {"attribute": "currentY", "minimumReportInterval": 5, "maximumReportInterval": 0xffff, "reportableChange": 1},
            ])
        )));
    assert_eq!(
        bind.devices["bulb_color"].ieee_address,
        "0x000b57fffec6a5b3"
    );
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/unbind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "bulb_color",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_unbind_coordinator() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "Coordinator"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.unbind_calls.len(), 3);
    assert!(endpoint
        .unbind_calls
        .contains(&("genOnOff".to_string(), "Coordinator".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genLevelCtrl".to_string(), "Coordinator".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genScenes".to_string(), "Coordinator".to_string())));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/unbind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "Coordinator",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_unbind_from_default_bind_group() {
    let mut bind = setup();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "default_bind_group"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.unbind_calls.len(), 3);
    assert!(endpoint
        .unbind_calls
        .contains(&("genOnOff".to_string(), "default_bind_group".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genLevelCtrl".to_string(), "default_bind_group".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genScenes".to_string(), "default_bind_group".to_string())));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/unbind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "default_bind_group",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_unbind_from_group() {
    let mut bind = setup();
    bind.add_group_member("group_1", "bulb", "1");

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "group_1"}),
    );

    let endpoint = bind.endpoint("remote", "default");
    assert_eq!(endpoint.unbind_calls.len(), 3);
    assert!(endpoint
        .unbind_calls
        .contains(&("genOnOff".to_string(), "group_1".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genLevelCtrl".to_string(), "group_1".to_string())));
    assert!(endpoint
        .unbind_calls
        .contains(&("genScenes".to_string(), "group_1".to_string())));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/unbind",
        json!({
            "data": {
                "from": "remote",
                "from_endpoint": "default",
                "to": "group_1",
                "clusters": ["genScenes", "genOnOff", "genLevelCtrl"],
                "failed": [],
            },
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_unbind_from_group_with_skip_disable_reporting_false() {
    let mut bind = setup();
    bind.add_group_member("group_1", "bulb_2", "1");

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "group_1", "skip_disable_reporting": false}),
    );

    assert_eq!(bind.endpoint("remote", "default").unbind_calls.len(), 3);
    let member = bind.endpoint("bulb_2", "1");
    assert_eq!(member.configure_reporting.len(), 2);
    assert!(member.configure_reporting.contains(&(
            "genLevelCtrl".to_string(),
            json!([{"attribute": "currentLevel", "maximumReportInterval": 65535, "minimumReportInterval": 5, "reportableChange": 1}])
        )));
    assert!(member.configure_reporting.contains(&(
            "genOnOff".to_string(),
            json!([{"attribute": "onOff", "maximumReportInterval": 65535, "minimumReportInterval": 0, "reportableChange": 0}])
        )));
}

#[tokio::test]
async fn should_unbind_from_group_with_skip_disable_reporting_true() {
    let mut bind = setup();
    bind.add_group_member("group_1", "bulb_2", "1");

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/unbind",
        &json!({"from": "remote", "to": "group_1", "skip_disable_reporting": true}),
    );

    assert_eq!(bind.endpoint("remote", "default").unbind_calls.len(), 3);
    assert!(bind.endpoint("bulb_2", "1").configure_reporting.is_empty());
}

#[tokio::test]
async fn clears_all_bindings() {
    let mut bind = setup();
    bind.published.clear();
    bind.devices_changed = 0;

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"transaction": "1234", "target": "remote"}),
    );

    assert_eq!(bind.clear_bindings_calls.get("remote").copied(), Some(1));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({
            "transaction": "1234",
            "data": {
                "target": "remote",
                "ieee_list": ["0xffffffffffffffff"],
            },
            "status": "ok",
        })
    ));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
async fn clears_targeted_bindings() {
    let mut bind = setup();
    bind.published.clear();
    bind.devices_changed = 0;

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({
            "transaction": "1234",
            "target": "remote",
            "ieee_list": ["0x000b57fffec6a5b3"],
        }),
    );

    assert_eq!(bind.clear_bindings_calls.get("remote").copied(), Some(1));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({
            "transaction": "1234",
            "data": {
                "target": "remote",
                "ieee_list": ["0x000b57fffec6a5b3"],
            },
            "status": "ok",
        })
    ));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
async fn throw_on_invalid_clears_bindings_payload() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"targetz": "remote"}),
    );

    assert!(bind.clear_bindings_calls.is_empty());
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
async fn throw_on_invalid_clears_bindings_target() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"target": "remotez"}),
    );

    assert!(bind.clear_bindings_calls.is_empty());
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({"data": {}, "status": "error", "error": "Invalid target"})
    ));
}
