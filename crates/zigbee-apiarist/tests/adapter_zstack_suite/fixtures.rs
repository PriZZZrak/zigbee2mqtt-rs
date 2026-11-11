//! NV fixtures mirroring upstream `adapter.test.ts` builders.
//!
//! Anchor: .upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts
//! (`baseZnpRequestMock`, `empty*RequestMock`, `commissioned*RequestMock`).

#![allow(dead_code)]

use serde_json::{json, Value};

use zigbee_apiarist::adapter::z_stack::constants::common::{NvItemsIds, NvSystemIds};
use zigbee_apiarist::adapter::z_stack::structs::entries::nib::nib;
use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;

use super::support::{MockCoordinator, MockState, Reply};

pub fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            u8::from_str_radix(std::str::from_utf8(chunk).expect("ascii"), 16).expect("hex")
        })
        .collect()
}

pub const NIB_BASE: &str = "fb050279147900640000000105018f000700020d1e0000001500000000000000000000000bcd0800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";
pub const NIB_EMPTY_3_ALIGNED: &str = "fb050279147900640000000105018f000700020d1e000000150000000000000000000000ffff0800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";
pub const NIB_COMMISSIONED_3_ALIGNED: &str = "fb050279147900640000000105018f000700020d1e0000001500000000000000000000007b000800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";
pub const NIB_COMMISSIONED_3_CONFIG_MISMATCH: &str = "fb050279147900640000000105018f000700020d1e0000001500000000000000000000007e000800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";
pub const NIB_EMPTY_3_UNALIGNED: &str = "fb050279147900640000000105018f0700020d1e00001500000000000000000000ffff08000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a010000060200";
pub const NIB_COMMISSIONED_12_UNALIGNED: &str = "fb050279147900640000000105018f0700020d1e000015000000000000000000007b0008000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a010000060200";
pub const NIB_EMPTY_3_UNALIGNED_COMMISSIONED: &str = "fb050279147900640000000105018f0700020d1e000015000000000000000000007b0008000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a010000060200";
pub const NIB_COMMISSIONED_3_PAN_MISMATCH: &str = "fb050279147900640000000105018f000700020d1e0000001500000000000000000000007c000800000020000f0f0400010000000100000000779fd609004b1200010000000000000000000000000000000000000000000000000000000000000000000000003c0c0001780a0100000006020000";

pub const ADDRMGR_EMPTY: &str = "00ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff00000000000000000000";
pub const ADDRMGR_COMMISSIONED: &str = "01ff4f3a080000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff0000000000000000000000ff00000000000000000000";
pub const APS_LINK_KEY_TABLE_EMPTY: &str = "0000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000";
pub const APS_LINK_KEY_TABLE_3X0_EMPTY: &str = "0000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000feff00000000";
pub const APS_LINK_KEY_TABLE_EMPTY_UNALIGNED: &str = "0000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000feff000000";

pub const NETWORK_KEY: [u8; 16] = [
    0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f, 0x00, 0x02, 0x04, 0x06, 0x08, 0x0a, 0x0c, 0x0d,
];

fn table(mock: &MockCoordinator, start: u16, count: u16, length: usize) {
    for index in 0..count {
        mock.nv(start + index, Some(vec![0u8; length]));
    }
}

fn extended_table(mock: &MockCoordinator, id: u16, count: u16, length: usize) {
    for index in 0..count {
        mock.nv_ext(
            NvSystemIds::ZSTACK as u8,
            id,
            index,
            Some(vec![0u8; length]),
        );
    }
}

/// Base state from `baseZnpRequestMock` plus its fixture-level `.nv` entries.
pub fn base(mock: &MockCoordinator) {
    mock.nv(NvItemsIds::CHANLIST, Some(vec![0, 8, 0, 0]));
    mock.nv(NvItemsIds::PRECFGKEY, Some(vec![0u8; 16]));
    mock.nv(NvItemsIds::PRECFGKEYS_ENABLE, Some(vec![0]));
    mock.nv(NvItemsIds::NWKKEY, Some(vec![0u8; 24]));
    mock.nv(NvItemsIds::NWK_ACTIVE_KEY_INFO, Some(vec![0u8; 18]));
    mock.nv(NvItemsIds::NWK_ALTERN_KEY_INFO, Some(vec![0u8; 18]));
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x00]));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_BASE)));

    // `bdbStartCommissioning` replaces the NIB with the post-commissioning
    // image and `extNwkInfo` reports the current NIB.
    mock.handle(Subsystem::APP_CNF, "bdbStartCommissioning", |_, state| {
        state
            .nv
            .insert(NvItemsIds::NIB, Some(hex(NIB_COMMISSIONED_3_ALIGNED)));
        Reply::empty().with_extra(Subsystem::ZDO, 192, vec![9])
    });
    mock.handle(Subsystem::ZDO, "extNwkInfo", |_, state| {
        let Some(value) = state.nv.get(&NvItemsIds::NIB).cloned().flatten() else {
            return Reply::timeout();
        };
        let nib = nib(Some(&value));
        let panid = nib.get_u16("nwkPanId").unwrap_or(0);
        let extended_pan_id = nib.get_bytes("extendedPANID").unwrap_or_default();
        let channel = nib.get_u8("nwkLogicalChannel").unwrap_or(0);
        let extended = extended_pan_id
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Reply::payload(json!({
            "panid": panid,
            "extendedpanid": format!("0x{extended}"),
            "channel": channel,
        }))
    });
}

