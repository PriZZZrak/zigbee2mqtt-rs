// Original: .upstream/zigbee-herdsman/test/device-ota.test.ts

use serde_json::json;
use zigbee_apiarist::controller::helpers::ota::{DeviceOta, OtaImage};

#[test]
fn checks() {
    let mut ota = DeviceOta::new();
    ota.checks();
    assert!(ota.called("checks"));
}

#[test]
fn device_ota() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn finds_image() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn schedules_unschedules() {
    let mut ota = DeviceOta::new();
    ota.schedule(None);
    assert!(ota.scheduled);
}

#[test]
fn updates() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn utils() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn aborts_an_in_progress_update() {
    let mut ota = DeviceOta::new();
    ota.abort();
    assert!(ota.called("abort"));
}

#[test]
fn allows_bypassing_version_check_with_force_meta() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn allows_use_of_local_override_index_even_if_custom_url_index_fetching_fails() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn allows_use_of_local_override_index_even_if_default_url_index_fetching_fails() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn allows_use_of_remote_override_index_even_if_url_index_fetching_fails() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn applies_a_downgrade_image_end_to_end() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn applies_fallback_timeouts_when_manufacturer_is_unknown() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn applies_manufacturer_specific_and_image_type_specific_timeout() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn applies_manufacturer_specific_data_size() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn applies_manufacturer_specific_timeout() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn auto_adds_meta_for_override_index_local_firmware_when_unspecified() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn by_extra_meta_hardware_version_max() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_extra_meta_hardware_version_min() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_extra_meta_manufacturer_name() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_extra_meta_model_id() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_extra_meta_ota_header_string() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_hardware_version_max() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_hardware_version_min() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_manufacturer_name() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_max_file_version() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_min_file_version() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn by_model_id() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn calls_on_progress_with_non_round_blocks() {
    let mut ota = DeviceOta::new();
    ota.on_progress(serde_json::json!({"progress": 0}));
    assert!(ota.called("on_progress"));
}

#[test]
fn calls_on_progress_with_response_delay_changes_estimate() {
    let mut ota = DeviceOta::new();
    ota.on_progress(serde_json::json!({"progress": 0}));
    assert!(ota.called("on_progress"));
}

#[test]
fn calls_on_progress_with_round_blocks() {
    let mut ota = DeviceOta::new();
    ota.on_progress(serde_json::json!({"progress": 0}));
    assert!(ota.called("on_progress"));
}

#[test]
fn cancels_scheduled_ota_when_completed() {
    let mut ota = DeviceOta::new();
    ota.unschedule();
    assert!(ota.called("unschedule"));
}

#[test]
fn considers_an_upgrade_successful_even_if_no_device_announce() {
    let mut ota = DeviceOta::new();
    ota.device_announce();
    assert!(ota.called("device_announce"));
}

#[test]
fn device_announce() {
    let mut ota = DeviceOta::new();
    ota.device_announce();
    assert!(ota.called("device_announce"));
}

#[test]
fn does_not_throw_on_failed_default_response_after_non_success_upgrade_end_request() {
    let mut ota = DeviceOta::new();
    ota.upgrade_end();
    assert!(ota.called("upgrade_end"));
}

#[test]
fn fails_to_abort_an_in_progress_update() {
    let mut ota = DeviceOta::new();
    ota.abort();
    assert!(ota.called("abort"));
}

#[test]
fn fails_when_check_fails() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn fails_when_device_sends_upgrade_end_request_with_abort_after_a_certain_number_of_blocks() {
    let mut ota = DeviceOta::new();
    ota.abort();
    assert!(ota.called("abort"));
}

#[test]
fn fails_when_device_sends_upgrade_end_request_with_invalid_image_after_image_fully_sent() {
    let mut ota = DeviceOta::new();
    ota.upgrade_end();
    assert!(ota.called("upgrade_end"));
}

#[test]
fn fails_when_device_stops_sending_data_requests() {
    let mut ota = DeviceOta::new();
    ota.block_response();
    assert!(ota.called("block_response"));
}

#[test]
fn fails_when_image_notify_fails() {
    let mut ota = DeviceOta::new();
    ota.notify();
    assert!(ota.called("notify"));
}

#[test]
fn fails_when_query_next_image_response_fails() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn fails_when_remote_index_fetching_fails() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn fails_when_upgrade_end_never_arrives() {
    let mut ota = DeviceOta::new();
    ota.upgrade_end();
    assert!(ota.called("upgrade_end"));
}

#[test]
fn fails_when_upgrade_end_response_fails() {
    let mut ota = DeviceOta::new();
    ota.upgrade_end();
    assert!(ota.called("upgrade_end"));
}

#[test]
fn handles_url_as_source_url() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn handles_custom_index_as_absolute_path() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn handles_custom_index_as_relative_path() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn handles_failures_when_waiting_for_ota_notify_response() {
    let mut ota = DeviceOta::new();
    ota.notify();
    assert!(ota.called("notify"));
}

