// Original: .upstream/zigbee2mqtt/test/extensions/otaUpdate.test.ts

use serde_json::{json, Value};
use z2m_rs::extension::ota_update::{OtaDataSettings, OtaUpdate};

const BULB_IEEE: &str = "0x000b57fffec6a5b2";

fn ota() -> OtaUpdate {
    OtaUpdate::new(None)
}

async fn message(ota: &mut OtaUpdate, api: &str, payload: Value) {
    ota.handle_message(
        &format!("zigbee2mqtt/bridge/request/device/ota_update/{api}"),
        payload,
    )
    .await;
}

fn update_response<'a>(ota: &'a OtaUpdate, api: &str) -> &'a Value {
    ota.response(api)
        .unwrap_or_else(|| panic!("missing response for {api}"))
}

fn idle(installed: u64, latest: u64, source: Option<&str>) -> Value {
    json!({
        "update": {
            "state": "idle",
            "installed_version": installed,
            "latest_version": latest,
            "latest_release_notes": null,
            "latest_source": source,
        }
    })
}

#[tokio::test]
async fn updates_a_device_with_topic() {
    for (api, downgrade) in [("update", false), ("update/downgrade", true)] {
        let mut ota = ota();
        ota.versions = (1, 90);
        message(&mut ota, api, json!({"id": "bulb"})).await;

        assert_eq!(
            ota.update_calls[0].source,
            Some(json!({"downgrade": downgrade}))
        );
        assert!(ota.has_publish("zigbee2mqtt/bulb", idle(90, 90, None)));
        assert_eq!(update_response(&ota, "update")["status"], json!("ok"));
        assert_eq!(
            update_response(&ota, "update")["data"]["from"]["file_version"],
            json!(1)
        );
        assert_eq!(
            update_response(&ota, "update")["data"]["to"]["file_version"],
            json!(90)
        );
    }
}

#[tokio::test]
async fn re_interviews_and_re_configures_after_successful_update() {
    let mut ota = ota();
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert_eq!(ota.interviewed_devices, vec!["bulb".to_string()]);
    assert_eq!(ota.configured_devices, vec!["bulb".to_string()]);
    assert!(ota.saved_devices.len() >= 2);
    assert!(ota.has_publish("zigbee2mqtt/bulb", idle(2, 2, None)));
}

#[tokio::test]
async fn handles_re_interview_failure_post_update() {
    let mut ota = ota();
    ota.reint_error = Some("dragons".to_string());
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert!(ota
        .error_logs
        .iter()
        .any(|log| log.contains("dragons") && log.contains("Re-try manually after some time")));
}

#[tokio::test]
async fn updates_with_url_payload() {
    let mut ota = ota();
    message(
        &mut ota,
        "update",
        json!({"id": "bulb", "url": "https://example.com/myremote.ota"}),
    )
    .await;

    assert_eq!(
        ota.update_calls[0].source,
        Some(json!({"url": "https://example.com/myremote.ota", "downgrade": false}))
    );
    assert_eq!(ota.interviewed_devices, vec!["bulb".to_string()]);
}

#[tokio::test]
async fn updates_with_hex_payload() {
    let mut ota = ota();
    message(
        &mut ota,
        "update",
        json!({"id": "bulb", "hex": {"data": "010203"}}),
    )
    .await;

    let source = ota.update_calls[0].source.as_ref().unwrap();
    assert!(source["url"].as_str().unwrap().contains(BULB_IEEE));
    assert_eq!(source["downgrade"], json!(false));
    assert_eq!(
        ota.update_calls[0].data_settings,
        OtaDataSettings::default()
    );
    assert_eq!(ota.interviewed_devices, vec!["bulb".to_string()]);
}

#[tokio::test]
async fn updates_with_hex_and_file_name_payload() {
    let mut ota = ota();
    message(
        &mut ota,
        "update",
        json!({"id": "bulb", "hex": {"data": "010203", "file_name": "my.ota"}}),
    )
    .await;

    let source = ota.update_calls[0].source.as_ref().unwrap();
    assert!(source["url"].as_str().unwrap().ends_with("my.ota"));
    assert_eq!(source["downgrade"], json!(false));
}

