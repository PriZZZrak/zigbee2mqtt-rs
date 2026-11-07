// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/zpiObject.ts
// TODO: port ZpiObject.

use serde_json::Value;

use super::super::unpi::constants::{Subsystem, Type};
use super::super::unpi::frame::Frame;

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

    /// Upstream: `ZpiObject.fromUnpiFrame(frame)`.
    pub fn from_unpi_frame(frame: &Frame) -> Self {
        todo!()
    }

    /// Upstream: `obj.createPayloadBuffer()`.
    pub fn create_payload_buffer(&self) -> Vec<u8> {
        todo!()
    }

    /// Upstream: `obj.isResetCommand()`.
    pub fn is_reset_command(&self) -> bool {
        todo!()
    }

    /// Upstream: `obj.toString()`.
    pub fn to_string(&self) -> String {
        todo!()
    }
}
