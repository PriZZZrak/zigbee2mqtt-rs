// Original: .upstream/zigbee-herdsman/test/controller.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Adapter disconnected event
//   - Adapter permitJoin fails during stop
//   - Adapter stop fails after adapter disconnected
//   - Add endpoint to group
//   - Add install code 16 byte - missing CRC is appended
//   - Add install code 18 byte
//   - Add install code Aqara
//   - Add install code Hue
//   - Add install code invalid
//   - Add install code pipe
//   - Add install code pipe-enclosed
//   - Add install code unknown format
//   - Add install code widely adopted format
//   - Allow to override read response through `device.customReadResponse
//   - Bind error
//   - Call controller constructor error on invalid channel
//   - Call controller constructor error when extendedPanID is too long
//   - Call controller constructor error when network key too small
//   - Call controller constructor error with invalid panID
//   - Call controller constructor options mixed with default options
//   - Change channel on start
//   - Change channel on start when nwkUpdateID is 0xff
//   - Command response
//   - Configure reporting endpoint custom attributes
//   - ConfigureReporting error
//   - Control4 device join (genBasic reads fail, interview completes via quirk)
//   - Controller
//   - Controller permit joining all, disabled automatically
//   - Controller permit joining all, disabled manually
//   - Controller permit joining for specific time
//   - Controller permit joining for too long time throws
//   - Controller permit joining through specific device
//   - Controller should ignore touchlink messages
//   - Controller start
//   - Controller stop, should create backup
//   - Controller update ieeeAddr if changed
//   - DefaultResponse error
//   - DefaultResponse error when transactionSequenceNumber provided through options
//   - Device LQI table fails
//   - Device LQI table with more than 1 request
//   - Device active endpoints fails
//   - Device announce event
//   - Device announce event should update network address when different
//   - Device binding table
//   - Device binding table fails
//   - Device binding table with more than 1 request
//   - Device clear all bindings fails
//   - Device clears all bindings
//   - Device joins and interview fails
//   - Device joins and interview iAs enrollment fails
//   - Device joins and interview iAs enrollment succeeds
//   - Device joins and interview iAs enrollment throws
//   - Device joins and interview succeeds
//   - Device joins another time with different network address
//   - Device joins with endpoints [2,1], as 2 is the only endpoint supporting genBasic it should read modelID from that
//   - Device joins with endpoints [4,1], should read modelID from 1
//   - Device joins, shouldnt enroll when already enrolled
//   - Device leave event and remove from database
//   - Device leave event with only nwk addr and remove from database
//   - Device lqi
//   - Device network address changed while Z2M was offline - fails to retrieve new one
//   - Device network address changed while Z2M was offline, no duplicate triggering of IEEE request
//   - Device network address changed while Z2M was offline, no spamming of IEEE request when device doesnt respond
//   - Device network address changed while Z2M was offline, received no notification on start
//   - Device network address changed while Z2M was running, received no notification
//   - Device node descriptor fails
//   - Device ping
//   - Device remove from network fails
//   - Device requests network address - changed
//   - Device requests network address - unchanged
//   - Device routing table
//   - Device routing table fails
//   - Device routing table with more than 1 request
//   - Device selectively clears multi-target bindings
//   - Device selectively clears target bindings
//   - Device should update properties when reported
//   - Device updates endpoint bindings from binding table
//   - Device updates existing endpoint bindings from binding table
//   - Device without meta should set meta to {}
//   - Does not add endpoint to group when Add Group response reports failure
//   - Does not change channel on start if not changed
//   - Does not remove endpoint from group when Remove Group response reports failure
//   - Emit lastSeenChanged event even when no message is emitted from it
//   - Emit read from device
//   - Emit write from device
//   - Endpoint addBinding
//   - Endpoint bind
//   - Endpoint command error
//   - Endpoint command with duplicate cluster ID
//   - Endpoint command with duplicate identifier
//   - Endpoint command with options
//   - Endpoint commandResponse
//   - Endpoint commandResponse error
//   - Endpoint commandResponse error when transactionSequenceNumber provided through options
//   - Endpoint configure reporting
//   - Endpoint configure reporting fails when status code is not 0
//   - Endpoint configure reporting fails when status code is not 0 default rsp
//   - Endpoint configure reporting for manufacturer specific attribute
//   - Endpoint configure reporting for manufacturer specific attribute from definition
//   - Endpoint configure reporting with disable response
//   - Endpoint get binds non-existing device
//   - Endpoint get id
//   - Endpoint get id by endpoint device type
//   - Endpoint ignores unknown attributes for read reporting config
//   - Endpoint manuf-spec read reporting config
//   - Endpoint read reporting config
//   - Endpoint simple descriptor fails
//   - Endpoint throws without response to read reporting config
//   - Endpoint unbind
//   - Existing database.tmp should not be overwritten
//   - Get coordinator version
//   - Get device should return same instance
//   - Get input/ouptut clusters
//   - Get network parameters
//   - Gledopto GL-C-007/GL-C-008 join (all endpoints support genBasic but only 12 responds)
//   - Green power
//   - Green power channel request
//   - Green power rxOnCap
//   - Green power unicast
//   - Group addBinding
//   - Group addBinding by number (should create group)
//   - Group bind
//   - Group bind by number (should create group)
//   - Group command
//   - Group command error
//   - Group unbind
//   - Group unbind by number
//   - Group unbind by number, non-existing group and non-existing bind with force=true
//   - Handle retransmitted Xiaomi messages
//   - Handle unbind with number not matching any group
//   - Hard reset
//   - IEEE address event from unknown device
//   - IEEE address event should update network address when different
//   - IEEE address event should update the last seen value
//   - IEEE address event shouldnt update network address when the same
//   - Implicit checkin while send already in progress
//   - Interview on coordinator
//   - Iterates over all devices
//   - Iterates over all groups
//   - Iterates over devices with predicate
//   - Iterates over groups with predicate
//   - Join a device
//   - Join a device and explictly accept it
//   - Join a device and explictly refuses it
//   - Join a device and explictly refuses it but LEAVE request fails
//   - Load database
//   - Load database: interviewState migration and reset IN_PROGRESS
//   - Network address event from unknown device
//   - Network address event should update network address when different
//   - Network address event should update the last seen value
//   - Network address event shouldnt update network address when the same
//   - Node Descriptor on R pre-21 device
//   - Node Descriptor on R21 device
//   - Poll control supported
//   - Poll control unsupported
//   - Read error
//   - Read from endpoint unknown attribute with options
//   - Read from endpoint with custom attribute
//   - Read from endpoint with string
//   - Read from endpoint with string ignores unknown
//   - Read from group
//   - Read from group fails
//   - Read from group ignores unknown attributes
//   - Read response to endpoint throw when transaction sequence number provided through options
//   - Read response to endpoint with non ZCL attribute
//   - Read response to endpoint with unknown string attribute
//   - Read with disable response
//   - ReadResponse error
//   - Receive cluster command
//   - Receive cluster command from unknown cluster
//   - Receive raw data
//   - Receive raw data from unknown cluster
//   - Receive readResponse from unknown endpoint
//   - Receive zclData dont send default resopnse when broadcast
//   - Receive zclData dont send default resopnse with skipDefaultResponse
//   - Receive zclData from unkonwn device shouldnt emit anything
//   - Receive zclData occupancy report
//   - Receive zclData report from unkown attribute
//   - Receive zclData send default response
//   - Receive zclData send default response fails should NOT attempt route discover when adapter does not support it
//   - Receive zclData with device deleted during processing (prevent race condition)
//   - Remove endpoint from all groups
//   - Remove endpoint from group
//   - Remove endpoint from group by number
//   - Removes device from database and clears cache
//   - Removes device from database but keep in cache
//   - Removes device from network and clears cache
//   - Removes device from network and database but keep in cache
//   - Removes endpoint from group database when Remove Group response reports NOT_FOUND
//   - Removes every member when removing a group from the network
//   - Removes group from database
//   - Removes group from network and database
//   - Report error
//   - Report to endpoint custom attributes
//   - Report to endpoint with unknown string attribute
//   - Respond to genTime read
//   - Respond to genTime read fails
//   - Respond to read of attribute
//   - Return device from databse when not in lookup
//   - Return group from databse when not in lookup
//   - Save endpoint configure reporting
//   - Send to device with pendingRequestTimeout > 0, fails on non-timeout error
//   - Send to device with pendingRequestTimeout > 0, retry on timeout error
//   - Send zcl command to all no options
//   - Send zcl command to all with manufacturer option
//   - Sends read response for unsupported attribute
//   - Set device powersource by string
//   - Should allow to set type
//   - Should allow to specify custom attributes for existing cluster
//   - Should allow to specify custom cluster
//   - Should allow to specify custom cluster as override for Zcl cluster
//   - Should create backup of databse before clearing when datbaseBackupPath is provided
//   - Should do a coordinator check
//   - Should drop non-success records from readRsp payloads
//   - Should handle green power broadcast commissioning
//   - Should handle green power commissioning frame with IEEE addressing gracefully
//   - Should handle green power commissioning frame with switch info
//   - Should handle green power responses gracefully
//   - Should handle green power unicast commissioning failure
//   - Should handle green power unicast commissioning success
//   - Should ignore invalid green power frame
//   - Should remove disappeared endpoints on updateActiveEndpoints
//   - Should replace legacy configured reportings without manufacturerCode
//   - Should resolve generic cluster attribute names on generic ZCL frames: generic target device
//   - Should resolve manufacturer specific cluster attribute names on generic ZCL frames: Legrand target device
//   - Should resolve manufacturer specific cluster attribute names on specific ZCL frames: generic target device
//   - Should resolve manufacturer specific cluster attribute names on specific ZCL frames: specific target device
//   - Should roll-over transaction ID
//   - Should save received attributes
//   - Should throw datbase basic crud errors
//   - Should use cached node descriptor when device is re-interviewed, but retrieve it when ignoreCache=true
//   - Shouldnt create backup when adapter doesnt support it
//   - Shouldnt load device from group databaseentry
//   - Shouldnt throw error on coordinatorCheck when adapter doesnt support backups
//   - Skip Device announce event from unknown device
//   - Skip unbind if not bound
//   - Soft reset
//   - Start with reset should clear database
//   - Syncs runtime lookups
//   - Throw error when creating endpoint which already exists
//   - Throw error when creating group already exists
//   - Throw error when device with IEEE address already exists
//   - Throw error when interviewing and calling interview again
//   - Touchlink factory reset
//   - Touchlink factory reset first
//   - Touchlink fails to stop during stop
//   - Touchlink identify
//   - Touchlink lock
//   - Touchlink scan
//   - Touchlink stops during stop
//   - Try to get deleted device from endpoint
//   - Tuya end devices joins, stops responding after 1 requests, should read modelID and manufacturerName immediately on second pair
//   - Unbind against unbound cluster
//   - Unbind error
//   - Updates a device genBasic properties
//   - Write error
//   - Write response to endpoint throw when transaction sequence number provided through options
//   - Write response to endpoint with ZCL attribute
//   - Write response to endpoint with no status attribute specified
//   - Write response to endpoint with non ZCL attribute
//   - Write response to endpoint with options
//   - Write response to endpoint with unknown string attribute
//   - Write structured
//   - Write structured error
//   - Write structured with disable response
//   - Write to device with pendingRequestTimeout > 0
//   - Write to device with pendingRequestTimeout > 0, discard messages after expiration
//   - Write to device with pendingRequestTimeout > 0, error
//   - Write to device with pendingRequestTimeout > 0, override default sendPolicy
//   - Write to device with pendingRequestTimeout > 0, replace queued messages
//   - Write to device with pendingRequestTimeout > 0, send bulk messages
//   - Write to endpoint custom attributes
//   - Write to endpoint custom attributes without specifying manufacturerCode
//   - Write to endpoint with unknown string attribute
//   - Write to group
//   - Write to group fails
//   - Write to group with unknown attribute should fail
//   - Write with custom payload
//   - Write with disable response
//   - WriteResponse error
//   - WriteUndiv to endpoint custom attributes without response
//   - Xiaomi WXCJKG11LM join (get simple descriptor for endpoint 2 fails)
//   - Xiaomi end device joins (node descriptor fails)
//   - Xiaomi end device joins (node descriptor succeeds, but active endpoint response fails)
//   - ZCL frame converter attributeKeyValue
//   - ZCL frame converter attributeList
//   - ZDO response for END_DEVICE_ANNOUNCE from unknown device
//   - ZDO response for END_DEVICE_ANNOUNCE should bubble up event
//   - ZDO response for END_DEVICE_ANNOUNCE should update network address when different
//   - ZDO response for NETWORK_ADDRESS_RESPONSE from unknown device
//   - ZDO response for NETWORK_ADDRESS_RESPONSE should update network address when different
//   - ZDO response for NETWORK_ADDRESS_RESPONSE should update the last seen value
//   - ZDO response for NETWORK_ADDRESS_RESPONSE shouldnt update network address when the same
//   - ZDO response for cluster ID with no extra processing
//   - \n
//   - aborts start on signal
//   - controller should backup at defined interval
//   - controller should not crash on failed interval backup
//   - does not read/write to group with non-common custom clusters
//   - does not try to identify unknown device when deleted
//   - ensures manufacturer code is always unique in a single call
//   - handles configure reporting with non-Analog data type
//   - ignores invalid attributes when processing ZCL attribute
//   - merges added custom clusters definitions
//   - reads/writes to group with custom cluster when common to all members
//   - sends & receives command to group with custom cluster when common to all members
//   - throws when trying to configure reporting on endpoint with bad attribute
//   - triggers sendZclFrameInterPANBroadcast on sendRaw
//   - triggers sendZclFrameInterPANToIeeeAddr on sendRaw
//   - triggers sendZclFrameToAll on sendRaw
//   - triggers sendZclFrameToEndpoint on sendRaw
//   - triggers sendZclFrameToEndpoint with custom cluster on sendRaw
//   - triggers sendZclFrameToGroup on sendRaw
//   - triggers sendZdo on sendRaw
//   - zclCommand
//   - zclCommand with cluster/command objects
//   - zclCommand with error