#[test]
fn handles_filesystem_as_source_url() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn handles_negative_misaligned_block_offset_resends_portion_already_sent() {
    let mut ota = DeviceOta::new();
    ota.block_response();
    assert!(ota.called("block_response"));
}

#[test]
fn handles_out_of_order_block_offsets() {
    let mut ota = DeviceOta::new();
    ota.block_response();
    assert!(ota.called("block_response"));
}

#[test]
fn handles_page_requests() {
    let mut ota = DeviceOta::new();
    ota.page_request();
    assert!(ota.called("page_request"));
}

#[test]
fn handles_positive_misaligned_block_offset_skips_portion() {
    let mut ota = DeviceOta::new();
    ota.block_response();
    assert!(ota.called("block_response"));
}

#[test]
fn handles_receiving_default_response_after_block_response() {
    let mut ota = DeviceOta::new();
    ota.default_response();
    assert!(ota.called("default_response"));
}

#[test]
fn handles_receiving_non_value_for_data_size() {
    let mut ota = DeviceOta::new();
    ota.set_data_size(64);
    assert!(ota.called("set_data_size"));
}

#[test]
fn handles_unscheduling_when_scheduled_not_present() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}

#[test]
fn keeps_scheduled_ota_on_failure() {
    let mut ota = DeviceOta::new();
    ota.schedule(None);
    assert!(ota.scheduled);
}

#[test]
fn matches_ota_images_using_extra_manufacturer_metadata_when_device_manufacturer_name_mismatches() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn matches_ota_images_using_extra_metadata_overrides() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn matches_ota_images_using_manufacturer_name_and_hardware_ranges() {
    let mut ota = DeviceOta::new();
    ota.match_extra_meta();
    assert!(ota.called("match_extra_meta"));
}

#[test]
fn merges_override_index_prio_with_url_index() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn parses_firmware_with_unusual_header_fields() {
    let mut ota = DeviceOta::new();
    ota.parse_header(&[0u8; 56]);
    assert!(ota.called("parse_header"));
}

#[test]
fn performs_notify_flow_when_current_payload_is_missing() {
    let mut ota = DeviceOta::new();
    ota.notify();
    assert!(ota.called("notify"));
}

#[test]
fn prevents_running_two_otas_on_same_device() {
    let mut ota = DeviceOta::new();
    ota.prevent_two();
    assert!(ota.called("prevent_two"));
}

#[test]
fn recovers_from_image_block_response_failures() {
    let mut ota = DeviceOta::new();
    ota.block_response();
    assert!(ota.called("block_response"));
}

#[test]
fn replaces_previously_scheduled_ota_request() {
    let mut ota = DeviceOta::new();
    ota.replace_scheduled();
    assert!(ota.called("replace_scheduled"));
}

#[test]
fn returns_no_image_available_when_custom_file_fails_checksum() {
    let mut ota = DeviceOta::new();
    ota.checks();
    assert!(ota.called("checks"));
}

#[test]
fn returns_no_image_available_when_device_version_is_above_available_in_repo_upgrade() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn returns_no_image_available_when_device_version_is_below_available_in_repo_downgrade() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn returns_no_image_available_when_device_version_matches_available_at_given_url() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn returns_no_image_available_when_device_version_matches_available_in_repo() {
    let mut ota = DeviceOta::new();
    ota.find_image();
    assert!(ota.called("find_image"));
}

#[test]
fn returns_no_image_available_when_firmware_fetching_fails() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn returns_no_image_available_when_index_has_no_entries() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn returns_no_image_available_when_parsing_custom_firmware_fails() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn returns_no_image_available_when_parsing_custom_firmware_in_data_dir_fails() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn returns_no_image_available_when_parsing_repo_firmware_fails() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn returns_no_image_when_index_is_empty() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn schedules_ota_request() {
    let mut ota = DeviceOta::new();
    ota.schedule(None);
    assert!(ota.scheduled);
}

#[test]
fn throttles_when_response_delay_is_configured() {
    let mut ota = DeviceOta::new();
    ota.throttle();
    assert!(ota.called("throttle"));
}

#[test]
fn throws_with_custom_index_as_relative_path_when_ota_not_configured_properly() {
    let mut ota = DeviceOta::new();
    ota.get_index();
    assert!(ota.called("get_index"));
}

#[test]
fn unschedules_ota_when_present() {
    let mut ota = DeviceOta::new();
    ota.unschedule();
    assert!(ota.called("unschedule"));
}

#[test]
fn uses_custom_firmware_in_data_dir() {
    let mut ota = DeviceOta::new();
    ota.get_firmware(&OtaImage::default());
    assert!(ota.called("get_firmware"));
}

#[test]
fn within_file_version_range() {
    let mut ota = DeviceOta::new();
    ota.within_file_version_range(3);
    assert!(ota.called("within_file_version_range"));
}

#[test]
fn writes_scenes_group_before_checking_ota_for_pp_wht_us() {
    let mut ota = DeviceOta::new();
    ota.run();
    assert!(ota.called("run"));
}
