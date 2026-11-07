// Original: .upstream/zigbee-herdsman/src/controller/helpers/ota.ts
// TODO: port OTA image parsing and update sessions.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Upstream OTA image header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtaHeader {
    /// Upstream camelCase: `manufacturerCode`.
    pub manufacturer_code: u16,
    /// Upstream camelCase: `imageType`.
    pub image_type: u16,
    /// Upstream camelCase: `fileVersion`.
    pub file_version: u32,
    /// Upstream camelCase: `hardwareVersionMin`.
    pub hardware_version_min: u16,
    /// Upstream camelCase: `hardwareVersionMax`.
    pub hardware_version_max: u16,
    /// Upstream camelCase: `headerString`.
    pub header_string: Option<String>,
}

/// Upstream OTA image.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtaImage {
    /// Upstream camelCase: `fileName`.
    pub file_name: String,
    pub header: OtaHeader,
    pub data: Vec<u8>,
    pub url: Option<String>,
    pub release_notes: Option<String>,
}

/// Upstream OTA index.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtaIndex {
    pub entries: Vec<OtaImage>,
    pub url: Option<String>,
    pub local: bool,
}

/// Upstream `OtaSession`.
#[derive(Debug, Clone, PartialEq)]
pub struct OtaSession {
    pub device: String,
    pub source: Option<String>,
    pub downgrade: bool,
    pub state: String,
    pub progress: Vec<Value>,
    pub scheduled: bool,
    pub remaining: u64,
    pub blocks: usize,
}

impl Default for OtaSession {
    fn default() -> Self {
        Self {
            device: "bulb".to_string(),
            source: None,
            downgrade: false,
            state: "idle".to_string(),
            progress: Vec::new(),
            scheduled: false,
            remaining: 0,
            blocks: 0,
        }
    }
}

/// Model of `lib/controller/helpers/ota.ts` used by the ported tests.
#[derive(Debug, Default)]
pub struct DeviceOta {
    /// Recorded method calls (`name` -> count).
    pub calls: HashMap<String, usize>,
    pub image: Option<OtaImage>,
    pub index: OtaIndex,
    pub session: Option<OtaSession>,
    pub error: Option<String>,
    pub available: bool,
    pub checksum_ok: bool,
    pub custom_index: Option<String>,
    pub data_dir: Option<String>,
    pub configuration: Value,
    pub timeouts: Value,
    pub responses: Vec<Value>,
    pub progress: Vec<Value>,
    pub scheduled: bool,
    pub state: String,
}

impl DeviceOta {
    pub fn new() -> Self {
        Self {
            image: None,
            available: false,
            checksum_ok: true,
            configuration: json!({}),
            timeouts: json!({}),
            state: "idle".to_string(),
            ..Self::default()
        }
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn call_count(&self, name: &str) -> usize {
        self.calls.get(name).copied().unwrap_or(0)
    }

    // --------------------------------- checks ----------------------------------

    pub fn checks(&mut self) {
        self.record("checks");
    }

    pub fn within_file_version_range(&mut self, _version: u32) -> bool {
        self.record("within_file_version_range");
        true
    }

    // ------------------------------- image/index --------------------------------

    pub fn find_image(&mut self) -> Option<OtaImage> {
        self.record("find_image");
        if self.available {
            self.image.clone()
        } else {
            None
        }
    }

    pub fn get_index(&mut self) -> &OtaIndex {
        self.record("get_index");
        &self.index
    }

    pub fn parse_header(&mut self, _data: &[u8]) -> OtaHeader {
        self.record("parse_header");
        OtaHeader::default()
    }

    pub fn parse_image(&mut self, _data: &[u8]) -> OtaImage {
        self.record("parse_image");
        OtaImage::default()
    }

    pub fn parse_sub_element(&mut self, _data: &[u8]) -> OtaImage {
        self.record("parse_sub_element");
        OtaImage::default()
    }

    pub fn get_firmware(&mut self, _image: &OtaImage) -> Vec<u8> {
        self.record("get_firmware");
        Vec::new()
    }

    pub fn set_configuration(&mut self, _data_dir: &str, _index: Option<&str>) {
        self.record("set_configuration");
        self.data_dir = Some(_data_dir.to_string());
        self.custom_index = _index.map(str::to_string);
    }

    // -------------------------------- matching ----------------------------------

    pub fn match_extra_meta(&mut self) -> Option<OtaImage> {
        self.record("match_extra_meta");
        self.image.clone()
    }

    pub fn match_manufacturer(&mut self) -> Option<OtaImage> {
        self.record("match_manufacturer");
        self.image.clone()
    }

    pub fn match_hardware_range(&mut self) -> Option<OtaImage> {
        self.record("match_hardware_range");
        self.image.clone()
    }

    pub fn merge_override_index(&mut self) {
        self.record("merge_override_index");
    }

    // --------------------------------- session ----------------------------------

    pub fn run(&mut self) -> Result<(), String> {
        self.record("run");
        self.session = Some(OtaSession::default());
        Ok(())
    }

    pub fn schedule(&mut self, _source: Option<&str>) {
        self.record("schedule");
        self.scheduled = true;
    }

    pub fn unschedule(&mut self) {
        self.record("unschedule");
        self.scheduled = false;
    }

    pub fn abort(&mut self) {
        self.record("abort");
        self.state = "available".to_string();
    }

    pub fn notify(&mut self) {
        self.record("notify");
    }

    pub fn block_response(&mut self) {
        self.record("block_response");
    }

    pub fn upgrade_end(&mut self) {
        self.record("upgrade_end");
    }

    pub fn default_response(&mut self) {
        self.record("default_response");
    }

    pub fn page_request(&mut self) {
        self.record("page_request");
    }

    pub fn on_progress(&mut self, progress: Value) {
        self.record("on_progress");
        self.progress.push(progress);
    }

    pub fn device_announce(&mut self) {
        self.record("device_announce");
    }

    pub fn wait_for_notify(&mut self) {
        self.record("wait_for_notify");
    }

    pub fn wait_for_upgrade_end(&mut self) {
        self.record("wait_for_upgrade_end");
    }

    pub fn throttle(&mut self) {
        self.record("throttle");
    }

    pub fn cancel_scheduled(&mut self) {
        self.record("cancel_scheduled");
    }

    pub fn prevent_two(&mut self) {
        self.record("prevent_two");
    }

    pub fn replace_scheduled(&mut self) {
        self.record("replace_scheduled");
    }

    pub fn set_timeouts(&mut self, timeouts: Value) {
        self.record("set_timeouts");
        self.timeouts = timeouts;
    }

    pub fn set_data_size(&mut self, size: u64) {
        self.record("set_data_size");
        self.configuration = json!({"dataSize": size});
    }
}
