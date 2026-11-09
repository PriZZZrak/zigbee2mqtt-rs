// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/constants.json.

use serde::Deserialize;
use zigbee_apiarist::adapter::z_stack::constants::utils;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    code: Option<u8>,
    channels: Option<Vec<u8>>,
    expected: serde_json::Value,
}

#[test]
fn constants_match_upstream_fixtures() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/constants.json")).expect("constants fixtures");

    for case in cases {
        match case.op.as_str() {
            "status_description" => {
                let actual = utils::status_description(case.code.expect("code"));
                assert_eq!(serde_json::json!(actual), case.expected, "{}", case.id);
            }
            "channel_mask" => {
                let actual = utils::get_channel_mask(&case.channels.expect("channels"));
                assert_eq!(serde_json::json!(actual), case.expected, "{}", case.id);
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
