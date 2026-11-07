// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/znp.ts
// TODO: port Znp.
//
// The tests mock SerialPort/parser/writer; the Rust port records those calls
// on `MockSerialPort` and exposes event hooks instead of node streams.

use std::collections::BTreeMap;

use super::super::unpi::constants::{Subsystem, Type};
use super::super::unpi::frame::Frame;
use super::zpi_object::ZpiObject;

#[derive(Debug, Clone, Default)]
pub struct MockSerialPort {
    /// `new SerialPort({path, autoOpen, baudRate, rtscts})` calls.
    pub constructor_calls: Vec<(String, u32, bool)>,
    pub pipe_calls: usize,
    pub async_open_calls: usize,
    pub once_calls: usize,
    pub write_buffer_calls: usize,
    pub flush_and_close_calls: usize,
    pub close_calls: usize,
    pub is_open: bool,
    pub socket_connect_calls: usize,
    pub socket_connect: Option<(u16, String)>,
    pub socket_destroy_calls: usize,
}

pub struct Znp {
    pub path: String,
    pub baud_rate: u32,
    pub rtscts: bool,
    pub serial: MockSerialPort,
    pub initialized: bool,
    pub close_calls: usize,

    // Test hooks mirroring the mocks in znp.test.ts.
    pub async_open_error: Option<String>,
    pub flush_and_close_error: Option<String>,
    pub socket_connect_error: bool,
    pub request_error: Option<String>,

    pub received: Vec<ZpiObject>,
    pub tcp_params: BTreeMap<String, String>,
    /// Frames passed to the UNPI writer.
    pub written_frames: Vec<Frame>,
}

/// Upstream `waitress.waitFor(...)` result.
pub struct Waiter {
    pub id: u64,
    pub r#type: Type,
    pub subsystem: Subsystem,
    pub command: String,
    pub target: Option<serde_json::Value>,
    pub transid: Option<u8>,
    pub state: Option<u8>,
}

pub struct WaiterStart {
    pub promise:
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<ZpiObject, String>> + Send>>,
}

impl Waiter {
    /// Upstream: `waiter.start().promise`.
    pub fn start(&self) -> WaiterStart {
        todo!()
    }
}

impl Znp {
    pub fn new(path: &str, baud_rate: Option<u32>, rtscts: Option<bool>) -> Self {
        Self {
            path: path.to_string(),
            baud_rate: baud_rate.unwrap_or(115200),
            rtscts: rtscts.unwrap_or(false),
            serial: MockSerialPort::default(),
            initialized: false,
            close_calls: 0,
            async_open_error: None,
            flush_and_close_error: None,
            socket_connect_error: false,
            request_error: None,
            received: Vec::new(),
            tcp_params: BTreeMap::new(),
            written_frames: Vec::new(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Upstream: `znp.on("close", cb)`.
    pub fn on_close(&mut self) {}

    /// Upstream: `znp.open()`.
    pub async fn open(&mut self) -> Result<(), String> {
        todo!()
    }

    /// Upstream: `znp.close()`.
    pub async fn close(&mut self) -> Result<(), String> {
        todo!()
    }

    /// Upstream: `znp.on("received", cb)` fed by the UNPI parser.
    pub fn receive_frame(&mut self, frame: &Frame) {
        todo!()
    }

    /// Upstream serial port `close` event.
    pub fn emit_serial_close(&mut self) {
        todo!()
    }

    /// Upstream serial port `error` event (no-op).
    pub fn emit_serial_error(&mut self) {
        todo!()
    }

    /// Upstream: `znp.waitFor(type, subsystem, command, target?, transid?, state?)`.
    pub fn wait_for(
        &mut self,
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        target: Option<serde_json::Value>,
        transid: Option<u8>,
        state: Option<u8>,
    ) -> Waiter {
        todo!()
    }

    /// Upstream `waitress.waiters.size`.
    pub fn waiters_size(&self) -> usize {
        todo!()
    }

    /// Upstream: `znp.request(...)`. SREQ returns the reply, AREQ returns `None`.
    pub async fn request(
        &mut self,
        subsystem: u16,
        command: &str,
        payload: serde_json::Value,
        waiter_id: Option<u64>,
    ) -> Result<Option<ZpiObject>, String> {
        todo!()
    }

    /// Upstream: `znp.requestWithReply(...)`.
    pub async fn request_with_reply(
        &mut self,
        subsystem: u16,
        command: &str,
        payload: serde_json::Value,
    ) -> Result<ZpiObject, String> {
        todo!()
    }

    /// Upstream: `znp.requestZdo(clusterId, payload, waiterId)`.
    pub async fn request_zdo(
        &mut self,
        cluster_id: u16,
        payload: Vec<u8>,
        waiter_id: Option<u64>,
    ) -> Result<(), String> {
        todo!()
    }
}
