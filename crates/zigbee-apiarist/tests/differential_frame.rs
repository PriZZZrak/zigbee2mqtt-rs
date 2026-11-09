// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/frame.json.
//
// The expected values come from the pinned Node implementation, not from the
// Rust port. Regenerate with:
//   cd tools/oracle && npm install && npm run generate

use serde::Deserialize;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    subsystem: Option<String>,
    #[serde(rename = "commandId")]
    command_id: Option<u8>,
    data: Option<String>,
    length: Option<u8>,
    #[serde(rename = "fcsPosition")]
    fcs_position: Option<usize>,
    buffer: Option<String>,
    expected: Option<String>,
    error: Option<String>,
}

fn unhex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_type(name: &str) -> Type {
    match name {
        "POLL" => Type::POLL,
        "SREQ" => Type::SREQ,
        "AREQ" => Type::AREQ,
        "SRSP" => Type::SRSP,
        other => panic!("unknown UNPI type '{other}'"),
    }
}

fn parse_subsystem(name: &str) -> Subsystem {
    match name {
        "RESERVED" => Subsystem::RESERVED,
        "SYS" => Subsystem::SYS,
        "MAC" => Subsystem::MAC,
        "NWK" => Subsystem::NWK,
        "AF" => Subsystem::AF,
        "ZDO" => Subsystem::ZDO,
        "SAPI" => Subsystem::SAPI,
        "UTIL" => Subsystem::UTIL,
        "DEBUG" => Subsystem::DEBUG,
        "APP" => Subsystem::APP,
        "APP_CNF" => Subsystem::APP_CNF,
        "GREENPOWER" => Subsystem::GREENPOWER,
        other => panic!("unknown UNPI subsystem '{other}'"),
    }
}

#[test]
fn frame_matches_upstream_fixtures() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/frame.json")).expect("frame fixtures");

    for case in cases {
        match case.op.as_str() {
            "to_buffer" => {
                let frame = Frame::new(
                    parse_type(case.kind.as_deref().expect("type")),
                    parse_subsystem(case.subsystem.as_deref().expect("subsystem")),
                    case.command_id.expect("commandId"),
                    unhex(case.data.as_deref().unwrap_or_default()),
                    0,
                    0,
                );
                assert_eq!(
                    hex(&frame.to_buffer()),
                    case.expected.expect("expected"),
                    "{}",
                    case.id
                );
            }
            "from_buffer" => {
                let frame = Frame::from_buffer(
                    case.length.expect("length"),
                    case.fcs_position.expect("fcsPosition"),
                    &unhex(case.buffer.as_deref().expect("buffer")),
                );
                match (frame, case.expected, case.error) {
                    (Some(frame), Some(expected), _) => {
                        assert_eq!(hex(&frame.to_buffer()), expected, "{}", case.id);
                    }
                    (None, _, Some(error)) => {
                        assert_eq!(error, "Invalid checksum", "{}", case.id);
                    }
                    other => panic!("{}: unexpected result {other:?}", case.id),
                }
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
