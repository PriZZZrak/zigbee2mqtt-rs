/// OTA orchestration: parse an OTA image file and drive a transfer from ZCL
/// queries/blocks to the upgrade end.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Deserialize;

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

const MAGIC_BYTES: [u8; 4] = [0x1E, 0xF1, 0xEE, 0x0B];

const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

fn sha512_digest(data: &[u8]) -> [u8; 64] {
    let mut state: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 128 != 112 {
        message.push(0);
    }
    let bit_len = (data.len() as u128) * 8;
    message.extend_from_slice(&bit_len.to_be_bytes());
    for block in 0..message.len() / 128 {
        let chunk = &message[block * 128..(block + 1) * 128];
        let mut w = [0u64; 80];
        for index in 0..16 {
            let word = &chunk[index * 8..index * 8 + 8];
            w[index] = u64::from_be_bytes([
                word[0], word[1], word[2], word[3], word[4], word[5], word[6], word[7],
            ]);
        }
        for index in 16..80 {
            let s0 = w[index - 15].rotate_right(1)
                ^ w[index - 15].rotate_right(8)
                ^ (w[index - 15] >> 7);
            let s1 =
                w[index - 2].rotate_right(19) ^ w[index - 2].rotate_right(61) ^ (w[index - 2] >> 6);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for index in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA512_K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    let mut out = [0u8; 64];
    for (index, word) in state.iter().enumerate() {
        out[index * 8..index * 8 + 8].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[must_use]
pub fn sha512_hex(data: &[u8]) -> String {
    sha512_digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Validate a firmware file: verify SHA-512 when provided and strip any vendor
/// prefix before the OTA magic.
pub fn validate_firmware(data: &[u8], expected_sha512: Option<&str>) -> Result<Vec<u8>, String> {
    if let Some(expected) = expected_sha512 {
        let actual = sha512_hex(data);
        if !actual.eq_ignore_ascii_case(expected.trim()) {
            return Err("File checksum validation failed".into());
        }
    }
    let offset = data
        .windows(MAGIC_BYTES.len())
        .position(|window| window == MAGIC_BYTES)
        .ok_or_else(|| "Not a valid OTA file".to_string())?;
    Ok(data[offset..].to_vec())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtaIndexEntry {
    #[serde(default)]
    pub file_name: Option<String>,
    pub file_version: u32,
    #[serde(default)]
    pub file_size: Option<u32>,
    pub url: String,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub manufacturer_code: Option<u16>,
    #[serde(default)]
    pub image_type: Option<u16>,
    #[serde(default)]
    pub min_file_version: Option<u32>,
    #[serde(default)]
    pub max_file_version: Option<u32>,
    #[serde(default)]
    pub hardware_version_min: Option<u16>,
    #[serde(default)]
    pub hardware_version_max: Option<u16>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub manufacturer_name: Option<Vec<String>>,
    #[serde(default)]
    pub ota_header_string: Option<String>,
    #[serde(default)]
    pub release_notes: Option<String>,
    #[serde(default)]
    pub force: Option<bool>,
}

pub fn parse_index(json: &str) -> Result<Vec<OtaIndexEntry>, String> {
    serde_json::from_str(json).map_err(|error| format!("invalid OTA index: {error}"))
}

#[derive(Debug, Clone, Copy)]
pub struct OtaQueryIdentity<'a> {
    pub manufacturer_code: u16,
    pub image_type: u16,
    pub file_version: u32,
    pub hardware_version: Option<u16>,
    pub model_id: Option<&'a str>,
    pub manufacturer_name: Option<&'a str>,
}

/// Select the first matching index entry, mirroring herdsman `findMatchingOtaImage`.
#[must_use]
pub fn select_image<'a>(
    entries: &'a [OtaIndexEntry],
    query: &OtaQueryIdentity<'_>,
) -> Option<&'a OtaIndexEntry> {
    entries.iter().find(|entry| {
        entry.image_type == Some(query.image_type)
            && entry.manufacturer_code == Some(query.manufacturer_code)
            && entry
                .min_file_version
                .is_none_or(|minimum| query.file_version >= minimum)
            && entry
                .max_file_version
                .is_none_or(|maximum| query.file_version <= maximum)
            && entry
                .model_id
                .as_deref()
                .is_none_or(|model| Some(model) == query.model_id)
            && entry.manufacturer_name.as_ref().is_none_or(|names| {
                query
                    .manufacturer_name
                    .is_some_and(|name| names.iter().any(|candidate| candidate == name))
            })
            && match (entry.hardware_version_min, query.hardware_version) {
                (Some(minimum), Some(actual)) => actual >= minimum,
                (Some(_), None) => false,
                (None, _) => true,
            }
            && match (entry.hardware_version_max, query.hardware_version) {
                (Some(maximum), Some(actual)) => actual <= maximum,
                (Some(_), None) => false,
                (None, _) => true,
            }
    })
}

/// Per-device automatic OTA check throttling (`ota.update_check_interval`).
pub struct OtaCheckScheduler {
    interval: Duration,
    last_checked: HashMap<String, Instant>,
}

impl OtaCheckScheduler {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_checked: HashMap::new(),
        }
    }

    #[must_use]
    pub fn should_check(&self, ieee: &str, now: Instant) -> bool {
        self.last_checked
            .get(ieee)
            .is_none_or(|last| now.saturating_duration_since(*last) > self.interval)
    }

    pub fn mark_checked(&mut self, ieee: &str, now: Instant) {
        self.last_checked.insert(ieee.to_string(), now);
    }

    pub fn forget(&mut self, ieee: &str) {
        self.last_checked.remove(ieee);
    }
}

/// Bounded retry policy with backoff for OTA check/start failures.
pub struct OtaRetryPolicy {
    max_attempts: u8,
    backoff: Duration,
    attempts: u8,
    next_attempt: Option<Instant>,
}

impl OtaRetryPolicy {
    #[must_use]
    pub fn new(max_attempts: u8, backoff: Duration) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            backoff,
            attempts: 0,
            next_attempt: None,
        }
    }

    #[must_use]
    pub fn can_attempt(&self, now: Instant) -> bool {
        self.next_attempt.is_none_or(|next| now >= next)
    }

    /// Record a failure. Returns true when another attempt is allowed.
    pub fn record_failure(&mut self, now: Instant) -> bool {
        self.attempts = self.attempts.saturating_add(1);
        if self.attempts >= self.max_attempts {
            return false;
        }
        self.next_attempt = Some(now + self.backoff);
        true
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
        self.next_attempt = None;
    }

    #[must_use]
    pub fn attempts(&self) -> u8 {
        self.attempts
    }
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

    /// Validate checksum/prefix and inspect the image without starting a transfer.
    pub fn check_checked(
        &self,
        bytes: &[u8],
        expected_sha512: Option<&str>,
    ) -> Result<OtaImageInfo, String> {
        let stripped = validate_firmware(bytes, expected_sha512)?;
        self.check(&stripped)
    }

    /// Validate checksum/prefix and start a transfer from the stripped image.
    pub fn start_checked(
        &mut self,
        ieee: &str,
        bytes: &[u8],
        expected_sha512: Option<&str>,
    ) -> Result<OtaImageInfo, String> {
        let stripped = validate_firmware(bytes, expected_sha512)?;
        self.start(ieee, &stripped)
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

    #[test]
    fn sha512_matches_known_vector() {
        assert_eq!(
            sha512_hex(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn validate_firmware_checks_sha512_and_strips_prefix() {
        let image = image_bytes();
        let mut prefixed = vec![0xAA, 0xBB, 0xCC];
        prefixed.extend_from_slice(&image);
        let digest = sha512_hex(&prefixed);
        let stripped = validate_firmware(&prefixed, Some(&digest)).unwrap();
        assert_eq!(stripped, image);
        assert!(validate_firmware(&image, Some("00")).is_err());
        assert!(validate_firmware(&[0u8; 4], None).is_err());
    }

    #[test]
    fn index_parsing_and_selection_filters_identity_and_hardware() {
        let index = r#"[
            {"fileName":"fw.ota","fileVersion":5,"url":"fw.ota","sha512":"aa","manufacturerCode":4151,"imageType":1,"minFileVersion":3,"maxFileVersion":6,"hardwareVersionMin":1,"hardwareVersionMax":3,"modelId":"M","manufacturerName":["Vendor"],"releaseNotes":"notes"}
        ]"#;
        let entries = parse_index(index).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].release_notes.as_deref(), Some("notes"));
        let identity = OtaQueryIdentity {
            manufacturer_code: 0x1037,
            image_type: 1,
            file_version: 4,
            hardware_version: Some(2),
            model_id: Some("M"),
            manufacturer_name: Some("Vendor"),
        };
        assert!(select_image(&entries, &identity).is_some());
        let wrong_hardware = OtaQueryIdentity {
            hardware_version: Some(9),
            ..identity
        };
        assert!(select_image(&entries, &wrong_hardware).is_none());
        let wrong_model = OtaQueryIdentity {
            model_id: Some("Other"),
            ..identity
        };
        assert!(select_image(&entries, &wrong_model).is_none());
        let old_version = OtaQueryIdentity {
            file_version: 7,
            ..identity
        };
        assert!(select_image(&entries, &old_version).is_none());
    }

    #[test]
    fn check_scheduler_throttles_by_interval() {
        let mut scheduler = OtaCheckScheduler::new(Duration::from_secs(3600));
        let now = Instant::now();
        assert!(scheduler.should_check("0x01", now));
        scheduler.mark_checked("0x01", now);
        assert!(!scheduler.should_check("0x01", now + Duration::from_secs(1800)));
        assert!(scheduler.should_check("0x01", now + Duration::from_secs(3601)));
        scheduler.forget("0x01");
        assert!(scheduler.should_check("0x01", now));
    }

    #[test]
    fn retry_policy_bounds_attempts_and_backoff() {
        let mut retry = OtaRetryPolicy::new(2, Duration::from_secs(30));
        let now = Instant::now();
        assert!(retry.can_attempt(now));
        assert!(retry.record_failure(now));
        assert!(!retry.can_attempt(now + Duration::from_secs(10)));
        assert!(retry.can_attempt(now + Duration::from_secs(30)));
        assert!(!retry.record_failure(now + Duration::from_secs(30)));
        assert_eq!(retry.attempts(), 2);
        retry.reset();
        assert!(retry.can_attempt(now));
    }
}
