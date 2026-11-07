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
    pub async fn connect(&mut self, config: Value) {
        let path = config
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                config
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        self.ports.push(SerialPortOptions {
            path,
            baud_rate: 115200,
            rtscts: false,
            auto_open: false,
            parity: "none".to_string(),
            stop_bits: 1,
            xon: true,
            xoff: true,
        });
        self.pipe_calls += 1;
        self.async_open_calls += 1;
        self.once_calls += 1;
        self.write_buffer_calls += 1;
    }

    /// Upstream `sendDATA(frame)`; at most 8 frames are in flight at once.
    pub async fn send_data(&mut self, frame: Vec<u8>) {
        if self.sent.len() >= 8 {
            return;
        }
        if self.sent.is_empty() {
            self.write_buffer_calls += 1;
        }
        self.sent.push(frame);
    }

    pub fn close(&mut self) {
        self.sent.clear();
    }

    pub fn is_initialized(&self) -> bool {
        !self.ports.is_empty()
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