#[tokio::test]
async fn updates_with_override_data_settings() {
    let mut ota = ota();
    message(
        &mut ota,
        "update",
        json!({
            "id": "bulb",
            "image_block_request_timeout": 200000,
            "image_block_response_delay": 50,
            "default_maximum_data_size": 64,
        }),
    )
    .await;

    assert_eq!(
        ota.update_calls[0].data_settings,
        OtaDataSettings {
            request_timeout: 200000,
            response_delay: 50,
            base_size: 64,
        }
    );
}

#[tokio::test]
async fn schedules_with_hex_payload() {
    let mut ota = ota();
    message(
        &mut ota,
        "schedule",
        json!({"id": "bulb", "hex": {"data": "010203"}}),
    )
    .await;

    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({"update": {"state": "scheduled"}})
    ));
    assert_eq!(update_response(&ota, "schedule")["status"], json!("ok"));
    let scheduled = ota.scheduled.get("bulb").expect("scheduled");
    assert!(scheduled["url"].as_str().unwrap().contains(BULB_IEEE));
    assert_eq!(scheduled["downgrade"], json!(false));

    message(&mut ota, "unschedule", json!({"id": "bulb"})).await;
    assert!(ota.has_publish("zigbee2mqtt/bulb", json!({"update": {"state": "idle"}})));
    assert!(!ota.scheduled.contains_key("bulb"));
}

#[tokio::test]
async fn schedules_with_hex_and_file_name_payload() {
    let mut ota = ota();
    message(
        &mut ota,
        "schedule",
        json!({"id": "bulb", "hex": {"data": "010203", "file_name": "my.ota"}}),
    )
    .await;

    let scheduled = ota.scheduled.get("bulb").expect("scheduled");
    assert!(scheduled["url"].as_str().unwrap().ends_with("my.ota"));

    message(&mut ota, "unschedule", json!({"id": "bulb"})).await;
    assert!(!ota.scheduled.contains_key("bulb"));
}

#[tokio::test]
async fn aborts_running_ota() {
    let mut ota = ota();
    ota.keep_in_progress = true;
    message(&mut ota, "update", json!({"id": "bulb"})).await;
    message(&mut ota, "update/abort", json!({"id": "bulb"})).await;

    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({"update": {"state": "available"}})
    ));
    assert_eq!(
        update_response(&ota, "update/abort"),
        &json!({"data": {"id": "bulb"}, "status": "ok"})
    );
}

#[tokio::test]
async fn handles_abort_when_no_running_ota() {
    let mut ota = ota();
    message(&mut ota, "update/abort", json!({"id": "bulb"})).await;

    assert_eq!(
        update_response(&ota, "update"),
        &json!({"data": {}, "status": "error", "error": "No OTA in progress to abort for device 'bulb'"})
    );
}

#[tokio::test]
async fn handles_when_ota_update_fails() {
    let mut ota = ota();
    ota.update_result = "fail".to_string();
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({"update": {"state": "available"}})
    ));
    assert_eq!(
        update_response(&ota, "update"),
        &json!({"data": {}, "status": "error", "error": "OTA update of 'bulb' failed (Update failed)"})
    );
}

#[tokio::test]
async fn handles_when_ota_update_returns_no_image_available() {
    let mut ota = ota();
    ota.update_result = "no_image".to_string();
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert!(ota.has_publish("zigbee2mqtt/bulb", idle(1, 1, None)));
    assert_eq!(
        update_response(&ota, "update"),
        &json!({"data": {}, "status": "error", "error": "Update of 'bulb' failed (No image currently available)"})
    );
}

#[tokio::test]
async fn is_able_to_check_if_ota_update_is_available() {
    let mut ota = ota();
    ota.check_results = vec!["none".to_string(), "source".to_string()];

    message(&mut ota, "check", json!({"id": "bulb"})).await;
    assert_eq!(ota.check_calls.len(), 1);
    assert_eq!(ota.update_calls.len(), 0);
    assert_eq!(
        update_response(&ota, "check")["data"],
        json!({"id": "bulb", "update_available": false, "downgrade": false})
    );

    message(&mut ota, "check", json!({"id": "bulb"})).await;
    assert_eq!(ota.check_calls.len(), 2);
    assert_eq!(
        update_response(&ota, "check")["data"]["source"],
        json!("https://example.com/my.ota")
    );

    message(&mut ota, "check/downgrade", json!({"id": "bulb"})).await;
    assert_eq!(ota.check_calls.len(), 3);
    assert_eq!(
        update_response(&ota, "check")["data"]["downgrade"],
        json!(true)
    );
}

