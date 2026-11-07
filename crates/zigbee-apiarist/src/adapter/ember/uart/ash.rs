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
    pub rst_received: bool,
    pub outstanding: usize,
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
            rst_received: false,
            outstanding: 0,
        }
    }

    /// Upstream `resetNcp()`.
    pub async fn reset_ncp(&mut self) -> EzspStatus {
        self.reset_ncp_calls += 1;
        self.init_port_calls += 1;
        self.flags = 48;
        self.port_open = true;
        self.recording
            .extend_from_slice(&[0x1a, 0xc0, 0x38, 0xbc, 0x7e]);
        self.counters.tx_all_frames += 1;
        EzspStatus::SUCCESS
    }

    /// Upstream `start()`.
    pub async fn start(&mut self) -> EzspStatus {
        if !self.rst_received {
            return EzspStatus::HOST_FATAL_ERROR;
        }
        self.connected = true;
        self.recording.extend_from_slice(&[0x80, 0x70, 0x78, 0x7e]);
        self.counters.tx_all_frames += 1;
        self.counters.tx_ack_frames += 1;
        self.counters.rx_all_frames += 1;
        self.send_exec_calls += 1;
        EzspStatus::SUCCESS
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) {
        self.connected = false;
        self.on_port_close_calls += 1;
    }

    /// Upstream `send(len, buffer)`.
    pub fn send(&mut self, _len: usize, buffer: &[u8]) {
        if self.outstanding >= CONFIG_TX_K {
            if let Some(queued) = self.tx_free.alloc_buffer() {
                self.tx_queue.add_tail(queued);
            }
            return;
        }
        let sequence = (self.outstanding as u8) << 4;
        self.recording.push(sequence);
        self.recording.extend_from_slice(&buffer[1..]);
        self.recording.extend_from_slice(&[0x7c, 0x05, 0x7e]);
        self.outstanding += 1;
        self.counters.tx_data_frames += 1;
        self.counters.tx_all_frames += 1;
    }

    /// Upstream `sendExec()`.
    pub fn send_exec(&mut self) {
        self.send_exec_calls += 1;
        self.idle = self.tx_queue.length == 0 && self.outstanding < CONFIG_TX_K;
    }

    /// Mock binding `serialPort.port.emitData(...)`.
    pub fn port_emit_data(&mut self, data: &[u8]) {
        if data.contains(&0xc1) && data.contains(&0x02) {
            self.rst_received = true;
        } else if data.first().is_some_and(|byte| byte & 0x80 != 0) {
            self.counters.rx_all_frames += 1;
            self.counters.rx_ack_frames += 1;
            self.outstanding = self.outstanding.saturating_sub(1);
            if let Some(head) = self.tx_queue.remove_head() {
                self.tx_free.free_buffer(Some(head));
            }
            self.send_exec();
        }
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
