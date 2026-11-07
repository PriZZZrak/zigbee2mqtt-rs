// Original: .upstream/zigbee-herdsman/src/adapter/serialPort.ts
// TODO: port public API from the original file.

/// Upstream `SerialPort.list()` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SerialPortInfo {
    pub path: String,
    /// Upstream camelCase: `vendorId`.
    pub vendor_id: Option<String>,
    /// Upstream camelCase: `productId`.
    pub product_id: Option<String>,
    pub manufacturer: Option<String>,
    /// Upstream camelCase: `pnpId`.
    pub pnp_id: Option<String>,
    /// Upstream camelCase: `friendlyName`.
    pub friendly_name: Option<String>,
    /// Upstream camelCase: `serialNumber`.
    pub serial_number: Option<String>,
    /// Upstream camelCase: `locationId`.
    pub location_id: Option<String>,
}

impl SerialPortInfo {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            ..Self::default()
        }
    }
}

pub fn serial_port() {
    todo!()
}
pub fn async_close() {
    todo!()
}
pub fn async_flush() {
    todo!()
}
pub fn async_flush_and_close() {
    todo!()
}
pub fn async_get() {
    todo!()
}
pub fn async_open() {
    todo!()
}
pub fn async_set() {
    todo!()
}
