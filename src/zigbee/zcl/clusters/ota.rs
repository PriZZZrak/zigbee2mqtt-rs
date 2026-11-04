/// ZCL OTA Upgrade cluster (0x0019) codecs and transfer state machine.
///
/// This is the transport-independent core used by a fake coordinator in tests:
/// a device queries next image, requests blocks, and closes with an upgrade end
/// request. Network retrieval/metadata selection is intentionally separate.
use crate::zigbee::zcl::frame::ZclFrameHeader;

pub const CLUSTER_ID: u16 = 0x0019;
pub const CMD_QUERY_NEXT_IMAGE_REQUEST: u8 = 0x01;
pub const CMD_QUERY_NEXT_IMAGE_RESPONSE: u8 = 0x02;
pub const CMD_IMAGE_BLOCK_REQUEST: u8 = 0x03;
pub const CMD_IMAGE_BLOCK_RESPONSE: u8 = 0x05;
pub const CMD_UPGRADE_END_REQUEST: u8 = 0x06;
pub const CMD_UPGRADE_END_RESPONSE: u8 = 0x07;

pub const STATUS_SUCCESS: u8 = 0x00;
pub const STATUS_FAILURE: u8 = 0x01;
pub const STATUS_NO_IMAGE_AVAILABLE: u8 = 0x98;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtaImage {
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryNextImageRequest {
    pub field_control: u8,
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
    pub hardware_version: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlockRequest {
    pub field_control: u8,
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
    pub file_offset: u32,
    pub max_data_size: u8,
    pub minimum_block_period: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeEndRequest {
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
}

fn response_frame(tsn: u8, command: u8) -> Vec<u8> {
    // cluster-specific, server->client, disable default response
    vec![0x19, tsn, command]
}

fn parse_payload(raw: &[u8]) -> Option<(ZclFrameHeader, &[u8])> {
    let (header, offset) = ZclFrameHeader::parse(raw).ok()?;
    Some((header, raw.get(offset..)?))
}

pub fn parse_query_next_image_request(raw: &[u8]) -> Option<(u8, QueryNextImageRequest)> {
    let (header, payload) = parse_payload(raw)?;
    if header.command_id != CMD_QUERY_NEXT_IMAGE_REQUEST || payload.len() < 9 {
        return None;
    }
    let field_control = payload[0];
    let manufacturer_code = u16::from_le_bytes([payload[1], payload[2]]);
    let image_type = u16::from_le_bytes([payload[3], payload[4]]);
    let file_version = u32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]);
    let hardware_version = if field_control & 0x01 != 0 {
        if payload.len() < 11 {
            return None;
        }
        Some(u16::from_le_bytes([payload[9], payload[10]]))
    } else {
        None
    };
    Some((
        header.command_id,
        QueryNextImageRequest {
            field_control,
            manufacturer_code,
            image_type,
            file_version,
            hardware_version,
        },
    ))
}

pub fn query_next_image_response(tsn: u8, image: &OtaImage) -> Vec<u8> {
    let mut response = response_frame(tsn, CMD_QUERY_NEXT_IMAGE_RESPONSE);
    response.push(STATUS_SUCCESS);
    response.extend_from_slice(&image.manufacturer_code.to_le_bytes());
    response.extend_from_slice(&image.image_type.to_le_bytes());
    response.extend_from_slice(&image.file_version.to_le_bytes());
    response.extend_from_slice(&(image.bytes.len() as u32).to_le_bytes());
    response
}

pub fn no_image_response(tsn: u8) -> Vec<u8> {
    let mut response = response_frame(tsn, CMD_QUERY_NEXT_IMAGE_RESPONSE);
    response.push(STATUS_NO_IMAGE_AVAILABLE);
    response
}

pub fn parse_image_block_request(raw: &[u8]) -> Option<ImageBlockRequest> {
    let (header, payload) = parse_payload(raw)?;
    if header.command_id != CMD_IMAGE_BLOCK_REQUEST || payload.len() < 14 {
        return None;
    }
    let field_control = payload[0];
    let manufacturer_code = u16::from_le_bytes([payload[1], payload[2]]);
    let image_type = u16::from_le_bytes([payload[3], payload[4]]);
    let file_version = u32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]);
    let file_offset = u32::from_le_bytes([payload[9], payload[10], payload[11], payload[12]]);
    let max_data_size = payload[13];
    let mut position = 14;
    if field_control & 0x01 != 0 {
        position += 8; // request node address
    }
    let minimum_block_period = if field_control & 0x02 != 0 {
        if payload.len() < position + 2 {
            return None;
        }
        let value = u16::from_le_bytes([payload[position], payload[position + 1]]);
        position += 2;
        Some(value)
    } else {
        None
    };
    let _ = position;
    Some(ImageBlockRequest {
        field_control,
        manufacturer_code,
        image_type,
        file_version,
        file_offset,
        max_data_size,
        minimum_block_period,
    })
}

