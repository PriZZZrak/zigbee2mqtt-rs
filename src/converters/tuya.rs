use serde_json::{json, Value};

pub fn from_zigbee_ef00(data: &[u8]) -> Option<Value> {
    if data.len() < 7 { return None; }
    let dp = data[2];
    let len = u16::from_be_bytes([data[4], data[5]]) as usize;
    if data.len() < 6+len { return None; }
    let val = &data[6..6+len];
    match dp {
        1 if val.len()>=1 => Some(json!({"state": if val[0]==1 {"ON"} else {"OFF"}})),
        _ => Some(json!({format!("dp_{}", dp): val.iter().map(|b| format!("{:02x}", b)).collect::<String>()})),
    }
}
pub fn to_zigbee_ef00(dp: u8, val: u8) -> Vec<u8> {
    vec![0x00, 0x00, dp, 1, 0, 1, val]
}
