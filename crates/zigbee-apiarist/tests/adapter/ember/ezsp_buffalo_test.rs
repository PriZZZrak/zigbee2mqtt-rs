// Original: .upstream/zigbee-herdsman/test/adapter/ember/ezspBuffalo.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Ember EZSP Buffalo
//   - Is empty after init
//   - Maps EmberStatus/EzspStatus to SLStatus
//   - Writes & read at position without altering internal position tracker

use zigbee_apiarist::adapter::ember::enums::SLStatus;
use zigbee_apiarist::adapter::ember::ezsp::buffalo::EzspBuffalo;
use zigbee_apiarist::adapter::ember::ezsp::consts::{
    EZSP_EXTENDED_FRAME_CONTROL_LB_INDEX, EZSP_FRAME_CONTROL_COMMAND,
    EZSP_FRAME_CONTROL_NETWORK_INDEX_MASK, EZSP_FRAME_CONTROL_NETWORK_INDEX_OFFSET,
    EZSP_FRAME_CONTROL_SLEEP_MODE_MASK, EZSP_FRAME_ID_INDEX, EZSP_MAX_FRAME_LENGTH,
    EZSP_PARAMETERS_INDEX, EZSP_SEQUENCE_INDEX,
};
use zigbee_apiarist::adapter::ember::ezsp::enums::EzspFrameID;

#[test]
fn is_empty_after_init() {
    let buffalo = EzspBuffalo::new(vec![0; EZSP_MAX_FRAME_LENGTH]);
    assert_eq!(buffalo.get_written(), Vec::<u8>::new());
}

#[test]
fn maps_ember_status_ezsp_status_to_slstatus() {
    let mut buffalo = EzspBuffalo::new(vec![0; EZSP_MAX_FRAME_LENGTH]);
    buffalo.set_command_byte(0, 0x00);
    buffalo.set_command_byte(1, 0x00);
    buffalo.set_command_byte(2, 0x00);
    buffalo.set_command_byte(3, 0x00);
    // zero always zero
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status(0x0d), SLStatus::OK);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status_no_ember(0x0d), SLStatus::OK);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status(0x0e), SLStatus::OK);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status_no_ember(0x0e), SLStatus::OK);

    buffalo.set_command_byte(0, 0x02);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status(0x0d), SLStatus::INVALID_PARAMETER);
    buffalo.set_position(0);
    assert_eq!(
        buffalo.read_status_no_ember(0x0d),
        SLStatus::ZIGBEE_EZSP_ERROR
    );

    buffalo.set_command_byte(0, 0x9c);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status(0x0d), SLStatus::ZIGBEE_NETWORK_OPENED);
    buffalo.set_position(0);
    assert_eq!(
        buffalo.read_status_no_ember(0x0d),
        SLStatus::ZIGBEE_EZSP_ERROR
    );

    // no mapped value
    buffalo.set_command_byte(0, 0x4b);
    buffalo.set_position(0);
    assert_eq!(buffalo.read_status(0x0d), SLStatus::BUS_ERROR);
    buffalo.set_position(0);
    assert_eq!(
        buffalo.read_status_no_ember(0x0d),
        SLStatus::ZIGBEE_EZSP_ERROR
    );
}

#[test]
fn writes_read_at_position_without_altering_internal_position_tracker() {
    let mut buffalo = EzspBuffalo::new(vec![0; EZSP_MAX_FRAME_LENGTH]);
    buffalo.set_position(EZSP_PARAMETERS_INDEX);
    buffalo.set_command_byte(EZSP_FRAME_ID_INDEX, EzspFrameID::VERSION as u8);
    buffalo.set_command_byte(EZSP_SEQUENCE_INDEX, 0);
    let frame_control = EZSP_FRAME_CONTROL_COMMAND
        | (0x00 & EZSP_FRAME_CONTROL_SLEEP_MODE_MASK)
        | ((0x00 << EZSP_FRAME_CONTROL_NETWORK_INDEX_OFFSET)
            & EZSP_FRAME_CONTROL_NETWORK_INDEX_MASK);
    buffalo.set_command_byte(EZSP_EXTENDED_FRAME_CONTROL_LB_INDEX, frame_control);
    buffalo.write_uint8(12); // desiredProtocolVersion

    assert_eq!(buffalo.get_written(), vec![0x00, 0x00, 0x00, 0x0c]);

    assert_eq!(
        buffalo.get_command_byte(EZSP_FRAME_ID_INDEX),
        EzspFrameID::VERSION as u8
    );
    assert_eq!(buffalo.get_command_byte(EZSP_SEQUENCE_INDEX), 0);
    assert_eq!(
        buffalo.get_command_byte(EZSP_EXTENDED_FRAME_CONTROL_LB_INDEX),
        frame_control
    );
}