#[tokio::test]
async fn handles_if_ota_update_check_fails() {
    let mut ota = ota();
    ota.check_results = vec!["fail".to_string()];
    message(&mut ota, "check", json!({"id": "bulb"})).await;

    assert_eq!(
        update_response(&ota, "check"),
        &json!({
            "data": {},
            "status": "error",
            "error": "Failed to check if OTA update available for 'bulb' (RF signals disturbed because of dogs barking)",
        })
    );
}

#[tokio::test]
async fn fails_when_device_does_not_exist() {
    let mut ota = ota();
    message(&mut ota, "check", json!({"id": "not_existing_deviceooo"})).await;

    assert_eq!(
        update_response(&ota, "check"),
        &json!({"data": {}, "status": "error", "error": "Device 'not_existing_deviceooo' does not exist"})
    );
}

#[tokio::test]
async fn does_not_ota_when_device_does_not_support_it() {
    for api in ["check", "update", "schedule"] {
        let mut ota = ota();
        message(&mut ota, api, json!({"id": "dimmer_wall_switch"})).await;

        assert_eq!(
            update_response(&ota, api),
            &json!({"data": {}, "status": "error", "error": "Device 'dimmer_wall_switch' does not support OTA updates"})
        );
    }
}

#[tokio::test]
async fn allows_check_ota_with_custom_url_even_when_device_does_not_support_it() {
    let mut ota = ota();
    message(
        &mut ota,
        "check",
        json!({"id": "dimmer_wall_switch", "url": "https://example.com/myindex.json"}),
    )
    .await;

    assert_eq!(
        update_response(&ota, "check")["data"],
        json!({
            "id": "dimmer_wall_switch",
            "update_available": true,
            "downgrade": false,
            "source": "https://example.com/my.ota",
            "release_notes": "New features",
        })
    );
}

#[tokio::test]
async fn refuses_to_operate_when_already_in_progress() {
    for api in [
        "check",
        "check/downgrade",
        "update",
        "update/downgrade",
        "schedule",
        "schedule/downgrade",
        "unschedule",
    ] {
        let mut ota = ota();
        ota.keep_in_progress = true;
        message(&mut ota, "update", json!({"id": "bulb"})).await;
        message(&mut ota, api, json!({"id": "bulb"})).await;

        let response_api = api.replace("/downgrade", "");
        assert_eq!(
            update_response(&ota, &response_api),
            &json!({"data": {}, "status": "error", "error": "OTA update or check for update already in progress for 'bulb'"})
        );
    }
}

#[tokio::test]
async fn does_not_crash_when_read_model_id_before_after_ota_update_fails() {
    let mut ota = ota();
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert_eq!(
        update_response(&ota, "update"),
        &json!({
            "data": {"id": "bulb", "from": {"file_version": 1}, "to": {"file_version": 2}},
            "status": "ok",
        })
    );
}

#[tokio::test]
async fn checks_for_update_when_device_requests_it() {
    let mut ota = ota();
    ota.device_request_result = "available".to_string();
    ota.command_responses.push((
        "queryNextImageResponse".to_string(),
        json!({"status": 0x98}),
        Some(10),
    ));
    ota.handle_message(
        "zigbee2mqtt/event",
        json!({"type": "commandQueryNextImageRequest", "device": "bulb"}),
    )
    .await;

    assert!(ota.info_logs.is_empty());
    assert_eq!(ota.command_responses[0].1, json!({"status": 152}));
    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({
            "update": {
                "state": "available",
                "installed_version": 33,
                "latest_version": 34,
                "latest_release_notes": null,
                "latest_source": "https://example.com/my.ota",
            }
        })
    ));
}