pub fn empty_3_aligned(mock: &MockCoordinator) {
    base(mock);
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x00]));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_EMPTY_3_ALIGNED)));
    mock.nv(NvItemsIds::ADDRMGR, Some(hex(ADDRMGR_EMPTY)));
    mock.nv(
        NvItemsIds::APS_LINK_KEY_TABLE,
        Some(hex(APS_LINK_KEY_TABLE_EMPTY)),
    );
    table(mock, NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START, 4, 12);
    table(mock, NvItemsIds::LEGACY_TCLK_TABLE_START, 16, 20);
    table(mock, NvItemsIds::APS_LINK_KEY_DATA_START, 16, 24);
}

pub fn empty_3_unaligned(mock: &MockCoordinator) {
    base(mock);
    mock.handle(Subsystem::APP_CNF, "bdbStartCommissioning", |_, state| {
        state.nv.insert(
            NvItemsIds::NIB,
            Some(hex(NIB_EMPTY_3_UNALIGNED_COMMISSIONED)),
        );
        Reply::empty().with_extra(Subsystem::ZDO, 192, vec![9])
    });
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x00]));
    mock.nv(NvItemsIds::NWKKEY, Some(vec![0u8; 21]));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_EMPTY_3_UNALIGNED)));
    mock.nv(NvItemsIds::ADDRMGR, Some(hex(ADDRMGR_EMPTY)));
    mock.nv(
        NvItemsIds::APS_LINK_KEY_TABLE,
        Some(hex(APS_LINK_KEY_TABLE_EMPTY_UNALIGNED)),
    );
    table(mock, NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START, 4, 12);
    table(mock, NvItemsIds::LEGACY_TCLK_TABLE_START, 16, 20);
    table(mock, NvItemsIds::APS_LINK_KEY_DATA_START, 16, 24);
}

pub fn empty_3x0_aligned(mock: &MockCoordinator) {
    base(mock);
    mock.handle(Subsystem::SYS, "version", |payload, _| {
        if payload.as_object().map(|object| object.is_empty()) == Some(true) {
            Reply::payload(json!({"product": 1, "revision": 20210430}))
        } else {
            Reply::timeout()
        }
    });
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x00]));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_EMPTY_3_ALIGNED)));
    mock.nv(
        NvItemsIds::APS_LINK_KEY_TABLE,
        Some(hex(APS_LINK_KEY_TABLE_3X0_EMPTY)),
    );
    extended_table(mock, NvItemsIds::ZCD_NV_EX_ADDRMGR, 16, 12);
    extended_table(mock, NvItemsIds::EX_NWK_SEC_MATERIAL_TABLE, 4, 12);
    extended_table(mock, NvItemsIds::EX_TCLK_TABLE, 16, 20);
    extended_table(mock, NvItemsIds::ZCD_NV_EX_APS_KEY_DATA_TABLE, 16, 24);
}

pub fn commissioned_3_aligned(mock: &MockCoordinator) {
    empty_3_aligned(mock);
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x55]));
    mock.nv(NvItemsIds::PRECFGKEY, Some(NETWORK_KEY.to_vec()));
    let mut active_key = vec![0];
    active_key.extend_from_slice(&NETWORK_KEY);
    active_key.push(0);
    mock.nv(NvItemsIds::NWK_ACTIVE_KEY_INFO, Some(active_key.clone()));
    mock.nv(NvItemsIds::NWK_ALTERN_KEY_INFO, Some(active_key));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_COMMISSIONED_3_ALIGNED)));
    mock.nv(NvItemsIds::ADDRMGR, Some(hex(ADDRMGR_COMMISSIONED)));
}

pub fn commissioned_3_aligned_config_mismatch(mock: &MockCoordinator) {
    commissioned_3_aligned(mock);
    mock.nv(
        NvItemsIds::NIB,
        Some(hex(NIB_COMMISSIONED_3_CONFIG_MISMATCH)),
    );
}

