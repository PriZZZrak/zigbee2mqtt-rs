// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/uart.ts
// TODO: port public API from the original file.

use serde_json::Value;

/// Options passed to `SerialPort` (upstream `SerialPortOptions`).
#[derive(Debug, Clone, PartialEq)]
pub struct SerialPortOptions {
    pub path: String,
    pub baud_rate: u32,
    pub rtscts: bool,
    pub auto_open: bool,
    pub parity: String,
    pub stop_bits: u8,
    pub xon: bool,
    pub xoff: bool,
}

/// Model of the EZSP serial driver (upstream `SerialDriver`).
#[derive(Debug, Default)]
pub struct SerialDriver {
    pub ports: Vec<SerialPortOptions>,
    pub pipe_calls: usize,
    pub async_open_calls: usize,
    pub once_calls: usize,
    pub write_buffer_calls: usize,
    /// Data frames passed to `sendDATA`.
    pub sent: Vec<Vec<u8>>,
}

impl SerialDriver {
    /// Upstream `new SerialDriver()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `connect(config)`.
    pub async fn connect(&mut self, _config: Value) {
        todo!()
    }

    /// Upstream `sendDATA(frame)`; at most 8 frames are in flight at once.
    pub async fn send_data(&mut self, _frame: Vec<u8>) {
        todo!()
    }

    pub fn close(&mut self) {
        todo!()
    }

    pub fn is_initialized(&self) -> bool {
        todo!()
    }
}

pub fn serial_driver() {
    todo!()
}
pub fn close() {
    todo!()
}
pub fn is_initialized() {
    todo!()
}
pub fn send_data_deprecated() {
    todo!()
}
pub fn wait_for() {
    todo!()
}
