//! Scripted ZNP coordinator for the ported `adapter.test.ts` suite.
//!
//! Upstream `adapter.test.ts` mocks `Znp.request`/`Znp.waitFor` directly. The
//! Rust adapter owns a concrete `Znp`, so the equivalent seam is the serial
//! wire: this harness answers UNPI SREQ frames like the upstream request mock
//! and injects AREQ frames for `waitFor`/event-loop expectations.
//!
//! Anchor: .upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts
//! (`ZnpRequestMockBuilder`, `baseZnpRequestMock`, `basicMocks`).

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex as StdMutex};

use serde_json::{json, Map, Value};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[path = "zdo_response.rs"]
pub mod zdo_response;

use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::manager::StartResult;
use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame as UnpiFrame;
use zigbee_apiarist::adapter::z_stack::znp::definition;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::{ParameterDef, ZpiObject};
use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

/// Result of a mocked ZNP request.
pub struct Reply {
    /// `false` mirrors an upstream handler that throws: no SRSP is sent and
    /// the adapter request times out.
    pub respond: bool,
    pub payload: Value,
    /// Raw SRSP payload bytes, bypassing parameter encoding.
    pub raw: Option<Vec<u8>>,
    /// Extra AREQ frames emitted right after the SRSP.
    pub extra: Vec<(u8, u8, Vec<u8>)>,
}

