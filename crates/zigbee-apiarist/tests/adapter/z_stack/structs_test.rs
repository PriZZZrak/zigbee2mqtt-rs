// Original: .upstream/zigbee-herdsman/test/adapter/z-stack/structs.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Z-Stack Structs
//   - should evaluate table struct
//   - should export js object from structure
//   - should fail if table entries are not the same length
//   - should fail if table initialization buffer is indivisible by entry length
//   - should fail if table initialization source is unsupported
//   - should fail to initialize struct from invalid length source
//   - should properly evaluate security manager table occupancy
//   - should properly return table capacity metrics
//   - should properly serialize structure in different alignment modes
//   - should properly serialize unaligned and aligned table - with inline occupancy
//   - should properly serialize unaligned and aligned table - without inline occupancy
//   - should throw error on invalid uint8 array length

use serde_json::json;
use zigbee_apiarist::adapter::z_stack::structs::{
    nib, nib_from, nwk_sec_material_descriptor_table, security_manager_table,
    security_manager_table_from_buffer, security_manager_table_from_entries,
    security_manager_table_from_value,
};

fn hex_decode(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}

fn hex_encode(value: &[u8]) -> String {
    value.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
#[ignore = "not implemented yet"]
fn should_evaluate_table_struct() {
    let table = nwk_sec_material_descriptor_table(8);
    assert_eq!(table.capacity, 8);
    assert_eq!(table.entries.len(), 8);
    assert_eq!(table.free_count, 8);
    assert_eq!(table.used_count, 0);
}

#[test]
#[ignore = "not implemented yet"]
fn should_export_js_object_from_structure() {
    let mut nib = nib();
    nib.set_extended_pan_id(vec![0x01; 8]);
    nib.nwk_logical_channel = 21;

    let obj = nib.to_json();
    assert_eq!(obj["extendedPANID"], json!(vec![0x01u8; 8]));
    assert_eq!(obj["nwkLogicalChannel"], json!(21));
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_if_table_entries_are_not_the_same_length() {
    let result = std::panic::catch_unwind(|| {
        security_manager_table_from_entries(vec![vec![0x00; 12], vec![0x00; 17]], "unaligned");
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_if_table_initialization_buffer_is_indivisible_by_entry_length() {
    let result = std::panic::catch_unwind(|| {
        security_manager_table_from_buffer(vec![0x00; 91]);
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_if_table_initialization_source_is_unsupported() {
    let result = std::panic::catch_unwind(|| {
        security_manager_table_from_value(&json!("das_garbage"));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_initialize_struct_from_invalid_length_source() {
    let result = std::panic::catch_unwind(|| {
        nib_from(hex_decode("01020304"));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn should_properly_evaluate_security_manager_table_occupancy() {
    let mut table = security_manager_table(8);
    table.free[6].ami = 0xfffe;
    table.free[6].authentication_option = 0x00;
    table.free[7].ami = 0x0000;
    table.free[7].authentication_option = 0x00;
    assert_eq!(table.free_count, 8);
    assert_eq!(table.used_count, 0);
}

#[test]
#[ignore = "not implemented yet"]
fn should_properly_return_table_capacity_metrics() {
    let table = security_manager_table(8);
    assert_eq!(table.used.len(), 0);
    assert_eq!(table.free.len(), 8);
}

#[test]
#[ignore = "not implemented yet"]
fn should_properly_serialize_structure_in_different_alignment_modes() {
    let source = "fb050279147900640000000105018f000700020d1e000000150000000000000000000000ffff0800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";
    let nib = nib_from(hex_decode(source));
    assert_eq!(
            hex_encode(&nib.serialize()),
            "fb050279147900640000000105018f0700020d1e00001500000000000000000000ffff08000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a010000060200"
        );
    assert_eq!(
            hex_encode(&nib.serialize_with("unaligned", true, 0)),
            "fb050279147900640000000105018f0700020d1e00001500000000000000000000ffff08000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a010000060200"
        );
    assert_eq!(
            hex_encode(&nib.serialize_with("aligned", true, 0)),
            "fb050279147900640000000105018f000700020d1e000000150000000000000000000000ffff0800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000"
        );
    assert_eq!(nib.get_length("aligned"), 116);
    assert_eq!(nib.get_length_unpadded("aligned"), 115);
    assert_eq!(nib.get_length("unaligned"), 110);
}

#[test]
#[ignore = "not implemented yet"]
fn should_properly_serialize_unaligned_and_aligned_table_with_inline_occupancy() {
    let table = security_manager_table(8);
    assert_eq!(
        hex_encode(&table.serialize()),
        "0000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000"
    );
    assert_eq!(
        hex_encode(&table.serialize_mode("unaligned")),
        "0000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000"
    );
    assert_eq!(
            hex_encode(&table.serialize_mode("aligned")),
            "0000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000"
        );
}

#[test]
#[ignore = "not implemented yet"]
fn should_properly_serialize_unaligned_and_aligned_table_without_inline_occupancy() {
    let table = nwk_sec_material_descriptor_table(8);
    let zeros = "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    assert_eq!(hex_encode(&table.serialize()), zeros);
    assert_eq!(hex_encode(&table.serialize_mode("unaligned")), zeros);
    assert_eq!(hex_encode(&table.serialize_mode("aligned")), zeros);
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_on_invalid_uint8_array_length() {
    let mut nib = nib();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        nib.set_extended_pan_id(vec![0x01; 10]);
    }));
    assert!(result.is_err());
}
