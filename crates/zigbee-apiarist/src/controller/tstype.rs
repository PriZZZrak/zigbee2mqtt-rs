// Original: .upstream/zigbee-herdsman/src/controller/tstype.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Runtime-relevant types from `tstype.ts`. The remaining upstream entries are
// TypeScript-only helper aliases; they are represented by the concrete Rust
// types (`Value`, `Vec<Value>`, `HashMap`) at their use sites.

use serde_json::Value;

/// Upstream `KeyValue` — generic JSON-like object.
pub type KeyValue = Value;

/// Upstream `SendPolicy` — tstype.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendPolicy {
    Bulk,
    Queue,
    Immediate,
    KeepPayload,
    KeepCommand,
    KeepCmdUndiv,
}

impl SendPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bulk => "bulk",
            Self::Queue => "queue",
            Self::Immediate => "immediate",
            Self::KeepPayload => "keep-payload",
            Self::KeepCommand => "keep-command",
            Self::KeepCmdUndiv => "keep-cmd-undiv",
        }
    }
}