impl Reply {
    pub fn payload(payload: Value) -> Self {
        Self {
            respond: true,
            payload,
            raw: None,
            extra: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self::payload(json!({}))
    }

    /// Upstream handlers may return a payload object with missing fields;
    /// the wire encoder cannot express that, so send raw bytes instead.
    pub fn raw(data: Vec<u8>) -> Self {
        Self {
            respond: true,
            payload: Value::Null,
            raw: Some(data),
            extra: Vec::new(),
        }
    }

    /// Upstream mock handlers signal errors by throwing; the harness stops
    /// answering so the adapter request expires.
    pub fn timeout() -> Self {
        Self {
            respond: false,
            payload: Value::Null,
            raw: None,
            extra: Vec::new(),
        }
    }

    pub fn with_extra(mut self, subsystem: Subsystem, command_id: u8, data: Vec<u8>) -> Self {
        self.extra.push((subsystem as u8, command_id, data));
        self
    }

    /// Attach a ZDO response AREQ (`mockZdoZpiObject`). `cluster_id` is the
    /// ZDO response cluster; the UNPI command id is resolved from the ZNP
    /// definition table.
    pub fn with_zdo(mut self, cluster_id: u16, srcaddr: Option<u16>, args: &[Value]) -> Self {
        let command = definition::for_subsystem(Subsystem::ZDO)
            .iter()
            .find(|command| {
                command.r#type == Type::AREQ && command.zdo_cluster_id == Some(cluster_id)
            })
            .unwrap_or_else(|| panic!("no ZDO response command for cluster {cluster_id}"));
        let mut data = Vec::new();
        if cluster_id != ClusterId::NETWORK_ADDRESS_RESPONSE
            && cluster_id != ClusterId::IEEE_ADDRESS_RESPONSE
        {
            if let Some(srcaddr) = srcaddr {
                data.extend_from_slice(&srcaddr.to_le_bytes());
            }
        }
        data.extend_from_slice(&zdo_response::build_zdo_response(cluster_id, args));
        self.extra.push((Subsystem::ZDO as u8, command.id, data));
        self
    }
}

pub type Handler = Arc<dyn Fn(&Value, &mut MockState) -> Reply + Send + Sync>;

/// Mutable state shared by all mock handlers (`ZnpRequestMockBuilder` fields).
pub struct MockState {
    pub responders: Vec<(u8, String, Handler)>,
    pub nv: BTreeMap<u16, Option<Vec<u8>>>,
    pub nv_ext: BTreeMap<(u8, u16, u16), Option<Vec<u8>>>,
    pub data_request_code: u8,
    pub data_request_ext_code: u8,
    pub data_confirm_code: u16,
    pub data_confirm_reset: bool,
    pub assoc_get_with_address_node_relation: u8,
    pub bind_status: u8,
    pub simple_descriptor_endpoint: u8,
    /// Raw payload bytes of the request currently being handled. ZDO SREQ
    /// commands carry their parameters as raw ZDO bytes, not as `MtParameter`s.
    pub current_data: Vec<u8>,
}

impl Default for MockState {
    fn default() -> Self {
        Self {
            responders: Vec::new(),
            nv: BTreeMap::new(),
            nv_ext: BTreeMap::new(),
            data_request_code: 0,
            data_request_ext_code: 0,
            data_confirm_code: 0,
            data_confirm_reset: false,
            assoc_get_with_address_node_relation: 1,
            bind_status: 0,
            simple_descriptor_endpoint: 0,
            current_data: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub subsystem: Subsystem,
    pub command: String,
    pub payload: Value,
    pub frame: Vec<u8>,
}

impl Request {
    pub fn subsystem(&self) -> Subsystem {
        self.subsystem
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

fn argument(payload: &Value, name: &str) -> Value {
    payload.get(name).cloned().unwrap_or(Value::Null)
}

pub fn bytes(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_u64().unwrap_or(0) as u8)
                .collect()
        })
        .unwrap_or_default()
}

fn u16_arg(payload: &Value, name: &str) -> u16 {
    payload.get(name).and_then(Value::as_u64).unwrap_or(0) as u16
}

fn u8_arg(payload: &Value, name: &str) -> u8 {
    payload.get(name).and_then(Value::as_u64).unwrap_or(0) as u8
}

/// Encode an SRSP payload using the command's declared response parameters.
fn encode_srsp(subsystem: Subsystem, command_name: &str, payload: &Value) -> UnpiFrame {
    let command = definition::find_by_name(subsystem, command_name)
        .unwrap_or_else(|| panic!("unknown command {command_name}"));
    if zigbee_apiarist::adapter::z_stack::znp::utils::is_mt_cmd_sreq_zdo(command) {
        return UnpiFrame::new(
            Type::SRSP,
            subsystem,
            command.id,
            vec![u8_arg(payload, "status")],
            0,
            0,
        );
    }
    let parameters = command
        .response
        .iter()
        .map(|parameter| ParameterDef {
            name: parameter.name.to_string(),
            parameter_type: parameter.parameter_type,
        })
        .collect::<Vec<_>>();
    let mut object = ZpiObject::new(
        Type::SRSP,
        subsystem,
        command_name,
        0,
        payload.clone(),
        parameters,
    );
    object.command.id = u16::from(command.id);
    object.to_unpi_frame()
}

/// Encode an AREQ payload using the command's declared request parameters.
pub fn encode_areq(subsystem: Subsystem, command_name: &str, payload: &Value) -> Vec<u8> {
    let command = definition::find_by_name(subsystem, command_name)
        .unwrap_or_else(|| panic!("unknown command {command_name}"));
    let parameters = command
        .request
        .iter()
        .map(|parameter| ParameterDef {
            name: parameter.name.to_string(),
            parameter_type: parameter.parameter_type,
        })
        .collect::<Vec<_>>();
    let mut object = ZpiObject::new(
        Type::AREQ,
        subsystem,
        command_name,
        0,
        payload.clone(),
        parameters,
    );
    object.command.id = u16::from(command.id);
    object.to_unpi_frame().to_buffer()
}

pub struct MockCoordinator {
    pub state: Arc<StdMutex<MockState>>,
    requests: Arc<StdMutex<Vec<Request>>>,
    commands: mpsc::UnboundedSender<Vec<u8>>,
    server: JoinHandle<()>,
    transport: Option<AdapterTransport>,
}

impl Drop for MockCoordinator {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl MockCoordinator {
    pub fn spawn() -> Self {
        let (reader, peer) = serial2_tokio::SerialPort::pair().expect("serial pair");
        let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
        transport.attach_serial(reader).expect("attach serial");

        let state = Arc::new(StdMutex::new(MockState::default()));
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let (commands, mut command_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let mut writer = peer.try_clone().expect("clone peer");
        let mut reader = peer;
        let server_state = state.clone();
        let server_requests = requests.clone();
        let server = tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            let mut pending: Vec<u8> = Vec::new();
            loop {
                tokio::select! {
                    read = reader.read(&mut buffer) => {
                        match read {
                            Ok(0) | Err(_) => return,
                            Ok(read) => pending.extend_from_slice(&buffer[..read]),
                        }
                    }
                    command = command_rx.recv() => {
                        let Some(bytes) = command else { return };
                        if writer.write_all(&bytes).await.is_err() {
                            return;
                        }
                        continue;
                    }
                }

                if !pending.is_empty() && pending[0] != 0xfe {
                    if let Some(start) = pending.iter().position(|byte| *byte == 0xfe) {
                        pending.drain(..start);
                    } else {
                        pending.clear();
                    }
                }

                loop {
                    if pending.len() < 5 {
                        break;
                    }
                    let length = pending[1] as usize;
                    let frame_length = 4 + length + 1;
                    if pending.len() < frame_length {
                        break;
                    }
                    let raw = pending[..frame_length].to_vec();
                    pending.drain(..frame_length);

                    let frame = match UnpiFrame::from_buffer(length as u8, 4 + length, &raw) {
                        Some(frame) => frame,
                        None => continue,
                    };
                    let object = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        ZpiObject::from_unpi_frame(&frame)
                    }))
                    .unwrap_or_else(|_| {
                        let command = definition::find_by_id(frame.subsystem, frame.commandID)
                            .unwrap_or_else(|| panic!("unknown command id {}", frame.commandID));
                        ZpiObject {
                            r#type: frame.r#type,
                            subsystem: frame.subsystem,
                            command:
                                zigbee_apiarist::adapter::z_stack::znp::zpi_object::CommandDef {
                                    name: command.name.to_string(),
                                    id: u16::from(command.id),
                                },
                            transid: 0,
                            payload: Value::Object(Map::new()),
                            parameters: Vec::new(),
                        }
                    });
                    server_requests.lock().unwrap().push(Request {
                        subsystem: object.subsystem,
                        command: object.command.name.clone(),
                        payload: object.payload.clone(),
                        frame: raw.clone(),
                    });

                    let handler = {
                        let state = server_state.lock().unwrap();
                        state
                            .responders
                            .iter()
                            .rev()
                            .find(|(subsystem, command, _)| {
                                *subsystem == object.subsystem as u8
                                    && command == &object.command.name
                            })
                            .map(|(_, _, handler)| handler.clone())
                    };

                    let reply = match handler {
                        Some(handler) => {
                            let mut state = server_state.lock().unwrap();
                            state.current_data = frame.data.clone();
                            handler(&object.payload, &mut state)
                        }
                        None => Reply::timeout(),
                    };

                    if reply.respond {
                        let mut response = match reply.raw {
                            Some(raw) => UnpiFrame::new(
                                Type::SRSP,
                                object.subsystem,
                                frame.commandID,
                                raw,
                                0,
                                0,
                            ),
                            None => {
                                encode_srsp(object.subsystem, &object.command.name, &reply.payload)
                            }
                        }
                        .to_buffer();
                        // SRSP echoes the request command byte (including any
                        // transaction bits in the low nibble).
                        response[3] = frame.commandID;
                        if writer.write_all(&response).await.is_err() {
                            return;
                        }
                    }
                    for (subsystem, command_id, data) in reply.extra {
                        let Some(subsystem) = Subsystem::from_u8(subsystem) else {
                            continue;
                        };
                        let areq = UnpiFrame::new(Type::AREQ, subsystem, command_id, data, 0, 0);
                        if writer.write_all(&areq.to_buffer()).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        let mock = Self {
            state,
            requests,
            commands,
            server,
            transport: Some(transport),
        };
        mock.install_defaults();
        mock
    }

    fn install_defaults(&self) {
        let state = self.state.clone();
        let mut state = state.lock().unwrap();

        // Upstream `ZnpRequestMockBuilder` NV responders.
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvRead".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let item = state.nv.get(&u16_arg(payload, "id")).cloned().flatten();
                match item {
                    Some(value) => {
                        let mut result = Map::new();
                        result.insert("status".to_string(), json!(0));
                        result.insert("len".to_string(), json!(value.len()));
                        result.insert(
                            "value".to_string(),
                            Value::Array(value.into_iter().map(|b| json!(b)).collect()),
                        );
                        Reply::payload(Value::Object(result))
                    }
                    None => Reply::payload(json!({"status": 1})),
                }
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvReadExt".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let item = state.nv.get(&u16_arg(payload, "id")).cloned().flatten();
                match item {
                    Some(value) => {
                        let mut result = Map::new();
                        result.insert("status".to_string(), json!(0));
                        result.insert("len".to_string(), json!(value.len()));
                        result.insert(
                            "value".to_string(),
                            Value::Array(value.into_iter().map(|b| json!(b)).collect()),
                        );
                        Reply::payload(Value::Object(result))
                    }
                    None => Reply::payload(json!({"status": 1})),
                }
            }),
        ));
        let osal_write = Arc::new(|payload: &Value, state: &mut MockState| {
            let id = u16_arg(payload, "id");
            if state.nv.contains_key(&id) {
                state
                    .nv
                    .insert(id, Some(bytes(&argument(payload, "value"))));
                Reply::payload(json!({"status": 0}))
            } else {
                Reply::payload(json!({"status": 1}))
            }
        });
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvWrite".to_string(),
            osal_write.clone(),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvWriteExt".to_string(),
            osal_write,
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvItemInit".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let id = u16_arg(payload, "id");
                let len = u8_arg(payload, "len") as usize;
                if let Some(item) = state.nv.get(&id) {
                    if let Some(value) = item {
                        if value.len() != len {
                            return Reply::payload(json!({"status": 0x0a}));
                        }
                    }
                    return Reply::payload(json!({"status": 0x00}));
                }
                state
                    .nv
                    .insert(id, Some(bytes(&argument(payload, "initvalue"))));
                Reply::payload(json!({"status": 0x09}))
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvLength".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let length = state
                    .nv
                    .get(&u16_arg(payload, "id"))
                    .and_then(|item| item.as_ref())
                    .map(Vec::len)
                    .unwrap_or(0);
                Reply::payload(json!({"length": length}))
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "osalNvDelete".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let id = u16_arg(payload, "id");
                let len = u16_arg(payload, "len") as usize;
                match state.nv.get(&id) {
                    Some(Some(value)) if value.len() != len => {
                        Reply::payload(json!({"status": 0x0a}))
                    }
                    Some(_) => {
                        state.nv.remove(&id);
                        Reply::payload(json!({"status": 0x00}))
                    }
                    None => Reply::payload(json!({"status": 0x09})),
                }
            }),
        ));

        let nv_read = Arc::new(|payload: &Value, state: &mut MockState| {
            let key = (
                u8_arg(payload, "sysid"),
                u16_arg(payload, "itemid"),
                u16_arg(payload, "subid"),
            );
            match state.nv_ext.get(&key).cloned().flatten() {
                Some(value) => {
                    let mut result = Map::new();
                    result.insert("status".to_string(), json!(0));
                    result.insert("len".to_string(), json!(value.len()));
                    result.insert(
                        "value".to_string(),
                        Value::Array(value.into_iter().map(|b| json!(b)).collect()),
                    );
                    Reply::payload(Value::Object(result))
                }
                None => Reply::payload(json!({"status": 1})),
            }
        });
        state
            .responders
            .push((Subsystem::SYS as u8, "nvRead".to_string(), nv_read));
        state.responders.push((
            Subsystem::SYS as u8,
            "nvWrite".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let key = (
                    u8_arg(payload, "sysid"),
                    u16_arg(payload, "itemid"),
                    u16_arg(payload, "subid"),
                );
                if state.nv_ext.contains_key(&key) {
                    state
                        .nv_ext
                        .insert(key, Some(bytes(&argument(payload, "value"))));
                    Reply::payload(json!({"status": 0}))
                } else {
                    Reply::payload(json!({"status": 1}))
                }
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "nvCreate".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let key = (
                    u8_arg(payload, "sysid"),
                    u16_arg(payload, "itemid"),
                    u16_arg(payload, "subid"),
                );
                let len = payload.get("len").and_then(Value::as_u64).unwrap_or(0) as usize;
                if let Some(item) = state.nv_ext.get(&key) {
                    if let Some(value) = item {
                        if value.len() != len {
                            return Reply::payload(json!({"status": 0x0a}));
                        }
                    }
                    return Reply::payload(json!({"status": 0x00}));
                }
                state.nv_ext.insert(key, None);
                Reply::payload(json!({"status": 0x09}))
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "nvLength".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let key = (
                    u8_arg(payload, "sysid"),
                    u16_arg(payload, "itemid"),
                    u16_arg(payload, "subid"),
                );
                let length = state
                    .nv_ext
                    .get(&key)
                    .and_then(|item| item.as_ref())
                    .map(Vec::len)
                    .unwrap_or(0);
                Reply::payload(json!({"len": length}))
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "nvDelete".to_string(),
            Arc::new(|payload: &Value, state: &mut MockState| {
                let key = (
                    u8_arg(payload, "sysid"),
                    u16_arg(payload, "itemid"),
                    u16_arg(payload, "subid"),
                );
                state.nv_ext.remove(&key);
                Reply::payload(json!({"status": 0}))
            }),
        ));

        // Upstream `baseZnpRequestMock` command responders.
        state.responders.push((
            Subsystem::SYS as u8,
            "version".to_string(),
            Arc::new(|payload: &Value, _state: &mut MockState| {
                if payload.as_object().map(|object| object.is_empty()) == Some(true) {
                    Reply::payload(json!({"product": 2, "revision": 20201026}))
                } else {
                    Reply::timeout()
                }
            }),
        ));
        for (subsystem, command) in [
            (Subsystem::SYS, "ping"),
            (Subsystem::SYS, "stackTune"),
            (Subsystem::ZDO, "extFindGroup"),
            (Subsystem::ZDO, "extAddGroup"),
            (Subsystem::ZDO, "activeEpReq"),
            (Subsystem::ZDO, "simpleDescReq"),
            (Subsystem::ZDO, "mgmtPermitJoinReq"),
            (Subsystem::ZDO, "nodeDescReq"),
            (Subsystem::ZDO, "bindReq"),
            (Subsystem::ZDO, "unbindReq"),
            (Subsystem::ZDO, "mgmtLeaveReq"),
            (Subsystem::ZDO, "mgmtLqiReq"),
            (Subsystem::ZDO, "mgmtRtgReq"),
            (Subsystem::ZDO, "mgmtNwkUpdateReq"),
            (Subsystem::ZDO, "extRouteDisc"),
            (Subsystem::ZDO, "nwkAddrReq"),
            (Subsystem::ZDO, "extNwkInfo"),
            (Subsystem::UTIL, "assocRemove"),
            (Subsystem::UTIL, "assocAdd"),
            (Subsystem::UTIL, "ledControl"),
            (Subsystem::APP_CNF, "bdbAddInstallCode"),
            (Subsystem::APP_CNF, "bdbSetChannel"),
            (Subsystem::APP_CNF, "bdbStartCommissioning"),
            (Subsystem::AF, "register"),
            (Subsystem::AF, "interPanCtl"),
        ] {
            state.responders.push((
                subsystem as u8,
                command.to_string(),
                Arc::new(|_payload: &Value, _state: &mut MockState| Reply::empty()),
            ));
        }

        fn first_u16(state: &MockState) -> u16 {
            state
                .current_data
                .get(0..2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                .unwrap_or(0)
        }

        state.responders.push((
            Subsystem::ZDO as u8,
            "activeEpReq".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                let target = first_u16(state);
                Reply::empty().with_zdo(
                    ClusterId::ACTIVE_ENDPOINTS_RESPONSE,
                    Some(target),
                    &[
                        json!(0),
                        json!({
                            "nwkAddress": target,
                            "endpointList": [1, 2, 3, 4, 5, 6, 8, 10, 11, 110, 12, 13, 47, 242],
                        }),
                    ],
                )
            }),
        ));
        state.responders.push((
            Subsystem::ZDO as u8,
            "simpleDescReq".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                let target = first_u16(state);
                let endpoint = state.current_data.get(2).copied().unwrap_or(0);
                let (profile_id, device_id, in_clusters, out_clusters) = if endpoint == 1 {
                    (123, 5, vec![1], vec![2])
                } else if endpoint == 99 {
                    (123, 5, vec![1], vec![2])
                } else {
                    (124, 7, vec![8], vec![9])
                };
                Reply::empty().with_zdo(
                    ClusterId::SIMPLE_DESCRIPTOR_RESPONSE,
                    Some(target),
                    &[
                        json!(0),
                        json!({
                            "length": 1,
                            "endpoint": endpoint,
                            "profileId": profile_id,
                            "deviceId": device_id,
                            "inClusterList": in_clusters,
                            "outClusterList": out_clusters,
                            "nwkAddress": target,
                            "deviceVersion": 0,
                        }),
                    ],
                )
            }),
        ));
        state.responders.push((
            Subsystem::ZDO as u8,
            "mgmtPermitJoinReq".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                let target = state
                    .current_data
                    .get(1..3)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .unwrap_or(0);
                Reply::empty().with_zdo(
                    ClusterId::PERMIT_JOINING_RESPONSE,
                    Some(target),
                    &[json!(0), Value::Null],
                )
            }),
        ));
        for command in ["bindReq", "unbindReq"] {
            state.responders.push((
                Subsystem::ZDO as u8,
                command.to_string(),
                Arc::new(|_payload: &Value, state: &mut MockState| {
                    let target = first_u16(state);
                    Reply::empty().with_zdo(
                        ClusterId::BIND_RESPONSE,
                        Some(target),
                        &[json!(state.bind_status), Value::Null],
                    )
                }),
            ));
        }
        state.responders.push((
            Subsystem::ZDO as u8,
            "mgmtLeaveReq".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                let target = first_u16(state);
                Reply::empty().with_zdo(
                    ClusterId::LEAVE_RESPONSE,
                    Some(target),
                    &[json!(0), Value::Null],
                )
            }),
        ));
        state.responders.push((
            Subsystem::ZDO as u8,
            "nwkAddrReq".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                let bytes = state.current_data.get(0..8).unwrap_or(&[]).to_vec();
                let value = u64::from_le_bytes(bytes.try_into().unwrap_or([0u8; 8]));
                let eui64 = format!("0x{value:016x}");
                let nwk = value as u16;
                Reply::empty().with_zdo(
                    ClusterId::NETWORK_ADDRESS_RESPONSE,
                    None,
                    &[
                        json!(0),
                        json!({
                            "eui64": eui64,
                            "nwkAddress": nwk,
                            "assocDevList": [],
                            "startIndex": 0,
                        }),
                    ],
                )
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "resetReq".to_string(),
            Arc::new(|_payload: &Value, _state: &mut MockState| {
                Reply::empty().with_extra(Subsystem::SYS, 128, vec![0, 0, 2, 7, 1, 0, 0])
            }),
        ));
        state.responders.push((
            Subsystem::ZDO as u8,
            "startupFromApp".to_string(),
            Arc::new(|_payload: &Value, _state: &mut MockState| {
                Reply::empty().with_extra(Subsystem::ZDO, 192, vec![9])
            }),
        ));
        state.responders.push((
            Subsystem::UTIL as u8,
            "getDeviceInfo".to_string(),
            Arc::new(|_payload: &Value, _state: &mut MockState| {
                Reply::payload(json!({"devicestate": 0, "ieeeaddr": "0x00124b0009d80ba7"}))
            }),
        ));
        state.responders.push((
            Subsystem::SYS as u8,
            "getExtAddr".to_string(),
            Arc::new(|_payload: &Value, _state: &mut MockState| {
                Reply::payload(json!({"extaddress": "0x00124b0009d69f77"}))
            }),
        ));
        state.responders.push((
            Subsystem::UTIL as u8,
            "assocGetWithAddress".to_string(),
            Arc::new(|_payload: &Value, state: &mut MockState| {
                Reply::payload(json!({"noderelation": state.assoc_get_with_address_node_relation}))
            }),
        ));
        for command in ["dataRequest", "dataRequestExt"] {
            state.responders.push((
                Subsystem::AF as u8,
                command.to_string(),
                Arc::new(move |payload: &Value, state: &mut MockState| {
                    let code = if command == "dataRequest" {
                        state.data_request_code
                    } else {
                        state.data_request_ext_code
                    };
                    if code != 0 {
                        return Reply::payload(json!({"status": code}));
                    }
                    let status = state.data_confirm_code;
                    if state.data_confirm_reset {
                        state.data_confirm_code = 0;
                    }
                    if status == 9999 {
                        return Reply::empty();
                    }
                    let transid = payload.get("transid").and_then(Value::as_u64).unwrap_or(0) as u8;
                    Reply::empty().with_extra(Subsystem::AF, 128, vec![status as u8, 1, transid])
                }),
            ));
        }
    }

    pub fn handle<F>(&self, subsystem: Subsystem, command: &str, handler: F)
    where
        F: Fn(&Value, &mut MockState) -> Reply + Send + Sync + 'static,
    {
        let mut state = self.state.lock().unwrap();
        state
            .responders
            .retain(|(sub, cmd, _)| !(*sub == subsystem as u8 && cmd == command));
        state
            .responders
            .push((subsystem as u8, command.to_string(), Arc::new(handler)));
    }

    pub fn nv(&self, id: u16, value: Option<Vec<u8>>) {
        self.state.lock().unwrap().nv.insert(id, value);
    }

    pub fn remove_nv(&self, id: u16) {
        self.state.lock().unwrap().nv.remove(&id);
    }

    pub fn nv_ext(&self, sys_id: u8, id: u16, sub_id: u16, value: Option<Vec<u8>>) {
        self.state
            .lock()
            .unwrap()
            .nv_ext
            .insert((sys_id, id, sub_id), value);
    }

    pub fn set_data_request_code(&self, code: u8) {
        self.state.lock().unwrap().data_request_code = code;
    }

    pub fn set_data_request_ext_code(&self, code: u8) {
        self.state.lock().unwrap().data_request_ext_code = code;
    }

    pub fn set_data_confirm_code(&self, code: u16, reset: bool) {
        let mut state = self.state.lock().unwrap();
        state.data_confirm_code = code;
        state.data_confirm_reset = reset;
    }

    pub fn set_assoc_node_relation(&self, relation: u8) {
        self.state
            .lock()
            .unwrap()
            .assoc_get_with_address_node_relation = relation;
    }

    pub fn set_bind_status(&self, status: u8) {
        self.state.lock().unwrap().bind_status = status;
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    pub fn clear_requests(&self) {
        self.requests.lock().unwrap().clear();
    }

    pub fn requests_named(&self, command: &str) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|request| request.command == command)
            .collect()
    }

    pub fn count(&self, command: &str) -> usize {
        self.requests()
            .into_iter()
            .filter(|request| request.command == command)
            .count()
    }

    /// Inject a raw frame towards the adapter (upstream `znpReceived(...)`).
    pub fn inject(&self, frame: Vec<u8>) {
        let _ = self.commands.send(frame);
    }

    pub fn inject_areq(&self, subsystem: Subsystem, command_name: &str, payload: Value) {
        self.inject(encode_areq(subsystem, command_name, &payload));
    }

    /// Inject an event with an explicit UNPI type.
    pub fn inject_typed(
        &self,
        r#type: Type,
        subsystem: Subsystem,
        command_name: &str,
        payload: Value,
    ) {
        let mut bytes = encode_areq(subsystem, command_name, &payload);
        bytes[2] = ((r#type as u8) << 5) | (subsystem as u8);
        self.inject(bytes);
    }

    /// Inject a ZDO AREQ response (`mockZdoZpiObject`).
    pub fn inject_zdo(&self, cluster_id: u16, srcaddr: Option<u16>, args: &[Value]) {
        let command = definition::for_subsystem(Subsystem::ZDO)
            .iter()
            .find(|command| {
                command.r#type == Type::AREQ && command.zdo_cluster_id == Some(cluster_id)
            })
            .unwrap_or_else(|| panic!("no ZDO response command for cluster {cluster_id}"));
        let mut data = Vec::new();
        if cluster_id != ClusterId::NETWORK_ADDRESS_RESPONSE
            && cluster_id != ClusterId::IEEE_ADDRESS_RESPONSE
        {
            if let Some(srcaddr) = srcaddr {
                data.extend_from_slice(&srcaddr.to_le_bytes());
            }
        }
        data.extend_from_slice(&zdo_response::build_zdo_response(cluster_id, args));
        let frame = UnpiFrame::new(Type::AREQ, Subsystem::ZDO, command.id, data, 0, 0);
        self.inject(frame.to_buffer());
    }

    /// Write raw bytes to the coordinator side of the serial link.
    pub async fn write_raw(&self, bytes: &[u8]) {
        let _ = self.commands.send(bytes.to_vec());
    }

    pub fn take_transport(&mut self) -> AdapterTransport {
        self.transport.take().expect("transport already taken")
    }
}

pub struct Harness {
    pub mock: MockCoordinator,
    pub adapter: ZStackAdapter,
}

impl Harness {
    pub fn new(
        network_options: ConfigNetworkOptions,
        backup_path: &str,
        adapter_options: AdapterOptions,
    ) -> Self {
        // Install the recording logger before the adapter can log anything.
        let _ = recorded_logs();
        let mut mock = MockCoordinator::spawn();
        let transport = mock.take_transport();
        let adapter = ZStackAdapter::new(
            transport,
            network_options,
            backup_path.to_string(),
            adapter_options,
        );
        Self { mock, adapter }
    }

    pub async fn start(&mut self) -> Result<StartResult, String> {
        self.adapter.start().await
    }
}

/// Captures logger output for tests that assert user-facing diagnostics.
pub use zigbee_apiarist::utils::logger::Logger;

pub struct RecordingLogger {
    messages: Arc<StdMutex<Vec<String>>>,
}

impl Logger for RecordingLogger {
    fn debug(&self, message: &str, _namespace: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }
    fn info(&self, message: &str, _namespace: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }
    fn warning(&self, message: &str, _namespace: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }
    fn error(&self, message: &str, _namespace: &str) {
        self.messages.lock().unwrap().push(message.to_string());
    }
}

static RECORDED: std::sync::OnceLock<Arc<StdMutex<Vec<String>>>> = std::sync::OnceLock::new();

pub fn recorded_logs() -> Arc<StdMutex<Vec<String>>> {
    RECORDED
        .get_or_init(|| {
            let messages = Arc::new(StdMutex::new(Vec::new()));
            zigbee_apiarist::utils::logger::set_logger(Arc::new(RecordingLogger {
                messages: messages.clone(),
            }));
            messages
        })
        .clone()
}

pub fn clear_logs() {
    recorded_logs().lock().unwrap().clear();
}

pub fn log_messages() -> Vec<String> {
    recorded_logs().lock().unwrap().clone()
}