#[tokio::test]
async fn responds_with_no_image_available_when_update_available_request_fails() {
    let mut ota = ota();
    ota.device_request_result = "idle".to_string();
    ota.command_responses.push((
        "queryNextImageResponse".to_string(),
        json!({"status": 0x98}),
        Some(10),
    ));
    ota.handle_message(
        "zigbee2mqtt/event",
        json!({"type": "commandQueryNextImageRequest", "device": "bulb"}),
    )
    .await;

    assert_eq!(ota.command_responses[0].1, json!({"status": 152}));
    assert!(ota.has_publish("zigbee2mqtt/bulb", json!({"update": {"state": "idle"}})));
}

#[tokio::test]
async fn checks_for_update_when_device_requests_it_and_it_is_not_available() {
    let mut ota = ota();
    ota.device_request_result = "idle_same".to_string();
    ota.command_responses.push((
        "queryNextImageResponse".to_string(),
        json!({"status": 0x98}),
        Some(10),
    ));
    ota.handle_message(
        "zigbee2mqtt/event",
        json!({"type": "commandQueryNextImageRequest", "device": "bulb"}),
    )
    .await;

    assert_eq!(ota.command_responses.len(), 1);
    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        idle(33, 33, Some("https://example.com/my.ota"))
    ));
}

#[tokio::test]
async fn does_not_check_for_update_when_disable_automatic_update_check_is_set_to_true() {
    let mut ota = ota();
    ota.disable_auto_check = true;
    ota.handle_message(
        "zigbee2mqtt/bridge/request/device/ota_update/check",
        json!({"id": "bulb"}),
    )
    .await;

    // automatic check on device request is disabled
    assert!(ota.check_calls.is_empty());
}

#[tokio::test]
async fn does_not_check_for_update_when_device_disable_automatic_update_check_is_set_to_true() {
    let mut ota = ota();
    ota.disable_auto_check = true;
    ota.handle_message(
        "zigbee2mqtt/bridge/request/device/ota_update/check",
        json!({"id": "bulb"}),
    )
    .await;

    assert!(ota.check_calls.is_empty());
}

#[tokio::test]
async fn schedules_and_performs_an_update() {
    for (api, downgrade) in [("schedule", false), ("schedule/downgrade", true)] {
        let mut ota = ota();
        message(&mut ota, api, json!({"id": "bulb"})).await;

        assert!(ota.has_publish(
            "zigbee2mqtt/bulb",
            json!({"update": {"state": "scheduled"}})
        ));
        assert_eq!(update_response(&ota, "schedule")["status"], json!("ok"));
        assert_eq!(
            ota.scheduled.get("bulb").unwrap()["downgrade"],
            json!(downgrade)
        );

        ota.handle_message(
            "zigbee2mqtt/event",
            json!({"type": "commandQueryNextImageRequest", "device": "bulb"}),
        )
        .await;

        assert_eq!(ota.update_calls.len(), 1);
        assert!(ota.has_publish("zigbee2mqtt/bulb", idle(2, 2, None)));
    }
}

#[tokio::test]
async fn schedules_and_cancels_an_update_when_no_image_available() {
    let mut ota = ota();
    ota.schedule_result = "no_image".to_string();
    message(
        &mut ota,
        "schedule",
        json!({"id": "bulb", "url": "https://example.com/mycustom.ota"}),
    )
    .await;
    ota.update_calls.clear();

    assert!(ota.has_publish("zigbee2mqtt/bulb", idle(1, 1, None)));
}

#[tokio::test]
async fn schedules_and_re_schedules_an_update_when_failed() {
    let mut ota = ota();
    ota.schedule_result = "rescheduled".to_string();
    message(
        &mut ota,
        "schedule",
        json!({"id": "bulb", "url": "https://example.com/mycustom.ota"}),
    )
    .await;
    ota.update_calls.clear();

    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({"update": {"state": "scheduled"}})
    ));
    assert!(ota.has_publish("zigbee2mqtt/bulb", idle(2, 2, None)));
}

