// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/buffalo_znp.json.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde::Deserialize;
use serde_json::{json, Value};
use zigbee_apiarist::adapter::z_stack::znp::buffalo_znp::BuffaloZnp;
use zigbee_apiarist::adapter::z_stack::znp::parameter_type::ParameterType;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    #[serde(rename = "type")]
    kind: String,
    value: Option<Value>,
    buffer: Option<String>,
    options: Option<Value>,
    expected: Option<Value>,
    error: Option<String>,
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

fn parameter_type(name: &str) -> u32 {
    match name {
        "UINT8" => ParameterType::UINT8 as u32,
        "UINT16" => ParameterType::UINT16 as u32,
        "UINT32" => ParameterType::UINT32 as u32,
        "IEEEADDR" => ParameterType::IEEEADDR as u32,
        "BUFFER" => ParameterType::BUFFER as u32,
        "BUFFER8" => ParameterType::BUFFER8 as u32,
        "BUFFER16" => ParameterType::BUFFER16 as u32,
        "BUFFER18" => ParameterType::BUFFER18 as u32,
        "BUFFER32" => ParameterType::BUFFER32 as u32,
        "BUFFER42" => ParameterType::BUFFER42 as u32,
        "BUFFER100" => ParameterType::BUFFER100 as u32,
        "LIST_UINT8" => ParameterType::LIST_UINT8 as u32,
        "LIST_UINT16" => ParameterType::LIST_UINT16 as u32,
        "LIST_NETWORK" => ParameterType::LIST_NETWORK as u32,
        "INT8" => ParameterType::INT8 as u32,
        other => panic!("unknown parameter type '{other}'"),
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "panic".to_string()
}

#[test]
fn buffalo_znp_matches_upstream_fixtures() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/buffalo_znp.json"))
        .expect("buffalo_znp fixtures");

    for case in cases {
        let options = case.options.clone().unwrap_or_else(|| json!({}));
        match case.op.as_str() {
            "read" => {
                let mut buffalo =
                    BuffaloZnp::new(unhex(case.buffer.as_deref().expect("buffer")), 0);
                let result = catch_unwind(AssertUnwindSafe(|| {
                    buffalo.read(parameter_type(&case.kind), &options)
                }));

                match (result, case.expected, case.error) {
                    (Ok(value), Some(expected), _) => {
                        assert_eq!(value, expected, "{}", case.id);
                        assert_eq!(
                            buffalo.get_position(),
                            case.position.expect("position"),
                            "{}",
                            case.id
                        );
                    }
                    (Err(payload), _, Some(error)) => {
                        assert_eq!(panic_message(payload), error, "{}", case.id);
                    }
                    other => panic!("{}: unexpected result {other:?}", case.id),
                }
            }
            "write" => {
                let mut buffalo = BuffaloZnp::new(vec![0; 250], 0);
                let value = case.value.as_ref().expect("value");
                let result = catch_unwind(AssertUnwindSafe(|| {
                    buffalo.write(parameter_type(&case.kind), value, &options)
                }));

                match (result, case.expected, case.error) {
                    (Ok(()), Some(expected), _) => {
                        assert_eq!(json!(hex(&buffalo.get_written())), expected, "{}", case.id);
                        assert_eq!(
                            buffalo.get_position(),
                            case.position.expect("position"),
                            "{}",
                            case.id
                        );
                    }
                    (Err(payload), _, Some(error)) => {
                        assert_eq!(panic_message(payload), error, "{}", case.id);
                    }
                    other => panic!("{}: unexpected result {other:?}", case.id),
                }
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
