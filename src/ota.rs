/// OTA orchestration: parse an OTA image file and drive a transfer from ZCL
/// queries/blocks to the upgrade end.
use std::collections::HashMap;

use crate::zigbee::zcl::clusters::ota::{self, OtaImage, OtaTransfer};

const OTA_MAGIC: u32 = 0x0BEE_F11E;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtaImageInfo {
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct OtaAction {
    pub response: Vec<u8>,
    pub progress: Option<f64>,
    pub completed: bool,
}

#[derive(Default)]
pub struct OtaManager {
    transfers: HashMap<String, OtaTransfer>,
    images: HashMap<String, OtaImage>,
}

/// Parse the 56-byte OTA file header used by Zigbee OTA images.
pub fn parse_image(bytes: &[u8]) -> Option<OtaImage> {
    if bytes.len() < 56 {
        return None;
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != OTA_MAGIC {
        return None;
    }
    let header_length = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let image_size = u32::from_le_bytes([bytes[52], bytes[53], bytes[54], bytes[55]]) as usize;
    if header_length < 56 || header_length > bytes.len() {
        return None;
    }
    if image_size > bytes.len() {
        return None;
    }
    Some(OtaImage {
        manufacturer_code: u16::from_le_bytes([bytes[10], bytes[11]]),
        image_type: u16::from_le_bytes([bytes[12], bytes[13]]),
        file_version: u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]),
        bytes: bytes[..image_size].to_vec(),
    })
}

impl OtaManager {
    pub fn check(&self, bytes: &[u8]) -> Result<OtaImageInfo, String> {
        let image = parse_image(bytes).ok_or_else(|| "invalid OTA image header".to_string())?;
        Ok(OtaImageInfo {
            manufacturer_code: image.manufacturer_code,
            image_type: image.image_type,
            file_version: image.file_version,
            size: image.bytes.len() as u32,
        })
    }

    pub fn start(&mut self, ieee: &str, bytes: &[u8]) -> Result<OtaImageInfo, String> {
        let image = parse_image(bytes).ok_or_else(|| "invalid OTA image header".to_string())?;
        let info = OtaImageInfo {
            manufacturer_code: image.manufacturer_code,
            image_type: image.image_type,
            file_version: image.file_version,
            size: image.bytes.len() as u32,
        };
        self.images.insert(ieee.to_string(), image.clone());
        self.transfers
            .insert(ieee.to_string(), OtaTransfer::new(image));
        Ok(info)
    }

    pub fn abort(&mut self, ieee: &str) -> bool {
        self.images.remove(ieee);
        self.transfers.remove(ieee).is_some()
    }

    #[must_use]
    pub fn progress(&self, ieee: &str) -> Option<f64> {
        self.transfers.get(ieee).map(OtaTransfer::progress)
    }

    /// Handle an incoming OTA cluster frame from the device.
    #[must_use]
    pub fn handle_zcl(&mut self, ieee: &str, raw: &[u8], tsn: u8) -> Option<OtaAction> {
        if let Some((_, _request)) = ota::parse_query_next_image_request(raw) {
            let image = self.images.get(ieee)?;
            return Some(OtaAction {
                response: ota::query_next_image_response(tsn, image),
                progress: Some(0.0),
                completed: false,
            });
        }
        if let Some(request) = ota::parse_image_block_request(raw) {
            let transfer = self.transfers.get_mut(ieee)?;
            let response = transfer.handle_block_request(&request).ok()?;
            return Some(OtaAction {
                response,
                progress: Some(transfer.progress()),
                completed: transfer.is_complete(),
            });
        }
        if let Some(request) = ota::parse_upgrade_end_request(raw) {
            let transfer = self.transfers.get_mut(ieee)?;
            let response = transfer.handle_upgrade_end(&request).ok()?;
            self.images.remove(ieee);
            self.transfers.remove(ieee);
            return Some(OtaAction {
                response,
                progress: Some(1.0),
                completed: true,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_bytes() -> Vec<u8> {
        let image_size = 64usize;
        let mut bytes = vec![0u8; image_size];
        bytes[0..4].copy_from_slice(&OTA_MAGIC.to_le_bytes());
        bytes[4..6].copy_from_slice(&0x0100u16.to_le_bytes()); // header version
        bytes[6..8].copy_from_slice(&56u16.to_le_bytes()); // header length
        bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // field control
        bytes[10..12].copy_from_slice(&0x1037u16.to_le_bytes());
        bytes[12..14].copy_from_slice(&0x0001u16.to_le_bytes());
        bytes[14..18].copy_from_slice(&0x01020304u32.to_le_bytes());
        bytes[18..20].copy_from_slice(&2u16.to_le_bytes()); // stack version
        bytes[20..36].copy_from_slice(b"rust-ota-fixture");
        bytes[52..56].copy_from_slice(&(image_size as u32).to_le_bytes());
        bytes
    }

    #[test]
    fn parses_header_and_rejects_bad_images() {
        let image = parse_image(&image_bytes()).unwrap();
        assert_eq!(image.manufacturer_code, 0x1037);
        assert_eq!(image.image_type, 1);
        assert_eq!(image.file_version, 0x01020304);
        assert_eq!(image.bytes.len(), 64);
        assert!(parse_image(&[0u8; 10]).is_none());
        assert!(parse_image(&[0u8; 56]).is_none());
    }

    #[test]
    fn fake_coordinator_replays_transfer_and_reports_progress() {
        let mut manager = OtaManager::default();
        let info = manager.start("0xec1bbdfffeaa66db", &image_bytes()).unwrap();
        assert_eq!(info.size, 64);

        let query = vec![
            0x01, 0x01, 0x01, 0x01, 0x37, 0x10, 0x01, 0x00, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00,
        ];
        let action = manager.handle_zcl("0xec1bbdfffeaa66db", &query, 1).unwrap();
        assert_eq!(action.response[2], ota::CMD_QUERY_NEXT_IMAGE_RESPONSE);
        assert_eq!(action.progress, Some(0.0));

        let mut offset = 0u32;
        let mut last_progress = 0.0;
        while offset < 64 {
            let mut block = vec![0x01, 7, ota::CMD_IMAGE_BLOCK_REQUEST, 0x00];
            block.extend_from_slice(&0x1037u16.to_le_bytes());
            block.extend_from_slice(&1u16.to_le_bytes());
            block.extend_from_slice(&0x01020304u32.to_le_bytes());
            block.extend_from_slice(&offset.to_le_bytes());
            block.push(32);
            let action = manager.handle_zcl("0xec1bbdfffeaa66db", &block, 7).unwrap();
            assert_eq!(action.response[2], ota::CMD_IMAGE_BLOCK_RESPONSE);
            offset += action.response[16] as u32;
            last_progress = action.progress.unwrap();
        }
        assert!((last_progress - 1.0).abs() < f64::EPSILON);

        let mut end = vec![0x01, 9, ota::CMD_UPGRADE_END_REQUEST, 0x00];
        end.extend_from_slice(&0x1037u16.to_le_bytes());
        end.extend_from_slice(&1u16.to_le_bytes());
        end.extend_from_slice(&0x01020304u32.to_le_bytes());
        let action = manager.handle_zcl("0xec1bbdfffeaa66db", &end, 9).unwrap();
        assert!(action.completed);
        assert_eq!(action.response[2], ota::CMD_UPGRADE_END_RESPONSE);
        assert!(!manager.abort("0xec1bbdfffeaa66db"));
        assert!(manager
            .handle_zcl("0xec1bbdfffeaa66db", &query, 1)
            .is_none());
    }
}