pub fn parse_upgrade_end_request(raw: &[u8]) -> Option<UpgradeEndRequest> {
    let (header, payload) = parse_payload(raw)?;
    if header.command_id != CMD_UPGRADE_END_REQUEST || payload.len() < 9 {
        return None;
    }
    Some(UpgradeEndRequest {
        manufacturer_code: u16::from_le_bytes([payload[1], payload[2]]),
        image_type: u16::from_le_bytes([payload[3], payload[4]]),
        file_version: u32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]),
    })
}

#[derive(Debug, Clone)]
pub struct OtaTransfer {
    image: OtaImage,
    offset: u32,
    complete: bool,
}

impl OtaTransfer {
    #[must_use]
    pub fn new(image: OtaImage) -> Self {
        Self {
            image,
            offset: 0,
            complete: false,
        }
    }

    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.image.bytes.is_empty() {
            return 1.0;
        }
        f64::from(self.offset) / self.image.bytes.len() as f64
    }

    #[must_use]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    fn validate(&self, manufacturer_code: u16, image_type: u16, file_version: u32) -> bool {
        self.image.manufacturer_code == manufacturer_code
            && self.image.image_type == image_type
            && self.image.file_version == file_version
    }

    /// Build the ImageBlockResponse for a device request.
    pub fn handle_block_request(&mut self, request: &ImageBlockRequest) -> Result<Vec<u8>, String> {
        if !self.validate(
            request.manufacturer_code,
            request.image_type,
            request.file_version,
        ) {
            return Err("image identity does not match the active transfer".into());
        }
        if request.max_data_size == 0 {
            return Err("device requested a zero-length block".into());
        }
        if request.file_offset as usize > self.image.bytes.len() {
            return Err("block offset is beyond the image".into());
        }
        let start = request.file_offset as usize;
        let end = (start + request.max_data_size as usize).min(self.image.bytes.len());
        let chunk = &self.image.bytes[start..end];
        self.offset = end as u32;
        if end == self.image.bytes.len() {
            self.complete = true;
        }
        let mut response = response_frame(request.field_control, CMD_IMAGE_BLOCK_RESPONSE);
        response.push(STATUS_SUCCESS);
        response.extend_from_slice(&self.image.manufacturer_code.to_le_bytes());
        response.extend_from_slice(&self.image.image_type.to_le_bytes());
        response.extend_from_slice(&self.image.file_version.to_le_bytes());
        response.extend_from_slice(&request.file_offset.to_le_bytes());
        response.push(chunk.len() as u8);
        response.extend_from_slice(chunk);
        Ok(response)
    }

    /// Validate the device's UpgradeEndRequest and return the response.
    pub fn handle_upgrade_end(&mut self, request: &UpgradeEndRequest) -> Result<Vec<u8>, String> {
        if !self.is_complete() {
            return Err("upgrade ended before the image was fully transferred".into());
        }
        if !self.validate(
            request.manufacturer_code,
            request.image_type,
            request.file_version,
        ) {
            return Err("upgrade end identity does not match the active transfer".into());
        }
        let mut response = response_frame(0, CMD_UPGRADE_END_RESPONSE);
        response.extend_from_slice(&self.image.manufacturer_code.to_le_bytes());
        response.extend_from_slice(&self.image.image_type.to_le_bytes());
        response.extend_from_slice(&self.image.file_version.to_le_bytes());
        response.extend_from_slice(&0u32.to_le_bytes()); // current time
        response.extend_from_slice(&0u32.to_le_bytes()); // upgrade time
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> OtaImage {
        OtaImage {
            manufacturer_code: 0x1037,
            image_type: 0x0001,
            file_version: 0x01020304,
            bytes: (0..10u8).collect(),
        }
    }

    #[test]
    fn parses_query_next_image_request() {
        let raw = vec![
            0x01, 0x01, 0x01, 0x01, 0x37, 0x10, 0x01, 0x00, 0x04, 0x03, 0x02, 0x01, 0x05, 0x00,
        ];
        let (_, request) = parse_query_next_image_request(&raw).unwrap();
        assert_eq!(request.manufacturer_code, 0x1037);
        assert_eq!(request.image_type, 1);
        assert_eq!(request.file_version, 0x01020304);
        assert_eq!(request.hardware_version, Some(5));
    }

    #[test]
    fn query_response_contains_identity_and_size() {
        let response = query_next_image_response(9, &image());
        assert_eq!(&response[..3], &[0x19, 9, 0x02]);
        assert_eq!(response[3], STATUS_SUCCESS);
        assert_eq!(
            u32::from_le_bytes([response[12], response[13], response[14], response[15]]),
            10
        );
    }

    #[test]
    fn block_requests_produce_sequential_progress() {
        let mut transfer = OtaTransfer::new(image());
        for (offset, expected_len) in [(0u32, 4usize), (4, 4), (8, 2)] {
            let request = ImageBlockRequest {
                field_control: 0,
                manufacturer_code: 0x1037,
                image_type: 0x0001,
                file_version: 0x01020304,
                file_offset: offset,
                max_data_size: 4,
                minimum_block_period: None,
            };
            let response = transfer.handle_block_request(&request).unwrap();
            assert_eq!(response[3], STATUS_SUCCESS);
            assert_eq!(response[16] as usize, expected_len);
        }
        assert!(transfer.is_complete());
        assert!((transfer.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wrong_image_identity_is_rejected() {
        let mut transfer = OtaTransfer::new(image());
        let request = ImageBlockRequest {
            field_control: 0,
            manufacturer_code: 0x1037,
            image_type: 0x0001,
            file_version: 0xFFFFFFFF,
            file_offset: 0,
            max_data_size: 4,
            minimum_block_period: None,
        };
        assert!(transfer.handle_block_request(&request).is_err());
        assert!(!transfer.is_complete());
        assert_eq!(transfer.progress(), 0.0);
    }

    #[test]
    fn upgrade_end_requires_complete_transfer_and_matching_identity() {
        let mut transfer = OtaTransfer::new(image());
        let request = UpgradeEndRequest {
            manufacturer_code: 0x1037,
            image_type: 0x0001,
            file_version: 0x01020304,
        };
        assert!(transfer.handle_upgrade_end(&request).is_err());

        let block = ImageBlockRequest {
            field_control: 0,
            manufacturer_code: 0x1037,
            image_type: 0x0001,
            file_version: 0x01020304,
            file_offset: 0,
            max_data_size: 10,
            minimum_block_period: None,
        };
        transfer.handle_block_request(&block).unwrap();
        assert!(transfer.is_complete());
        let response = transfer.handle_upgrade_end(&request).unwrap();
        assert_eq!(response[2], CMD_UPGRADE_END_RESPONSE);
    }

    #[test]
    fn parses_block_and_upgrade_end_requests() {
        let block = vec![
            0x01, 0x05, 0x03, 0x00, 0x37, 0x10, 0x01, 0x00, 0x04, 0x03, 0x02, 0x01, 0x04, 0x00,
            0x00, 0x00, 0x20,
        ];
        let parsed = parse_image_block_request(&block).unwrap();
        assert_eq!(parsed.file_offset, 4);
        assert_eq!(parsed.max_data_size, 0x20);

        let end = vec![
            0x01, 0x06, 0x06, 0x00, 0x37, 0x10, 0x01, 0x00, 0x04, 0x03, 0x02, 0x01,
        ];
        let parsed = parse_upgrade_end_request(&end).unwrap();
        assert_eq!(parsed.manufacturer_code, 0x1037);
    }

    #[test]
    fn fake_coordinator_replays_query_blocks_and_end() {
        let image = image();
        let query = vec![
            0x01, 0x01, 0x01, 0x01, 0x37, 0x10, 0x01, 0x00, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00,
        ];
        let (_, request) = parse_query_next_image_request(&query).unwrap();
        assert_eq!(request.manufacturer_code, image.manufacturer_code);
        let response = query_next_image_response(1, &image);
        assert_eq!(response[3], STATUS_SUCCESS);

        let mut transfer = OtaTransfer::new(image);
        let mut offset = 0u32;
        while offset < 10 {
            let request = ImageBlockRequest {
                field_control: 0,
                manufacturer_code: 0x1037,
                image_type: 1,
                file_version: 0x01020304,
                file_offset: offset,
                max_data_size: 4,
                minimum_block_period: None,
            };
            let response = transfer.handle_block_request(&request).unwrap();
            offset += response[16] as u32;
        }
        let end = UpgradeEndRequest {
            manufacturer_code: 0x1037,
            image_type: 1,
            file_version: 0x01020304,
        };
        let response = transfer.handle_upgrade_end(&end).unwrap();
        assert_eq!(response[2], CMD_UPGRADE_END_RESPONSE);
    }
}
