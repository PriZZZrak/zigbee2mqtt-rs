// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/buffalo.json.

use serde::Deserialize;
use serde_json::{json, Value};
use zigbee_apiarist::buffalo::buffalo::Buffalo;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    #[serde(rename = "type")]
    kind: String,
    value: Option<Value>,
    buffer: Option<String>,
    length: Option<usize>,
    expected: Value,
    position: Option<usize>,
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

fn number(value: &Value) -> u64 {
    value.as_u64().expect("unsigned value")
}

fn signed(value: &Value) -> i64 {
    value.as_i64().expect("signed value")
}

fn float(value: &Value) -> f64 {
    value.as_f64().expect("float value")
}

#[test]
fn buffalo_matches_upstream_fixtures() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/buffalo.json")).expect("buffalo fixtures");

    for case in cases {
        match case.op.as_str() {
            "write" => {
                let mut buffalo = Buffalo::new(vec![0; 250], 0);
                let value = case.value.as_ref().expect("value");
                match case.kind.as_str() {
                    "uint8" => buffalo.write_uint8(number(value)),
                    "uint16" => buffalo.write_uint16(number(value)),
                    "uint24" => buffalo.write_uint24(number(value)),
                    "uint32" => buffalo.write_uint32(number(value)),
                    "int8" => buffalo.write_int8(signed(value)),
                    "int16" => buffalo.write_int16(signed(value)),
                    "int24" => buffalo.write_int24(signed(value)),
                    "int32" => buffalo.write_int32(signed(value)),
                    "floatle" => buffalo.write_float_le(float(value)),
                    "doublele" => buffalo.write_double_le(float(value)),
                    "ieeeaddr" => buffalo.write_ieee_addr(value.as_str().expect("ieeeaddr")),
                    "buffer" => buffalo.write_buffer(
                        &unhex(value.as_str().expect("buffer")),
                        case.length.expect("length"),
                    ),
                    other => panic!("{}: unsupported write type '{other}'", case.id),
                }

                assert_eq!(
                    json!(hex(&buffalo.get_written())),
                    case.expected,
                    "{}",
                    case.id
                );
                assert_eq!(
                    buffalo.get_position(),
                    case.position.expect("position"),
                    "{}",
                    case.id
                );
            }
            "read" => {
                let mut buffalo = Buffalo::new(unhex(case.buffer.as_deref().expect("buffer")), 0);
                let value = match case.kind.as_str() {
                    "uint8" => json!(buffalo.read_uint8()),
                    "uint16" => json!(buffalo.read_uint16()),
                    "uint24" => json!(buffalo.read_uint24()),
                    "uint32" => json!(buffalo.read_uint32()),
                    "int8" => json!(buffalo.read_int8()),
                    "int16" => json!(buffalo.read_int16()),
                    "int24" => json!(buffalo.read_int24()),
                    "int32" => json!(buffalo.read_int32()),
                    "uint64" => json!({"bigint": buffalo.read_uint64().to_string()}),
                    "int64" => json!({"bigint": buffalo.read_int64().to_string()}),
                    "floatle" => json!(buffalo.read_float_le()),
                    "doublele" => json!(buffalo.read_double_le()),
                    "ieeeaddr" => json!(buffalo.read_ieee_addr()),
                    "buffer" => json!(hex(&buffalo.read_buffer(case.length.expect("length")))),
                    other => panic!("{}: unsupported read type '{other}'", case.id),
                };

                assert_eq!(value, case.expected, "{}", case.id);
                assert_eq!(
                    buffalo.get_position(),
                    case.position.expect("position"),
                    "{}",
                    case.id
                );
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
