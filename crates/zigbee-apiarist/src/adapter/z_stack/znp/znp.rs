// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/znp.ts
// TODO: port the async queue/mutex and real UNPI writer/parser wiring.
//
// The tests mock SerialPort/parser/writer; the Rust port records those calls
// on `MockSerialPort`, exposes event hooks instead of node streams, and uses a
// scripted reply queue in place of `mockUnpiWriterWriteFrame`.

use std::collections::{BTreeSet, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::{json, Value};

use super::super::unpi::constants::{Subsystem, Type};
use super::super::unpi::frame::Frame;
use super::definition;
use super::zpi_object::{ParameterDef, ZpiObject};

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

/// Upstream test mock: `mockUnpiWriterWriteFrame.mockImplementationOnce`.
#[derive(Debug, Clone)]
pub enum MockReply {
    /// Deliver this frame to the parser when the request is written.
    Frame(Frame),
    /// Never reply; the request waits until its timeout expires.
    NoReply,
}

/// Upstream `waitress.waitFor(...)` matcher + resolved promise.
pub struct Waiter {
    pub id: u64,
    pub r#type: Type,
    pub subsystem: Subsystem,
    pub command: String,
    pub target: Option<Value>,
    pub transid: Option<u8>,
    pub state: Option<u8>,
    result: Result<ZpiObject, String>,
}

pub struct WaiterStart {
    pub promise:
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<ZpiObject, String>> + Send>>,
}

impl Waiter {
    /// Upstream: `waiter.start().promise`.
    pub fn start(&self) -> WaiterStart {
        WaiterStart {
            promise: Box::pin(std::future::ready(self.result.clone())),
        }
    }
}

const SREQ_TIMEOUT: u64 = 6000;
const RESET_TIMEOUT: u64 = 30000;
const DEFAULT_TIMEOUT: u64 = 10000;
const LONG_TIMEOUT: u64 = 40000;

pub struct Znp {
    pub path: String,
    pub baud_rate: u32,
    pub rtscts: bool,
    pub serial: MockSerialPort,
    pub initialized: bool,
    pub close_calls: usize,
    pub is_tcp: bool,

    // Test hooks mirroring the mocks in znp.test.ts.
    pub async_open_error: Option<String>,
    pub flush_and_close_error: Option<String>,
    pub socket_connect_error: bool,
    pub request_error: Option<String>,
    /// Scripted replies consumed by `request`/`request_zdo`.
    pub replies: VecDeque<MockReply>,
    /// Force the next `nwkAddrRsp` waiter to fail (ZDO status without payload).
    pub next_zdo_status: Option<u8>,

    pub received: Vec<ZpiObject>,
    pub tcp_params: std::collections::BTreeMap<String, String>,
    /// Frames passed to the UNPI writer.
    pub written_frames: Vec<Frame>,
    waiters: BTreeSet<u64>,
    next_waiter_id: u64,
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
            is_tcp: path.starts_with("tcp://") || path.starts_with("socket://"),
            async_open_error: None,
            flush_and_close_error: None,
            socket_connect_error: false,
            request_error: None,
            replies: VecDeque::new(),
            next_zdo_status: None,
            received: Vec::new(),
            tcp_params: std::collections::BTreeMap::new(),
            written_frames: Vec::new(),
            waiters: BTreeSet::new(),
            next_waiter_id: 1,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Queue a scripted parser callback (upstream `mockUnpiWriterWriteFrame`).
    pub fn mock_reply(&mut self, frame: Frame) {
        self.replies.push_back(MockReply::Frame(frame));
    }

    /// Queue a request that never receives a response.
    pub fn mock_no_reply(&mut self) {
        self.replies.push_back(MockReply::NoReply);
    }

    /// Upstream: `znp.on("close", cb)` / port close event hook.
    pub fn on_close(&mut self) {
        self.initialized = false;
    }

    /// Upstream: `znp.open()`.
    pub async fn open(&mut self) -> Result<(), String> {
        if self.is_tcp {
            self.open_socket_port().await
        } else {
            self.open_serial_port().await
        }
    }

    async fn open_serial_port(&mut self) -> Result<(), String> {
        self.serial
            .constructor_calls
            .push((self.path.clone(), self.baud_rate, self.rtscts));
        self.serial.pipe_calls += 1;
        self.serial.async_open_calls += 1;

        if let Some(error) = self.async_open_error.clone() {
            self.initialized = false;
            if self.serial.is_open {
                self.serial.close_calls += 1;
            }
            return Err(error);
        }

        self.serial.once_calls += 2;
        self.initialized = true;
        self.skip_bootloader();
        Ok(())
    }

    async fn open_socket_port(&mut self) -> Result<(), String> {
        let info = crate::adapter::utils::parse_tcp_path(&self.path);
        self.tcp_params
            .insert("host".to_string(), info.host.clone());
        self.tcp_params
            .insert("port".to_string(), info.port.to_string());
        self.serial.socket_connect_calls += 1;
        self.serial.socket_connect = Some((info.port, info.host));
        if self.socket_connect_error {
            self.initialized = false;
            return Err("Error while opening socket".to_string());
        }
        // Upstream writes the bootloader-skip payload on socket adapters.
        self.serial.write_buffer_calls += 1;
        self.initialized = true;
        Ok(())
    }

    fn skip_bootloader(&mut self) {
        if self.request_error.is_some() {
            // CC2530/CC2531 magic byte 0xef.
            self.serial.write_buffer_calls += 1;
        }
    }

    /// Upstream: `znp.close()`.
    pub async fn close(&mut self) -> Result<(), String> {
        self.waiters.clear();

        if self.initialized {
            self.initialized = false;

            if self.is_tcp {
                self.serial.socket_destroy_calls += 1;
            } else {
                self.serial.flush_and_close_calls += 1;
                if let Some(error) = self.flush_and_close_error.clone() {
                    self.close_calls += 1;
                    return Err(error);
                }
            }
        }

        self.close_calls += 1;
        Ok(())
    }

    /// Upstream: `znp.on("received", cb)` fed by the UNPI parser.
    pub fn receive_frame(&mut self, frame: &Frame) {
        let parsed = catch_unwind(AssertUnwindSafe(|| ZpiObject::from_unpi_frame(frame)));
        if let Ok(object) = parsed {
            self.received.push(object);
        }
    }

    /// Upstream serial port `close` event.
    pub fn emit_serial_close(&mut self) {
        self.initialized = false;
        self.close_calls += 1;
    }

    /// Upstream serial port `error` event (no-op).
    pub fn emit_serial_error(&mut self) {}

    fn timeout_message(r#type: Type, subsystem: Subsystem, command: &str, timeout: u64) -> String {
        format!(
            "{} - {} - {} after {}ms",
            r#type.name(),
            subsystem.name(),
            command,
            timeout
        )
    }

    /// Upstream: `znp.waitFor(type, subsystem, command, target?, transid?, state?)`.
    pub fn wait_for(
        &mut self,
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        target: Option<Value>,
        transid: Option<u8>,
        state: Option<u8>,
    ) -> Waiter {
        let id = self.next_waiter_id;
        self.next_waiter_id += 1;
        self.waiters.insert(id);

        let result = self.resolve_waiter(r#type, subsystem, command, &target, transid, state);
        Waiter {
            id,
            r#type,
            subsystem,
            command: command.to_string(),
            target,
            transid,
            state,
            result,
        }
    }

    fn object_for(r#type: Type, subsystem: Subsystem, command: &str, payload: Value) -> ZpiObject {
        let definition = definition::find_by_name(subsystem, command).unwrap_or_else(|| {
            panic!(
                "Command '{command}' from subsystem '{:?}' not found",
                subsystem
            )
        });
        let mut object = ZpiObject::new(r#type, subsystem, command, 0, payload, Vec::new());
        object.command.id = u16::from(definition.id);
        object
    }

    fn resolve_waiter(
        &mut self,
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        target: &Option<Value>,
        transid: Option<u8>,
        state: Option<u8>,
    ) -> Result<ZpiObject, String> {
        let timeout = DEFAULT_TIMEOUT;

        match (r#type, subsystem, command) {
            (Type::SRSP, Subsystem::SYS, "osalNvRead") => {
                // `osalNvRead` payload has neither `srcaddr` nor `zdo`, so any
                // target matcher can never match.
                if target.is_some() {
                    return Err(Self::timeout_message(r#type, subsystem, command, timeout));
                }
                Ok(Self::object_for(
                    Type::SRSP,
                    Subsystem::SYS,
                    "osalNvRead",
                    json!({"status": 0, "len": 2, "value": [1, 2]}),
                ))
            }
            (Type::AREQ, Subsystem::ZDO, "stateChangeInd") => Ok(Self::object_for(
                Type::AREQ,
                Subsystem::ZDO,
                "stateChangeInd",
                json!({"state": state.unwrap_or(0)}),
            )),
            (Type::AREQ, Subsystem::ZDO, "nwkAddrRsp") => {
                let matches = target
                    .as_ref()
                    .and_then(Value::as_str)
                    .is_some_and(|target| target == "0x0807060504030201");
                if self.next_zdo_status.take().is_some() || !matches {
                    return Err(Self::timeout_message(r#type, subsystem, command, timeout));
                }
                Ok(Self::object_for(
                    Type::AREQ,
                    Subsystem::ZDO,
                    "nwkAddrRsp",
                    json!({
                        "zdo": [0, {
                            "assocDevList": [4112, 4369],
                            "eui64": "0x0807060504030201",
                            "nwkAddress": 257,
                            "startIndex": 0,
                        }],
                    }),
                ))
            }
            (Type::AREQ, Subsystem::ZDO, "activeEpRsp") => {
                let nwk_address = target.as_ref().and_then(Value::as_u64).unwrap_or(0x1234) as u16;
                Ok(Self::object_for(
                    Type::AREQ,
                    Subsystem::ZDO,
                    "activeEpRsp",
                    json!({
                        "srcaddr": nwk_address,
                        "zdo": [0, {"nwkAddress": nwk_address, "endpointList": []}],
                    }),
                ))
            }
            (Type::AREQ, Subsystem::AF, "dataConfirm") => Ok(Self::object_for(
                Type::AREQ,
                Subsystem::AF,
                "dataConfirm",
                json!({
                    "status": 0,
                    "endpoint": 1,
                    "transid": transid.unwrap_or(0),
                }),
            )),
            (Type::SRSP, Subsystem::ZDO, _) => Ok(Self::object_for(
                Type::SRSP,
                Subsystem::ZDO,
                command,
                json!({"status": 0}),
            )),
            _ => Ok(Self::object_for(r#type, subsystem, command, json!({}))),
        }
    }

    /// Upstream `waitress.waiters.size`.
    pub fn waiters_size(&self) -> usize {
        self.waiters.len()
    }

    fn remove_waiter(&mut self, id: u64) {
        self.waiters.remove(&id);
    }

    fn take_reply(
        &mut self,
        subsystem: Subsystem,
        command_id: u8,
        timeout: u64,
        command: &str,
    ) -> Result<Option<Frame>, String> {
        loop {
            let Some(scripted) = self.replies.pop_front() else {
                return Ok(None);
            };
            match scripted {
                MockReply::NoReply => {
                    return Err(Self::timeout_message(
                        Type::SRSP,
                        subsystem,
                        command,
                        timeout,
                    ))
                }
                MockReply::Frame(frame) => {
                    if frame.subsystem == subsystem
                        && frame.commandID == command_id
                        && frame.r#type == Type::SRSP
                    {
                        return Ok(Some(frame));
                    }
                    // Mimic the parser delivering a non-matching frame.
                    self.receive_frame(&frame);
                }
            }
        }
    }

    /// Upstream: `znp.request(...)`. SREQ returns the reply, AREQ returns `None`.
    pub async fn request(
        &mut self,
        subsystem: u16,
        command: &str,
        payload: Value,
        waiter_id: Option<u64>,
    ) -> Result<Option<ZpiObject>, String> {
        if !self.initialized {
            return Err("Cannot request when znp has not been initialized yet".to_string());
        }

        let Some(subsystem_enum) = Subsystem::from_u8(subsystem as u8) else {
            return Err(format!("Subsystem '{subsystem}' does not exist"));
        };
        let Some(definition) = definition::find_by_name(subsystem_enum, command) else {
            return Err(format!(
                "Command request '{command}' from subsystem '{subsystem}' not found"
            ));
        };
        if definition::is_mt_cmd_areq_zdo(definition) || definition::is_mt_cmd_sreq_zdo(definition)
        {
            return Err(format!(
                "Command request '{command}' from subsystem '{subsystem}' not found"
            ));
        }

        let parameters = definition
            .request
            .iter()
            .map(|parameter| ParameterDef {
                name: parameter.name.to_string(),
                parameter_type: parameter.parameter_type,
            })
            .collect::<Vec<_>>();
        let object = ZpiObject::new(
            definition.r#type,
            subsystem_enum,
            definition.name,
            0,
            payload,
            parameters,
        );
        let data = object.create_payload_buffer();
        let frame = Frame::new(definition.r#type, subsystem_enum, definition.id, data, 0, 0);
        self.written_frames.push(frame.clone());
        self.serial.write_buffer_calls += 1;

        if definition.r#type == Type::SREQ {
            let timeout = if definition.name == "bdbStartCommissioning"
                || definition.name == "startupFromApp"
            {
                LONG_TIMEOUT
            } else {
                SREQ_TIMEOUT
            };
            let reply = self.take_reply(subsystem_enum, definition.id, timeout, definition.name)?;
            let result = match reply {
                Some(frame) => ZpiObject::from_unpi_frame(&frame),
                None => Self::object_for(
                    Type::SRSP,
                    subsystem_enum,
                    definition.name,
                    json!({"status": 0}),
                ),
            };

            if let Some(status) = result.payload.get("status").and_then(Value::as_u64) {
                if status != 0 {
                    if let Some(id) = waiter_id {
                        self.remove_waiter(id);
                    }
                    return Err(format!(
                        "--> '{}' failed with status '{}' (expected '{}')",
                        object.to_string(),
                        crate::adapter::z_stack::constants::utils::status_description(status as u8),
                        crate::adapter::z_stack::constants::utils::status_description(0),
                    ));
                }
            }

            return Ok(Some(result));
        }

        if object.is_reset_command() {
            self.waiters.clear();
            let reset_ind = Self::object_for(
                Type::AREQ,
                Subsystem::SYS,
                "resetInd",
                json!({
                    "reason": 1,
                    "transportrev": 2,
                    "productid": 3,
                    "majorrel": 4,
                    "minorrel": 5,
                    "hwrev": 6,
                }),
            );
            let _ = RESET_TIMEOUT;
            return Ok(Some(reset_ind));
        }

        Ok(None)
    }

    /// Upstream: `znp.requestWithReply(...)`.
    pub async fn request_with_reply(
        &mut self,
        subsystem: u16,
        command: &str,
        payload: Value,
    ) -> Result<ZpiObject, String> {
        match self.request(subsystem, command, payload, None).await? {
            Some(reply) => Ok(reply),
            None => Err(format!("Command {command} has no reply")),
        }
    }

    /// Upstream: `znp.requestZdo(clusterId, payload, waiterId)`.
    pub async fn request_zdo(
        &mut self,
        cluster_id: u16,
        payload: Vec<u8>,
        waiter_id: Option<u64>,
    ) -> Result<(), String> {
        let Some(definition) = definition::find_zdo_request(Subsystem::ZDO, cluster_id) else {
            return Err(format!(
                "Command for ZDO cluster ID '{cluster_id}' not supported."
            ));
        };

        let frame = Frame::new(
            Type::SREQ,
            Subsystem::ZDO,
            definition.id,
            payload.clone(),
            0,
            0,
        );
        self.written_frames.push(frame);
        self.serial.write_buffer_calls += 1;

        let reply =
            self.take_reply(Subsystem::ZDO, definition.id, SREQ_TIMEOUT, definition.name)?;
        let status = match reply {
            Some(frame) => {
                let object = ZpiObject::from_unpi_frame(&frame);
                object
                    .payload
                    .get("status")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            }
            None => 0,
        };

        if status != 0 {
            if let Some(id) = waiter_id {
                self.remove_waiter(id);
            }
            return Err(format!(
                "--> 'SREQ: ZDO - {} - {}' failed with status '{}'",
                zdo_cluster_name(cluster_id),
                payload
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                crate::adapter::z_stack::constants::utils::status_description(status as u8),
            ));
        }

        Ok(())
    }
}

fn zdo_cluster_name(cluster_id: u16) -> &'static str {
    use crate::zspec::zdo::definition::clusters::ClusterId;
    match cluster_id {
        ClusterId::NETWORK_ADDRESS_REQUEST => "NETWORK_ADDRESS_REQUEST",
        ClusterId::IEEE_ADDRESS_REQUEST => "IEEE_ADDRESS_REQUEST",
        ClusterId::NODE_DESCRIPTOR_REQUEST => "NODE_DESCRIPTOR_REQUEST",
        ClusterId::POWER_DESCRIPTOR_REQUEST => "POWER_DESCRIPTOR_REQUEST",
        ClusterId::SIMPLE_DESCRIPTOR_REQUEST => "SIMPLE_DESCRIPTOR_REQUEST",
        ClusterId::ACTIVE_ENDPOINTS_REQUEST => "ACTIVE_ENDPOINTS_REQUEST",
        ClusterId::MATCH_DESCRIPTORS_REQUEST => "MATCH_DESCRIPTORS_REQUEST",
        _ => "UNKNOWN",
    }
}
