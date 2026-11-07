// Original: .upstream/zigbee-herdsman/src/adapter/ember/uart/ash.ts
// TODO: port the full public API from the original file.

use serde_json::Value;

use super::queues::{
    EzspFreeList, EzspQueue, EZSP_HOST_RX_POOL_SIZE, EZSP_MAX_FRAME_LENGTH, TX_POOL_BUFFERS,
};
use crate::adapter::ezsp::driver::types::basic::EzspStatus;

/// Upstream `CONFIG_TX_K`.
pub const CONFIG_TX_K: usize = 3;

/// Upstream ASH counters (`readAndClearCounters`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AshCounters {
    /// Upstream camelCase: `txAllFrames`.
    pub tx_all_frames: u32,
    /// Upstream camelCase: `txAckFrames`.
    pub tx_ack_frames: u32,
    /// Upstream camelCase: `txDataFrames`.
    pub tx_data_frames: u32,
    /// Upstream camelCase: `txNakFrames`.
    pub tx_nak_frames: u32,
    /// Upstream camelCase: `rxAllFrames`.
    pub rx_all_frames: u32,
    /// Upstream camelCase: `rxAckFrames`.
    pub rx_ack_frames: u32,
    /// Upstream camelCase: `rxDataFrames`.
    pub rx_data_frames: u32,
    /// Upstream camelCase: `rxNakFrames`.
    pub rx_nak_frames: u32,
    /// Upstream camelCase: `rxCrcErrors`.
    pub rx_crc_errors: u32,
    /// Upstream camelCase: `rxCancels`.
    pub rx_cancels: u32,
    /// Upstream camelCase: `rxTimeouts`.
    pub rx_timeouts: u32,
    /// Upstream camelCase: `txTimeouts`.
    pub tx_timeouts: u32,
}

/// Model of the ASH protocol layer (upstream `UartAsh`).
#[derive(Debug)]
pub struct UartAsh {
    pub connected: bool,
    pub tx_queue: EzspQueue,
    pub re_tx_queue: EzspQueue,
    pub tx_free: EzspFreeList,
    pub rx_queue: EzspQueue,
    pub rx_free: EzspFreeList,
    /// Upstream camelCase: `ncpSleepEnabled`.
    pub ncp_sleep_enabled: bool,
    /// Upstream camelCase: `ncpHasCallbacks`.
    pub ncp_has_callbacks: bool,
    pub counters: AshCounters,
    /// Upstream private `flags` (RST|CAN == 48 after reset).
    pub flags: u8,
    /// Upstream `portOpen`.
    pub port_open: bool,
    pub idle: bool,
    /// Upstream `serialPort.port.recording`.
    pub recording: Vec<u8>,
    pub send_exec_calls: usize,
    pub init_port_calls: usize,
    pub reset_ncp_calls: usize,
    pub on_port_close_calls: usize,
    pub on_port_error_calls: usize,
    pub settings: Value,
}

impl UartAsh {
    /// Upstream `new UartAsh(openOpts)`.
    pub fn new(open_opts: Value) -> Self {
        Self {
            connected: false,
            tx_queue: EzspQueue::new(),
            re_tx_queue: EzspQueue::new(),
            tx_free: EzspFreeList::new(TX_POOL_BUFFERS, EZSP_MAX_FRAME_LENGTH),
            rx_queue: EzspQueue::new(),
            rx_free: EzspFreeList::new(EZSP_HOST_RX_POOL_SIZE, EZSP_MAX_FRAME_LENGTH),
            ncp_sleep_enabled: false,
            ncp_has_callbacks: false,
            counters: AshCounters::default(),
            flags: 0,
            port_open: false,
            idle: false,
            recording: Vec::new(),
            send_exec_calls: 0,
            init_port_calls: 0,
            reset_ncp_calls: 0,
            on_port_close_calls: 0,
            on_port_error_calls: 0,
            settings: open_opts,
        }
    }

    /// Upstream `resetNcp()`.
    pub async fn reset_ncp(&mut self) -> EzspStatus {
        todo!()
    }

    /// Upstream `start()`.
    pub async fn start(&mut self) -> EzspStatus {
        todo!()
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) {
        todo!()
    }

    /// Upstream `send(len, buffer)`.
    pub fn send(&mut self, _len: usize, _buffer: &[u8]) {
        todo!()
    }

    /// Upstream `sendExec()`.
    pub fn send_exec(&mut self) {
        todo!()
    }

    /// Mock binding `serialPort.port.emitData(...)`.
    pub fn port_emit_data(&mut self, _data: &[u8]) {
        todo!()
    }
}

pub fn config_tx_k() {
    todo!()
}
pub fn uart_ash() {
    todo!()
}
pub fn close_port() {
    todo!()
}
pub fn get_frame_type() {
    todo!()
}
pub fn randomize_buffer() {
    todo!()
}
pub fn read_and_clear_counters() {
    todo!()
}
pub fn reset_ncp() {
    todo!()
}
pub fn send() {
    todo!()
}
pub fn send_exec() {
    todo!()
}
pub fn start() {
    todo!()
}
pub fn stop() {
    todo!()
}
