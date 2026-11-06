// Original: .upstream/zigbee-herdsman/test/device-ota.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Checks
//   - Device OTA
//   - Finds image
//   - Schedules / Unschedules
//   - Updates
//   - Utils
//   - aborts an in-progress update
//   - allows bypassing version check with force meta
//   - allows use of local override index even if custom URL index fetching fails
//   - allows use of local override index even if default URL index fetching fails
//   - allows use of remote override index even if URL index fetching fails
//   - applies a downgrade image end-to-end
//   - applies fallback timeouts when manufacturer is unknown
//   - applies manufacturer-specific and imageType-specific timeout
//   - applies manufacturer-specific data size
//   - applies manufacturer-specific timeout
//   - auto-adds meta for override index local firmware when unspecified
//   - by extra meta hardwareVersionMax
//   - by extra meta hardwareVersionMin
//   - by extra meta manufacturerName
//   - by extra meta modelId
//   - by extra meta otaHeaderString
//   - by hardwareVersionMax
//   - by hardwareVersionMin
//   - by manufacturerName
//   - by maxFileVersion
//   - by minFileVersion
//   - by modelId
//   - calls onProgress with non-round blocks
//   - calls onProgress with response delay (changes estimate)
//   - calls onProgress with round blocks
//   - cancels scheduled OTA when completed
//   - considers an upgrade successful even if no device announce
//   - deviceAnnounce
//   - does not throw on failed default response after non-success upgrade end request
//   - fails to abort an in-progress update
//   - fails when check fails
//   - fails when device sends upgrade end request with ABORT after a certain number of blocks
//   - fails when device sends upgrade end request with INVALID_IMAGE after image fully sent
//   - fails when device stops sending data requests
//   - fails when image notify fails
//   - fails when query next image response fails
//   - fails when remote index fetching fails
//   - fails when upgrade end never arrives
//   - fails when upgrade end response fails
//   - handles URL as source URL
//   - handles custom index as absolute path
//   - handles custom index as relative path
//   - handles failures when waiting for OTA notify response
//   - handles filesystem as source URL
//   - handles negative misaligned block offset - resends portion already sent
//   - handles out-of-order block offsets
//   - handles page requests
//   - handles positive misaligned block offset - skips portion
//   - handles receiving default response after block response
//   - handles receiving non-value for data size
//   - handles unscheduling when scheduled not present
//   - keeps scheduled OTA on failure
//   - matches OTA images using extra manufacturer metadata when device manufacturer name mismatches
//   - matches OTA images using extra metadata overrides
//   - matches OTA images using manufacturer name and hardware ranges
//   - merges override index (prio) with URL index
//   - parses firmware with unusual header fields
//   - performs notify flow when current payload is missing
//   - prevents running two OTAs on same device
//   - recovers from image block response failures
//   - replaces previously-scheduled OTA request
//   - returns NO_IMAGE_AVAILABLE when custom file fails checksum
//   - returns NO_IMAGE_AVAILABLE when device version is above available in repo (upgrade)
//   - returns NO_IMAGE_AVAILABLE when device version is below available in repo (downgrade)
//   - returns NO_IMAGE_AVAILABLE when device version matches available at given URL
//   - returns NO_IMAGE_AVAILABLE when device version matches available in repo
//   - returns NO_IMAGE_AVAILABLE when firmware fetching fails
//   - returns NO_IMAGE_AVAILABLE when index has no entries
//   - returns NO_IMAGE_AVAILABLE when parsing custom firmware fails
//   - returns NO_IMAGE_AVAILABLE when parsing custom firmware in dataDir fails
//   - returns NO_IMAGE_AVAILABLE when parsing repo firmware fails
//   - returns no image when index is empty
//   - schedules OTA request
//   - throttles when response delay is configured
//   - throws with custom index as relative path when OTA not configured properly
//   - unschedules OTA when present
//   - uses custom firmware in dataDir
//   - within file version range
//   - writes scenes group before checking OTA for PP-WHT-US

#[test]
#[ignore = "not implemented yet"]
fn checks() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_ota() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn finds_image() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn schedules_unschedules() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn updates() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn utils() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn aborts_an_in_progress_update() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn allows_bypassing_version_check_with_force_meta() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn allows_use_of_local_override_index_even_if_custom_url_index_fetching_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn allows_use_of_local_override_index_even_if_default_url_index_fetching_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn allows_use_of_remote_override_index_even_if_url_index_fetching_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn applies_a_downgrade_image_end_to_end() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn applies_fallback_timeouts_when_manufacturer_is_unknown() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn applies_manufacturer_specific_and_image_type_specific_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn applies_manufacturer_specific_data_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn applies_manufacturer_specific_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn auto_adds_meta_for_override_index_local_firmware_when_unspecified() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_extra_meta_hardware_version_max() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_extra_meta_hardware_version_min() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_extra_meta_manufacturer_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_extra_meta_model_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_extra_meta_ota_header_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_hardware_version_max() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_hardware_version_min() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_manufacturer_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_max_file_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_min_file_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn by_model_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn calls_on_progress_with_non_round_blocks() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn calls_on_progress_with_response_delay_changes_estimate() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn calls_on_progress_with_round_blocks() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn cancels_scheduled_ota_when_completed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn considers_an_upgrade_successful_even_if_no_device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_throw_on_failed_default_response_after_non_success_upgrade_end_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_abort_an_in_progress_update() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_check_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_device_sends_upgrade_end_request_with_abort_after_a_certain_number_of_blocks() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_device_sends_upgrade_end_request_with_invalid_image_after_image_fully_sent() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_device_stops_sending_data_requests() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_image_notify_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_query_next_image_response_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_remote_index_fetching_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_upgrade_end_never_arrives() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_upgrade_end_response_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_url_as_source_url() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_custom_index_as_absolute_path() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_custom_index_as_relative_path() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_failures_when_waiting_for_ota_notify_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_filesystem_as_source_url() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_negative_misaligned_block_offset_resends_portion_already_sent() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_out_of_order_block_offsets() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_page_requests() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_positive_misaligned_block_offset_skips_portion() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_receiving_default_response_after_block_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_receiving_non_value_for_data_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_unscheduling_when_scheduled_not_present() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_scheduled_ota_on_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn matches_ota_images_using_extra_manufacturer_metadata_when_device_manufacturer_name_mismatches() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn matches_ota_images_using_extra_metadata_overrides() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn matches_ota_images_using_manufacturer_name_and_hardware_ranges() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn merges_override_index_prio_with_url_index() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn parses_firmware_with_unusual_header_fields() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn performs_notify_flow_when_current_payload_is_missing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn prevents_running_two_otas_on_same_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn recovers_from_image_block_response_failures() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn replaces_previously_scheduled_ota_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_custom_file_fails_checksum() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_device_version_is_above_available_in_repo_upgrade() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_device_version_is_below_available_in_repo_downgrade() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_device_version_matches_available_at_given_url() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_device_version_matches_available_in_repo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_firmware_fetching_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_index_has_no_entries() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_parsing_custom_firmware_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_parsing_custom_firmware_in_data_dir_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_available_when_parsing_repo_firmware_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_no_image_when_index_is_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn schedules_ota_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throttles_when_response_delay_is_configured() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_with_custom_index_as_relative_path_when_ota_not_configured_properly() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn unschedules_ota_when_present() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_custom_firmware_in_data_dir() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn within_file_version_range() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_scenes_group_before_checking_ota_for_pp_wht_us() {
    todo!()
}
