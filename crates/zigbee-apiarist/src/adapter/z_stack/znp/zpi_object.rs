// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/zpiObject.ts
// TODO: port remaining ZDO response catalog entries.

use serde_json::{json, Map, Value};

use super::super::unpi::constants::{Subsystem, Type};
use super::super::unpi::frame::Frame;
use super::buffalo_znp::BuffaloZnp;
use super::definition::{self, MtCmd};
use super::parameter_type::ParameterType;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDef {
    pub name: String,
    pub id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDef {
    pub name: String,
    pub parameter_type: u32,
}

#[derive(Debug, Clone)]
pub struct ZpiObject {
    pub r#type: Type,
    pub subsystem: Subsystem,
    pub command: CommandDef,
    pub transid: u8,
    pub payload: Value,
    pub parameters: Vec<ParameterDef>,
}

fn is_buffer_and_list_type(parameter_type: u32) -> bool {
    matches!(
        parameter_type,
        value if value == ParameterType::BUFFER as u32
            || value == ParameterType::BUFFER8 as u32
            || value == ParameterType::BUFFER16 as u32
            || value == ParameterType::BUFFER18 as u32
            || value == ParameterType::BUFFER32 as u32
            || value == ParameterType::BUFFER42 as u32
            || value == ParameterType::BUFFER100 as u32
            || value == ParameterType::LIST_UINT16 as u32
            || value == ParameterType::LIST_NETWORK as u32
            || value == ParameterType::LIST_UINT8 as u32
    )
}

impl ZpiObject {
    pub fn new(
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        transid: u8,
        payload: Value,
        parameters: Vec<ParameterDef>,
    ) -> Self {
        Self {
            r#type,
            subsystem,
            command: CommandDef {
                name: command.to_string(),
                id: 0,
            },
            transid,
            payload,
            parameters,
        }
    }

    fn from_command(
        r#type: Type,
        subsystem: Subsystem,
        command: &'static MtCmd,
        payload: Value,
    ) -> Self {
        Self {
            r#type,
            subsystem,
            command: CommandDef {
                name: command.name.to_string(),
                id: u16::from(command.id),
            },
            transid: 0,
            payload,
            parameters: Vec::new(),
        }
    }

    /// Upstream: `ZpiObject.fromUnpiFrame(frame)`.
    pub fn from_unpi_frame(frame: &Frame) -> Self {
        let command =
            definition::find_by_id(frame.subsystem, frame.commandID).unwrap_or_else(|| {
                panic!(
                    "CommandID '{}' from subsystem '{}' not found",
                    frame.commandID,
                    frame.subsystem.name()
                )
            });

        let mut payload = Map::new();

        if definition::is_mt_cmd_areq_zdo(command) {
            let cluster_id = command.zdo_cluster_id.expect("zdo cluster id");
            if cluster_id == ClusterId::NETWORK_ADDRESS_RESPONSE
                || cluster_id == ClusterId::IEEE_ADDRESS_RESPONSE
            {
                // Z-Stack swaps `startindex` and `numassocdev` compared to ZDO;
                // swap them back before handing the payload to ZDO.
                let mut data = frame.data.clone();
                if data.len() > 12 {
                    data.swap(11, 12);
                }
                payload.insert(
                    "zdo".to_string(),
                    BuffaloZdo::read_response(false, cluster_id, &data),
                );
            } else {
                if frame.data.len() < 2 {
                    panic!("Not enough bytes to read srcaddr");
                }
                payload.insert(
                    "srcaddr".to_string(),
                    json!(u16::from_le_bytes([frame.data[0], frame.data[1]])),
                );
                payload.insert(
                    "zdo".to_string(),
                    BuffaloZdo::read_response(false, cluster_id, &frame.data[2..]),
                );
            }
        } else if definition::is_mt_cmd_sreq_zdo(command) {
            payload.insert(
                "status".to_string(),
                json!(*frame.data.first().unwrap_or(&0)),
            );
        } else {
            let parameters = if frame.r#type == Type::SRSP && command.r#type != Type::AREQ {
                command.response
            } else {
                command.request
            };
            assert!(
                !parameters.is_empty(),
                "CommandID '{}' from subsystem '{}' cannot be a {}",
                frame.commandID,
                frame.subsystem.name(),
                if frame.r#type == Type::SRSP {
                    "response"
                } else {
                    "request"
                }
            );
            payload = Self::read_parameters(&frame.data, parameters);
        }

        Self::from_command(
            frame.r#type,
            frame.subsystem,
            command,
            Value::Object(payload),
        )
    }

    fn read_parameters(
        buffer: &[u8],
        parameters: &[definition::MtParameter],
    ) -> Map<String, Value> {
        let mut buffalo = BuffaloZnp::new(buffer.to_vec(), 0);
        let mut result = Map::new();

        for (index, parameter) in parameters.iter().enumerate() {
            let mut options = Map::new();

            if is_buffer_and_list_type(parameter.parameter_type) {
                if index > 0 {
                    let length_parameter = &parameters[index - 1];
                    if let Some(length) = result.get(length_parameter.name).and_then(Value::as_u64)
                    {
                        options.insert("length".to_string(), json!(length));
                    }
                }
            }

            let value = buffalo.read(parameter.parameter_type, &Value::Object(options));
            result.insert(parameter.name.to_string(), value);
        }

        result
    }

    /// Upstream: `obj.createPayloadBuffer()`.
    pub fn create_payload_buffer(&self) -> Vec<u8> {
        let mut buffalo = BuffaloZnp::new(vec![0; 250], 0);

        for (index, parameter) in self.parameters.iter().enumerate() {
            let value = self
                .payload
                .get(&parameter.name)
                .cloned()
                .unwrap_or(Value::Null);
            let mut options = Map::new();
            if is_buffer_and_list_type(parameter.parameter_type) && index > 0 {
                let length_parameter = &self.parameters[index - 1];
                if let Some(length) = self
                    .payload
                    .get(&length_parameter.name)
                    .and_then(Value::as_u64)
                {
                    options.insert("length".to_string(), json!(length));
                }
            }
            buffalo.write(parameter.parameter_type, &value, &Value::Object(options));
        }

        buffalo.get_written()
    }

    /// Upstream: `obj.isResetCommand()`.
    pub fn is_reset_command(&self) -> bool {
        (self.command.name == "resetReq" && self.subsystem == Subsystem::SYS)
            || (self.command.name == "systemReset" && self.subsystem == Subsystem::SAPI)
    }

    /// Upstream: `obj.toString()`.
    pub fn to_string(&self) -> String {
        format!(
            "{}: {} - {} - {}",
            self.r#type.name(),
            self.subsystem.name(),
            self.command.name,
            self.payload
        )
    }
}