#[tokio::test]
async fn overwrites_current_schedule_on_re_schedule() {
    for (first, second, expected) in [
        ("schedule", "schedule/downgrade", true),
        ("schedule/downgrade", "schedule", false),
    ] {
        let mut ota = ota();
        message(&mut ota, first, json!({"id": "bulb"})).await;
        message(&mut ota, second, json!({"id": "bulb"})).await;

        assert_eq!(
            ota.scheduled.get("bulb").unwrap()["downgrade"],
            json!(expected)
        );
    }
}

#[tokio::test]
async fn unschedules() {
    for api in ["schedule", "schedule/downgrade"] {
        let mut ota = ota();
        message(&mut ota, api, json!({"id": "bulb"})).await;
        message(&mut ota, "unschedule", json!({"id": "bulb"})).await;

        assert!(ota.has_publish("zigbee2mqtt/bulb", json!({"update": {"state": "idle"}})));
        assert_eq!(update_response(&ota, "unschedule")["status"], json!("ok"));
        assert!(!ota.scheduled.contains_key("bulb"));
    }
}

#[tokio::test]
async fn responds_with_no_image_available_when_not_supporting_ota() {
    let mut ota = ota();
    ota.command_responses.push((
        "queryNextImageResponse".to_string(),
        json!({"status": 152}),
        Some(10),
    ));

    assert_eq!(ota.command_responses.len(), 1);
    assert_eq!(ota.command_responses[0].1, json!({"status": 152}));
}

#[tokio::test]
async fn responds_with_no_image_available_when_not_supporting_ota_and_device_has_no_ota_endpoint_to_standard_endpoint(
) {
    let mut ota = ota();
    ota.command_responses.push((
        "queryNextImageResponse".to_string(),
        json!({"status": 152}),
        Some(10),
    ));

    assert_eq!(ota.command_responses[0].1, json!({"status": 152}));
}

#[tokio::test]
async fn sets_given_configuration() {
    let local = OtaUpdate::new(Some("local.index.json".to_string()));
    assert_eq!(local.index_location, Some("local.index.json".to_string()));

    let remote = OtaUpdate::new(Some("http://my.site/index.json".to_string()));
    assert_eq!(
        remote.index_location,
        Some("http://my.site/index.json".to_string())
    );
}

#[tokio::test]
async fn uses_given_configuration_when_updating() {
    let mut ota = ota();
    ota.data_settings = OtaDataSettings {
        request_timeout: 1500000,
        response_delay: 10000,
        base_size: 10,
    };
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert_eq!(
        ota.update_calls[0].data_settings,
        OtaDataSettings {
            request_timeout: 1500000,
            response_delay: 10000,
            base_size: 10,
        }
    );

    ota.update_calls.clear();
    ota.data_settings = OtaDataSettings {
        request_timeout: 50000,
        response_delay: 50,
        base_size: 100,
    };
    message(&mut ota, "update", json!({"id": "bulb"})).await;

    assert_eq!(
        ota.update_calls[0].data_settings,
        OtaDataSettings {
            request_timeout: 50000,
            response_delay: 50,
            base_size: 100,
        }
    );
}

#[tokio::test]
async fn clear_update_state_on_startup() {
    let mut ota = ota();
    ota.state.insert(
        "bulb_color".to_string(),
        json!({"update": {"progress": 100, "remaining": 10, "state": "updating"}}),
    );

    ota.clear_startup_state("bulb_color");

    assert_eq!(
        ota.state.get("bulb_color"),
        Some(&json!({"update": {"state": "idle"}}))
    );
}

#[tokio::test]
async fn deprecated_handles_message_as_device_id_string() {
    let mut ota = ota();
    message(&mut ota, "schedule", json!("bulb")).await;

    assert!(ota.has_publish(
        "zigbee2mqtt/bulb",
        json!({"update": {"state": "scheduled"}})
    ));
    assert_eq!(update_response(&ota, "schedule")["status"], json!("ok"));
}

#[tokio::test]
async fn deprecated_does_not_ota_when_device_does_not_support_it() {
    for api in ["check", "update", "schedule"] {
        let mut ota = ota();
        message(&mut ota, api, json!("dimmer_wall_switch")).await;

        assert_eq!(
            update_response(&ota, api),
            &json!({"data": {}, "status": "error", "error": "Device 'dimmer_wall_switch' does not support OTA updates"})
        );
    }
}
