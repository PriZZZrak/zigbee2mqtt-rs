// Differential fixture generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/structs.json.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde::Deserialize;
use serde_json::Value;
use zigbee_apiarist::adapter::z_stack::structs::entries::nib::nib;
use zigbee_apiarist::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_table::nwk_sec_material_descriptor_table;
use zigbee_apiarist::adapter::z_stack::structs::entries::security_manager_table::security_manager_table;
use zigbee_apiarist::adapter::z_stack::structs::r#struct::Alignment;
use zigbee_apiarist::adapter::z_stack::structs::table::TableData;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    source: Option<String>,
    capacity: Option<usize>,
    length: Option<usize>,
    expected: Option<Value>,
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

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "panic".to_string()
}

fn expected_error(case: &Case, result: Result<(), Box<dyn Any + Send>>) {
    match (result, &case.error) {
        (Err(payload), Some(error)) => assert!(
            panic_message(payload).starts_with(error),
            "{}: expected error prefix '{error}'",
            case.id
        ),
        (Ok(()), None) => {}
        (other, error) => panic!(
            "{}: unexpected result {other:?} (expected {error:?})",
            case.id
        ),
    }
}

#[test]
fn structs_match_upstream_fixtures() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/structs.json")).expect("structs fixtures");

    for case in cases {
        match case.op.as_str() {
            "nib_to_json" => {
                let mut nib = nib(None);
                nib.set_bytes("extendedPANID", &[0x01; 8]);
                nib.set_u8("nwkLogicalChannel", 21);
                let object = nib.to_json();
                let expected = case.expected.expect("expected");
                assert_eq!(
                    object["extendedPANID"].as_array().map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_u64)
                            .map(|value| value as u8)
                            .collect::<Vec<_>>()
                    }),
                    Some(unhex(expected["extendedPANID"].as_str().expect("hex")))
                );
                assert_eq!(object["nwkLogicalChannel"], expected["nwkLogicalChannel"]);
            }
            "nib_serialize" => {
                let source = unhex(case.source.as_deref().expect("source"));
                let nib = nib(Some(&source));
                let expected = case.expected.expect("expected");
                assert_eq!(
                    hex(&nib.serialize(Alignment::Unaligned, true, 0)),
                    expected["unaligned"].as_str().expect("unaligned")
                );
                assert_eq!(
                    hex(&nib.serialize(Alignment::Aligned, true, 0)),
                    expected["aligned"].as_str().expect("aligned")
                );
                assert_eq!(
                    nib.get_length(Alignment::Aligned, true, 0),
                    expected["alignedLength"].as_u64().expect("alignedLength") as usize
                );
                assert_eq!(
                    nib.get_length(Alignment::Aligned, false, 0),
                    expected["alignedLengthUnpadded"]
                        .as_u64()
                        .expect("alignedLengthUnpadded") as usize
                );
                assert_eq!(
                    nib.get_length(Alignment::Unaligned, true, 0),
                    expected["unalignedLength"]
                        .as_u64()
                        .expect("unalignedLength") as usize
                );
            }
            "security_table_serialize" => {
                let table = security_manager_table(
                    TableData::Capacity(case.capacity.expect("capacity")),
                    Alignment::Unaligned,
                );
                let expected = case.expected.expect("expected");
                assert_eq!(
                    hex(&table.serialize(Alignment::Unaligned)),
                    expected["unaligned"].as_str().expect("unaligned")
                );
                assert_eq!(
                    hex(&table.serialize(Alignment::Aligned)),
                    expected["aligned"].as_str().expect("aligned")
                );
                assert_eq!(
                    table.free_count(),
                    expected["free"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    table.used_count(),
                    expected["used"].as_u64().unwrap() as usize
                );
            }
            "security_table_occupancy" => {
                let mut table = security_manager_table(
                    TableData::Capacity(case.capacity.expect("capacity")),
                    Alignment::Unaligned,
                );
                table.entry_mut(6).unwrap().set_u16("ami", 0xfffe);
                table
                    .entry_mut(6)
                    .unwrap()
                    .set_u8("authenticationOption", 0x00);
                table.entry_mut(7).unwrap().set_u16("ami", 0x0000);
                table
                    .entry_mut(7)
                    .unwrap()
                    .set_u8("authenticationOption", 0x00);
                let expected = case.expected.expect("expected");
                assert_eq!(
                    table.free_count(),
                    expected["free"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    table.used_count(),
                    expected["used"].as_u64().unwrap() as usize
                );
            }
            "nwk_table_serialize" => {
                let table = nwk_sec_material_descriptor_table(
                    TableData::Capacity(case.capacity.expect("capacity")),
                    Alignment::Unaligned,
                );
                let expected = case.expected.expect("expected");
                assert_eq!(
                    hex(&table.serialize(Alignment::Unaligned)),
                    expected["unaligned"].as_str().expect("unaligned")
                );
                assert_eq!(
                    hex(&table.serialize(Alignment::Aligned)),
                    expected["aligned"].as_str().expect("aligned")
                );
                assert_eq!(
                    table.free_count(),
                    expected["free"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    table.used_count(),
                    expected["used"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    table.capacity(),
                    expected["capacity"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    table.entries().len(),
                    expected["entries"].as_u64().unwrap() as usize
                );
            }
            "invalid_nib_length" => {
                let length = case.length.expect("length");
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let mut nib = nib(None);
                    nib.set_bytes("extendedPANID", &vec![0x01; length]);
                }));
                expected_error(&case, result);
            }
            "invalid_nib_source" => {
                let source = unhex(case.source.as_deref().expect("source"));
                let result = catch_unwind(AssertUnwindSafe(|| {
                    nib(Some(&source));
                }));
                expected_error(&case, result);
            }
            "table_entries_mismatch" => {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    security_manager_table(
                        TableData::Entries(vec![vec![0x00; 12], vec![0x00; 17]]),
                        Alignment::Unaligned,
                    );
                }));
                expected_error(&case, result);
            }
            "table_buffer_indivisible" => {
                let length = case.length.expect("length");
                let result = catch_unwind(AssertUnwindSafe(|| {
                    security_manager_table(
                        TableData::Buffer(vec![0x00; length]),
                        Alignment::Unaligned,
                    );
                }));
                expected_error(&case, result);
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