#[test]
#[ignore = "not implemented yet"]
fn adapter_disconnected_event() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_permit_join_fails_during_stop() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_stop_fails_after_adapter_disconnected() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_endpoint_to_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_16_byte_missing_crc_is_appended() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_18_byte() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_aqara() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_hue() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_invalid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_pipe() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_pipe_enclosed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_unknown_format() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_widely_adopted_format() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn allow_to_override_read_response_through_device_custom_read_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn bind_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_controller_constructor_error_on_invalid_channel() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_controller_constructor_error_when_extended_pan_id_is_too_long() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_controller_constructor_error_when_network_key_too_small() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_controller_constructor_error_with_invalid_pan_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_controller_constructor_options_mixed_with_default_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn change_channel_on_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn change_channel_on_start_when_nwk_update_id_is_0xff() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn command_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configure_reporting_endpoint_custom_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configure_reporting_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn control4_device_join_gen_basic_reads_fail_interview_completes_via_quirk() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_permit_joining_all_disabled_automatically() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_permit_joining_all_disabled_manually() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_permit_joining_for_specific_time() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_permit_joining_for_too_long_time_throws() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_permit_joining_through_specific_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_should_ignore_touchlink_messages() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_stop_should_create_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_update_ieee_addr_if_changed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn default_response_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn default_response_error_when_transaction_sequence_number_provided_through_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_lqi_table_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_lqi_table_with_more_than_1_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_active_endpoints_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_announce_event() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_announce_event_should_update_network_address_when_different() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_binding_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_binding_table_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_binding_table_with_more_than_1_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_clear_all_bindings_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_clears_all_bindings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_and_interview_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_and_interview_i_as_enrollment_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_and_interview_i_as_enrollment_succeeds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_and_interview_i_as_enrollment_throws() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_and_interview_succeeds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_another_time_with_different_network_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_with_endpoints_2_1_as_2_is_the_only_endpoint_supporting_gen_basic_it_should_read_model_id_from_that(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_with_endpoints_4_1_should_read_model_id_from_1() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joins_shouldnt_enroll_when_already_enrolled() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_leave_event_and_remove_from_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_leave_event_with_only_nwk_addr_and_remove_from_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_lqi() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_network_address_changed_while_z2_m_was_offline_fails_to_retrieve_new_one() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_network_address_changed_while_z2_m_was_offline_no_duplicate_triggering_of_ieee_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_network_address_changed_while_z2_m_was_offline_no_spamming_of_ieee_request_when_device_doesnt_respond(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_network_address_changed_while_z2_m_was_offline_received_no_notification_on_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_network_address_changed_while_z2_m_was_running_received_no_notification() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_node_descriptor_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_ping() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_remove_from_network_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_requests_network_address_changed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_requests_network_address_unchanged() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_routing_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_routing_table_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_routing_table_with_more_than_1_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_selectively_clears_multi_target_bindings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_selectively_clears_target_bindings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_should_update_properties_when_reported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_updates_endpoint_bindings_from_binding_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_updates_existing_endpoint_bindings_from_binding_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_without_meta_should_set_meta_to() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_add_endpoint_to_group_when_add_group_response_reports_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_change_channel_on_start_if_not_changed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_remove_endpoint_from_group_when_remove_group_response_reports_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emit_last_seen_changed_event_even_when_no_message_is_emitted_from_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emit_read_from_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emit_write_from_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_add_binding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_bind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_with_duplicate_cluster_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_with_duplicate_identifier() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_with_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_response_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_command_response_error_when_transaction_sequence_number_provided_through_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting_fails_when_status_code_is_not_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting_fails_when_status_code_is_not_0_default_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting_for_manufacturer_specific_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting_for_manufacturer_specific_attribute_from_definition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_configure_reporting_with_disable_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_get_binds_non_existing_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_get_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_get_id_by_endpoint_device_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_ignores_unknown_attributes_for_read_reporting_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_manuf_spec_read_reporting_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_read_reporting_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_simple_descriptor_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_throws_without_response_to_read_reporting_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn endpoint_unbind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn existing_database_tmp_should_not_be_overwritten() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_coordinator_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_device_should_return_same_instance() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_input_ouptut_clusters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_network_parameters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn gledopto_gl_c_007_gl_c_008_join_all_endpoints_support_gen_basic_but_only_12_responds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn green_power() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn green_power_channel_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn green_power_rx_on_cap() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn green_power_unicast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_add_binding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_add_binding_by_number_should_create_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_bind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_bind_by_number_should_create_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_command_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_unbind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_unbind_by_number() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn group_unbind_by_number_non_existing_group_and_non_existing_bind_with_force_true() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handle_retransmitted_xiaomi_messages() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handle_unbind_with_number_not_matching_any_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hard_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_address_event_from_unknown_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_address_event_should_update_network_address_when_different() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_address_event_should_update_the_last_seen_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_address_event_shouldnt_update_network_address_when_the_same() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn implicit_checkin_while_send_already_in_progress() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn interview_on_coordinator() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn iterates_over_all_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn iterates_over_all_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn iterates_over_devices_with_predicate() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn iterates_over_groups_with_predicate() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn join_a_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn join_a_device_and_explictly_accept_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn join_a_device_and_explictly_refuses_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn join_a_device_and_explictly_refuses_it_but_leave_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn load_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn load_database_interview_state_migration_and_reset_in_progress() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn network_address_event_from_unknown_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn network_address_event_should_update_network_address_when_different() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn network_address_event_should_update_the_last_seen_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn network_address_event_shouldnt_update_network_address_when_the_same() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn node_descriptor_on_r_pre_21_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn node_descriptor_on_r21_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn poll_control_supported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn poll_control_unsupported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_endpoint_unknown_attribute_with_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_endpoint_with_custom_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_endpoint_with_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_endpoint_with_string_ignores_unknown() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_group_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_from_group_ignores_unknown_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_response_to_endpoint_throw_when_transaction_sequence_number_provided_through_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_response_to_endpoint_with_non_zcl_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_response_to_endpoint_with_unknown_string_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_with_disable_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_response_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_cluster_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_cluster_command_from_unknown_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_raw_data() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_raw_data_from_unknown_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_read_response_from_unknown_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_dont_send_default_resopnse_when_broadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_dont_send_default_resopnse_with_skip_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_from_unkonwn_device_shouldnt_emit_anything() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_occupancy_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_report_from_unkown_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_send_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_send_default_response_fails_should_not_attempt_route_discover_when_adapter_does_not_support_it(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn receive_zcl_data_with_device_deleted_during_processing_prevent_race_condition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn remove_endpoint_from_all_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn remove_endpoint_from_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn remove_endpoint_from_group_by_number() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_device_from_database_and_clears_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_device_from_database_but_keep_in_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_device_from_network_and_clears_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_device_from_network_and_database_but_keep_in_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_endpoint_from_group_database_when_remove_group_response_reports_not_found() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_every_member_when_removing_a_group_from_the_network() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_group_from_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_group_from_network_and_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn report_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn report_to_endpoint_custom_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn report_to_endpoint_with_unknown_string_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn respond_to_gen_time_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn respond_to_gen_time_read_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn respond_to_read_of_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn return_device_from_databse_when_not_in_lookup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn return_group_from_databse_when_not_in_lookup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn save_endpoint_configure_reporting() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_to_device_with_pending_request_timeout_0_fails_on_non_timeout_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_to_device_with_pending_request_timeout_0_retry_on_timeout_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_command_to_all_no_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_command_to_all_with_manufacturer_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_read_response_for_unsupported_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn set_device_powersource_by_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_set_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_specify_custom_attributes_for_existing_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_specify_custom_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_specify_custom_cluster_as_override_for_zcl_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_create_backup_of_databse_before_clearing_when_datbase_backup_path_is_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_do_a_coordinator_check() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_drop_non_success_records_from_read_rsp_payloads() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_broadcast_commissioning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_commissioning_frame_with_ieee_addressing_gracefully() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_commissioning_frame_with_switch_info() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_responses_gracefully() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_unicast_commissioning_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_green_power_unicast_commissioning_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_ignore_invalid_green_power_frame() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_remove_disappeared_endpoints_on_update_active_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_replace_legacy_configured_reportings_without_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_resolve_generic_cluster_attribute_names_on_generic_zcl_frames_generic_target_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_generic_zcl_frames_legrand_target_device(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_specific_zcl_frames_generic_target_device(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_specific_zcl_frames_specific_target_device(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_roll_over_transaction_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_save_received_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_datbase_basic_crud_errors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_cached_node_descriptor_when_device_is_re_interviewed_but_retrieve_it_when_ignore_cache_true(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_create_backup_when_adapter_doesnt_support_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_load_device_from_group_databaseentry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_throw_error_on_coordinator_check_when_adapter_doesnt_support_backups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn skip_device_announce_event_from_unknown_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn skip_unbind_if_not_bound() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn soft_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_with_reset_should_clear_database() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn syncs_runtime_lookups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throw_error_when_creating_endpoint_which_already_exists() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throw_error_when_creating_group_already_exists() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throw_error_when_device_with_ieee_address_already_exists() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throw_error_when_interviewing_and_calling_interview_again() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_factory_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_factory_reset_first() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_fails_to_stop_during_stop() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_identify() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_lock() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_scan() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_stops_during_stop() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn try_to_get_deleted_device_from_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn tuya_end_devices_joins_stops_responding_after_1_requests_should_read_model_id_and_manufacturer_name_immediately_on_second_pair(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn unbind_against_unbound_cluster() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn unbind_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn updates_a_device_gen_basic_properties() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_throw_when_transaction_sequence_number_provided_through_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_with_zcl_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_with_no_status_attribute_specified() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_with_non_zcl_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_with_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_to_endpoint_with_unknown_string_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_structured() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_structured_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_structured_with_disable_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0_discard_messages_after_expiration() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0_override_default_send_policy() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0_replace_queued_messages() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_device_with_pending_request_timeout_0_send_bulk_messages() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_endpoint_custom_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_endpoint_custom_attributes_without_specifying_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_endpoint_with_unknown_string_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_group_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_to_group_with_unknown_attribute_should_fail() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_with_custom_payload() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_with_disable_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_response_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn write_undiv_to_endpoint_custom_attributes_without_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn xiaomi_wxcjkg11_lm_join_get_simple_descriptor_for_endpoint_2_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn xiaomi_end_device_joins_node_descriptor_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn xiaomi_end_device_joins_node_descriptor_succeeds_but_active_endpoint_response_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_converter_attribute_key_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_converter_attribute_list() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_end_device_announce_from_unknown_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_end_device_announce_should_bubble_up_event() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_end_device_announce_should_update_network_address_when_different() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_network_address_response_from_unknown_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_network_address_response_should_update_network_address_when_different() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_network_address_response_should_update_the_last_seen_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_network_address_response_shouldnt_update_network_address_when_the_same() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response_for_cluster_id_with_no_extra_processing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn n() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn aborts_start_on_signal() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_should_backup_at_defined_interval() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn controller_should_not_crash_on_failed_interval_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_read_write_to_group_with_non_common_custom_clusters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_try_to_identify_unknown_device_when_deleted() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ensures_manufacturer_code_is_always_unique_in_a_single_call() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_configure_reporting_with_non_analog_data_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_invalid_attributes_when_processing_zcl_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn merges_added_custom_clusters_definitions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_writes_to_group_with_custom_cluster_when_common_to_all_members() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_receives_command_to_group_with_custom_cluster_when_common_to_all_members() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_trying_to_configure_reporting_on_endpoint_with_bad_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_inter_panbroadcast_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_inter_panto_ieee_addr_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_to_all_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_to_endpoint_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_to_endpoint_with_custom_cluster_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zcl_frame_to_group_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_send_zdo_on_send_raw() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_command_with_cluster_command_objects() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_command_with_error() {
    todo!()
}
