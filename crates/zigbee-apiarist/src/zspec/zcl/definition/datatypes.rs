// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/datatypes.ts

use std::collections::BTreeMap;

use serde_json::{json, Value};

/// Upstream `ZCL_TYPE_INVALID_BY_TYPE_NAME`.
pub fn zcl_type_invalid_by_type_name() -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        ("Bool", json!(255)),
        ("Uint8", json!(255)),
        ("Uint16", json!(65535)),
        ("Uint24", json!(16777215)),
        ("Uint32", json!(4294967295u64)),
        ("Uint40", json!(1099511627775u64)),
        ("Uint48", json!(281474976710655u64)),
        ("Uint56", json!(72057594037927935u64)),
        ("Uint64", json!(18446744073709551615u64)),
        ("Int8", json!(-128)),
        ("Int16", json!(-32768)),
        ("Int24", json!(-8388608)),
        ("Int32", json!(-2147483648i64)),
        ("Int40", json!(-549755813888i64)),
        ("Int48", json!(-140737488355328i64)),
        ("Int56", json!(-36028797018963968i64)),
        ("Int64", json!(-9223372036854775808i64)),
        ("Enum8", json!(255)),
        ("Enum16", json!(65535)),
        ("ToD", json!(4294967295u64)),
        ("Date", json!(4294967295u64)),
        ("Utc", json!(4294967295u64)),
        ("ClusterId", json!(65535)),
        ("AttribId", json!(65535)),
        ("BacOid", json!(4294967295u64)),
    ])
}

/// Upstream `ZCL_TYPE_INVALID_BY_TYPE`.
pub fn zcl_type_invalid_by_type() -> BTreeMap<u8, Value> {
    BTreeMap::from([
        (16, json!(255)),
        (32, json!(255)),
        (33, json!(65535)),
        (34, json!(16777215)),
        (35, json!(4294967295u64)),
        (36, json!(1099511627775u64)),
        (37, json!(281474976710655u64)),
        (38, json!(72057594037927935u64)),
        (39, json!(18446744073709551615u64)),
        (40, json!(-128)),
        (41, json!(-32768)),
        (42, json!(-8388608)),
        (43, json!(-2147483648i64)),
        (44, json!(-549755813888i64)),
        (45, json!(-140737488355328i64)),
        (46, json!(-36028797018963968i64)),
        (47, json!(-9223372036854775808i64)),
        (48, json!(255)),
        (49, json!(65535)),
        (224, json!(4294967295u64)),
        (225, json!(4294967295u64)),
        (226, json!(4294967295u64)),
        (232, json!(65535)),
        (233, json!(65535)),
        (234, json!(4294967295u64)),
    ])
}
