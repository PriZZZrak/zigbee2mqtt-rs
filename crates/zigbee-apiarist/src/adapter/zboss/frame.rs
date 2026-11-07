// Original: .upstream/zigbee-herdsman/src/adapter/zboss/frame.ts
// TODO: port public API from the original file.

// pub fn ZBOSSBuffaloZcl() { todo!() }
// pub fn makeFrame() { todo!() }
// pub fn readByDesc() { todo!() }
// pub fn readZBOSSFrame() { todo!() }
// pub fn writeByDesc() { todo!() }
// pub fn writeZBOSSFrame() { todo!() }

pub fn zbossbuffalo_zcl() {
    todo!()
}
pub fn make_frame() {
    todo!()
}
pub fn read_by_desc() {
    todo!()
}
pub fn read_zbossframe() {
    todo!()
}
pub fn write_by_desc() {
    todo!()
}
pub fn write_zbossframe() {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    UNKNOWN = 0,
    RESPONSE = 1,
    INDICATION = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandId;

impl CommandId {
    pub const ZDO_NWK_ADDR_REQ: u16 = 0x0201;
    pub const ZDO_POWER_DESC_REQ: u16 = 0x0203;
    pub const ZDO_NODE_DESC_REQ: u16 = 0x0204;
    pub const ZDO_SIMPLE_DESC_REQ: u16 = 0x0205;
    pub const ZDO_ACTIVE_EP_REQ: u16 = 0x0206;
    pub const ZDO_MATCH_DESC_REQ: u16 = 0x0207;
    pub const ZDO_DEV_ANNCE_IND: u16 = 0x020c;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZBOSSFrame {
    pub version: u8,
    pub r#type: FrameType,
    pub command_id: u16,
    pub tsn: u8,
    pub payload: serde_json::Value,
}

/// Upstream: `readZBOSSFrame(buffer)`.
pub fn read_zboss_frame(buffer: &[u8]) -> ZBOSSFrame {
    use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;

    let version = buffer.first().copied().unwrap_or(0);
    let frame_type = match buffer.get(1).copied().unwrap_or(0) {
        1 => FrameType::RESPONSE,
        2 => FrameType::INDICATION,
        _ => FrameType::UNKNOWN,
    };
    let command_id = u16::from_le_bytes([
        buffer.get(2).copied().unwrap_or(0),
        buffer.get(3).copied().unwrap_or(0),
    ]);
    let is_response = frame_type == FrameType::RESPONSE;
    let tsn = if is_response {
        buffer.get(4).copied().unwrap_or(0)
    } else {
        0
    };
    let category = if is_response {
        serde_json::json!(buffer.get(5).copied().unwrap_or(0))
    } else {
        serde_json::Value::Null
    };
    let zdo_cluster = match command_id {
        CommandId::ZDO_NWK_ADDR_REQ => Some(0x8000u16),
        CommandId::ZDO_POWER_DESC_REQ => Some(0x8003),
        CommandId::ZDO_NODE_DESC_REQ => Some(0x8002),
        CommandId::ZDO_SIMPLE_DESC_REQ => Some(0x8004),
        CommandId::ZDO_ACTIVE_EP_REQ => Some(0x8005),
        CommandId::ZDO_MATCH_DESC_REQ => Some(0x8006),
        CommandId::ZDO_DEV_ANNCE_IND => Some(0x0013),
        _ => None,
    };
    let payload = if let Some(cluster) = zdo_cluster {
        let start = if is_response { 6 } else { 4 };
        let raw = buffer.get(start..).unwrap_or(&[]);
        let mut fixed = fix_zdo_payload(cluster, raw);
        if cluster == 0x8002 {
            while fixed.len() < 16 {
                fixed.push(0);
            }
        }
        let zdo = BuffaloZdo::read_response(false, cluster, &fixed);
        serde_json::json!({
            "category": category,
            "zdoClusterId": cluster,
            "zdo": zdo,
        })
    } else {
        serde_json::Value::Null
    };
    ZBOSSFrame {
        version,
        r#type: frame_type,
        command_id,
        tsn,
        payload,
    }
}

fn fix_zdo_payload(cluster: u16, buffer: &[u8]) -> Vec<u8> {
    match cluster {
        0x8002 | 0x8003 | 0x8005 | 0x8006 => {
            if buffer.len() < 3 {
                return buffer.to_vec();
            }
            let mut output = vec![buffer[0]];
            output.extend_from_slice(&buffer[buffer.len() - 2..]);
            output.extend_from_slice(&buffer[1..buffer.len() - 2]);
            output
        }
        0x8004 => {
            if buffer.len() < 10 {
                return buffer.to_vec();
            }
            let in_count = buffer[7] as usize;
            let in_size = in_count * 2;
            let mut output = vec![buffer[0]];
            output.extend_from_slice(&buffer[buffer.len() - 2..]);
            output.push((buffer.len() - 3) as u8);
            output.extend_from_slice(&buffer[1..8]);
            output.extend_from_slice(&buffer[9..(9 + in_size).min(buffer.len())]);
            output.extend_from_slice(&buffer[8..9]);
            let end = buffer.len().saturating_sub(2);
            if 9 + in_size < end {
                output.extend_from_slice(&buffer[9 + in_size..end]);
            }
            output
        }
        _ => buffer.to_vec(),
    }
}
