// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/zpi_object.json.

use serde::Deserialize;
use serde_json::Value;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    subsystem: String,
    command: Option<String>,
    payload: Option<Value>,
    #[serde(rename = "frameType")]
    frame_type: Option<String>,
    #[serde(rename = "commandId")]
    command_id: Option<u8>,
    data: Option<String>,
    expected: Value,
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

fn unhex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn zpi_object_matches_upstream_fixtures() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/zpi_object.json"))
        .expect("zpi_object fixtures");

    for case in cases {
        match case.op.as_str() {
            "create_request" => {
                let object = ZpiObject::create_request(
                    parse_subsystem(&case.subsystem),
                    case.command.as_deref().expect("command"),
                    case.payload.clone().unwrap_or(Value::Null),
                );
                assert_eq!(
                    hex(&object.to_unpi_frame().to_buffer()),
                    case.expected.as_str().expect("expected"),
                    "{}",
                    case.id
                );
            }
            "from_frame" => {
                let frame = Frame::new(
                    parse_type(case.frame_type.as_deref().expect("frameType")),
                    parse_subsystem(&case.subsystem),
                    case.command_id.expect("commandId"),
                    unhex(case.data.as_deref().unwrap_or_default()),
                    0,
                    0,
                );
                let object = ZpiObject::from_unpi_frame(&frame);
                assert_eq!(object.payload, case.expected, "{}", case.id);
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