pub fn commissioned_3x0_aligned(mock: &MockCoordinator) {
    empty_3x0_aligned(mock);
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, Some(vec![0x55]));
    mock.nv(NvItemsIds::PRECFGKEY, Some(NETWORK_KEY.to_vec()));
    let mut active_key = vec![0];
    active_key.extend_from_slice(&NETWORK_KEY);
    active_key.push(0);
    mock.nv(NvItemsIds::NWK_ACTIVE_KEY_INFO, Some(active_key.clone()));
    mock.nv(NvItemsIds::NWK_ALTERN_KEY_INFO, Some(active_key));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_COMMISSIONED_3_ALIGNED)));
    mock.nv_ext(
        NvSystemIds::ZSTACK as u8,
        NvItemsIds::ZCD_NV_EX_ADDRMGR,
        0,
        Some(hex("01ff4f3a0800000000000000")),
    );
}

pub fn commissioned_3x0_precfg_key_mismatch(mock: &MockCoordinator) {
    commissioned_3x0_aligned(mock);
    mock.nv(
        NvItemsIds::PRECFGKEY,
        Some(hex("aabbccddeeff00112233445566778899")),
    );
}

pub fn empty_12_unaligned(mock: &MockCoordinator) {
    base(mock);
    mock.handle(Subsystem::SYS, "version", |payload, _| {
        if payload.as_object().map(|object| object.is_empty()) == Some(true) {
            Reply::payload(json!({"product": 0}))
        } else {
            Reply::timeout()
        }
    });
    mock.handle(
        Subsystem::SAPI,
        "readConfiguration",
        |payload: &Value, state: &mut MockState| {
            if payload.get("configid").and_then(Value::as_u64)
                != Some(u64::from(NvItemsIds::PRECFGKEY))
            {
                return Reply::timeout();
            }
            match state.nv.get(&NvItemsIds::PRECFGKEY).cloned().flatten() {
                Some(value) => {
                    let mut result = serde_json::Map::new();
                    result.insert("status".to_string(), json!(0));
                    result.insert("configid".to_string(), json!(NvItemsIds::PRECFGKEY));
                    result.insert("len".to_string(), json!(value.len()));
                    result.insert(
                        "value".to_string(),
                        Value::Array(value.into_iter().map(|b| json!(b)).collect()),
                    );
                    Reply::payload(Value::Object(result))
                }
                None => Reply::payload(json!({"status": 1})),
            }
        },
    );
    mock.handle(
        Subsystem::SAPI,
        "writeConfiguration",
        |payload: &Value, state: &mut MockState| {
            if payload.get("configid").and_then(Value::as_u64)
                != Some(u64::from(NvItemsIds::PRECFGKEY))
            {
                return Reply::timeout();
            }
            let value = super::support::bytes(payload.get("value").unwrap_or(&Value::Null));
            state.nv.insert(NvItemsIds::PRECFGKEY, Some(value));
            state
                .nv
                .insert(NvItemsIds::NIB, Some(hex(NIB_COMMISSIONED_12_UNALIGNED)));
            Reply::payload(json!({"status": 0}))
        },
    );
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1, Some(vec![0x00]));
    mock.nv(NvItemsIds::NWKKEY, Some(vec![0u8; 21]));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_EMPTY_3_UNALIGNED)));
    mock.nv(NvItemsIds::ADDRMGR, Some(hex(ADDRMGR_EMPTY)));
    mock.nv(
        NvItemsIds::APS_LINK_KEY_TABLE,
        Some(hex(APS_LINK_KEY_TABLE_EMPTY_UNALIGNED)),
    );
    table(mock, NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START, 4, 12);
    table(mock, NvItemsIds::LEGACY_TCLK_TABLE_START, 16, 20);
    table(mock, NvItemsIds::APS_LINK_KEY_DATA_START, 16, 24);
}

pub fn commissioned_12_unaligned(mock: &MockCoordinator) {
    empty_12_unaligned(mock);
    mock.nv(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1, Some(vec![0x55]));
    mock.nv(NvItemsIds::PRECFGKEY, Some(NETWORK_KEY.to_vec()));
    mock.nv(NvItemsIds::NIB, Some(hex(NIB_COMMISSIONED_12_UNALIGNED)));
}

pub fn commissioned_12_unaligned_mismatch(mock: &MockCoordinator) {
    commissioned_12_unaligned(mock);
    mock.nv(
        NvItemsIds::PRECFGKEY,
        Some(hex("aabb0507090b0d0f00020406080a0c0d")),
    );
}
