/// ZCL Groups cluster (0x0004) command builders and response parsing.
use serde_json::{json, Value};

use crate::zigbee::zcl::frame::ZclFrameHeader;

pub const CLUSTER_ID: u16 = 0x0004;
pub const CMD_ADD_GROUP: u8 = 0x00;
pub const CMD_VIEW_GROUP: u8 = 0x01;
pub const CMD_GET_MEMBERSHIP: u8 = 0x02;
pub const CMD_REMOVE_GROUP: u8 = 0x03;
pub const CMD_REMOVE_ALL_GROUPS: u8 = 0x04;
pub const RSP_ADD_GROUP: u8 = 0x00;
pub const RSP_VIEW_GROUP: u8 = 0x01;
pub const RSP_GET_MEMBERSHIP: u8 = 0x02;
pub const RSP_REMOVE_GROUP: u8 = 0x03;

fn cluster_payload(tsn: u8, command: u8) -> Vec<u8> {
    // cluster-specific, client->server, default response enabled
    vec![0x01, tsn, command]
}

pub fn add_group_payload(tsn: u8, group_id: u16, name: &str) -> Vec<u8> {
    let mut payload = cluster_payload(tsn, CMD_ADD_GROUP);
    payload.extend_from_slice(&group_id.to_le_bytes());
    let name = name.as_bytes();
    let length = name.len().min(16);
    payload.push(length as u8);
    payload.extend_from_slice(&name[..length]);
    payload
}

pub fn view_group_payload(tsn: u8, group_id: u16) -> Vec<u8> {
    let mut payload = cluster_payload(tsn, CMD_VIEW_GROUP);
    payload.extend_from_slice(&group_id.to_le_bytes());
    payload
}

pub fn remove_group_payload(tsn: u8, group_id: u16) -> Vec<u8> {
    let mut payload = cluster_payload(tsn, CMD_REMOVE_GROUP);
    payload.extend_from_slice(&group_id.to_le_bytes());
    payload
}

pub fn get_membership_payload(tsn: u8, group_ids: &[u16]) -> Vec<u8> {
    let mut payload = cluster_payload(tsn, CMD_GET_MEMBERSHIP);
    payload.push(group_ids.len().min(255) as u8);
    for group_id in group_ids.iter().take(255) {
        payload.extend_from_slice(&group_id.to_le_bytes());
    }
    payload
}

#[must_use]
pub fn remove_all_groups_payload(tsn: u8) -> Vec<u8> {
    cluster_payload(tsn, CMD_REMOVE_ALL_GROUPS)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupResponse {
    pub command: u8,
    pub status: u8,
    pub group_id: Option<u16>,
    pub members: Vec<u16>,
}

impl GroupResponse {
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == 0
    }

    #[must_use]
    pub fn as_json(&self) -> Value {
        json!({
            "command": self.command,
            "status": self.status,
            "group_id": self.group_id,
            "members": self.members,
        })
    }
}

/// Parse a Groups-cluster response (specific response or default response).
#[must_use]
pub fn parse_response(raw: &[u8]) -> Option<GroupResponse> {
    let (header, offset) = ZclFrameHeader::parse(raw).ok()?;
    let payload = raw.get(offset..)?;
    match header.command_id {
        RSP_ADD_GROUP | RSP_REMOVE_GROUP => {
            if payload.len() < 3 {
                return None;
            }
            Some(GroupResponse {
                command: header.command_id,
                status: payload[0],
                group_id: Some(u16::from_le_bytes([payload[1], payload[2]])),
                members: Vec::new(),
            })
        }
        RSP_VIEW_GROUP => {
            if payload.len() < 4 {
                return None;
            }
            Some(GroupResponse {
                command: header.command_id,
                status: payload[0],
                group_id: Some(u16::from_le_bytes([payload[1], payload[2]])),
                members: Vec::new(),
            })
        }
        RSP_GET_MEMBERSHIP => {
            if payload.len() < 2 {
                return None;
            }
            let count = payload[1] as usize;
            if payload.len() < 2 + count * 2 {
                return None;
            }
            let members = (0..count)
                .map(|index| {
                    let offset = 2 + index * 2;
                    u16::from_le_bytes([payload[offset], payload[offset + 1]])
                })
                .collect();
            Some(GroupResponse {
                command: header.command_id,
                status: 0,
                group_id: None,
                members,
            })
        }
        0x0B => {
            // default response: payload = [command, status]
            if payload.len() < 2 {
                return None;
            }
            Some(GroupResponse {
                command: 0x0B,
                status: payload[1],
                group_id: None,
                members: Vec::new(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_group_payload_layout() {
        let payload = add_group_payload(7, 0x1234, "kitchen");
        assert_eq!(&payload[..5], &[0x01, 7, 0x00, 0x34, 0x12]);
        assert_eq!(payload[5], 7);
        assert_eq!(&payload[6..], b"kitchen");
    }

    #[test]
    fn add_group_name_is_truncated_to_16_bytes() {
        let payload = add_group_payload(1, 1, "a-very-long-group-name");
        assert_eq!(payload[5], 16);
        assert_eq!(payload.len(), 6 + 16);
    }

    #[test]
    fn remove_and_remove_all_payload_layout() {
        assert_eq!(
            remove_group_payload(9, 0x0102),
            vec![0x01, 9, 0x03, 0x02, 0x01]
        );
        assert_eq!(remove_all_groups_payload(3), vec![0x01, 3, 0x04]);
    }

    #[test]
    fn get_membership_payload_layout() {
        assert_eq!(
            get_membership_payload(4, &[0x0102, 0x0304]),
            vec![0x01, 4, 0x02, 0x02, 0x02, 0x01, 0x04, 0x03]
        );
    }

    #[test]
    fn parses_add_and_default_responses() {
        let add = parse_response(&[0x18, 7, 0x00, 0x00, 0x34, 0x12]).unwrap();
        assert!(add.is_success());
        assert_eq!(add.group_id, Some(0x1234));
        let failure = parse_response(&[0x18, 7, 0x00, 0x01, 0x34, 0x12]).unwrap();
        assert!(!failure.is_success());

        let default = parse_response(&[0x18, 7, 0x0B, 0x04, 0x00]).unwrap();
        assert!(default.is_success());
        assert_eq!(default.command, 0x0B);
    }

    #[test]
    fn parses_get_membership_response() {
        let response =
            parse_response(&[0x18, 7, 0x02, 0x01, 0x02, 0x02, 0x01, 0x04, 0x03]).unwrap();
        assert_eq!(response.members, vec![0x0102, 0x0304]);
    }
}
