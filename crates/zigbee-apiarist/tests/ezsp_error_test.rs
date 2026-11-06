// Port of .upstream/zigbee-herdsman/test/adapter/ember/ezspError.test.ts

use zigbee_apiarist::adapter::ember::ezsp_error::EzspError;
use zigbee_apiarist::adapter::ezsp::driver::types::basic::EzspStatus;

#[test]
#[ignore = "not implemented yet"]
fn creates_error() {
    let error = EzspError::new(EzspStatus::ASH_ACK_TIMEOUT);

    assert_eq!(error.message(), "ASH_ACK_TIMEOUT");
    assert_eq!(error.code, EzspStatus::ASH_ACK_TIMEOUT);
}
