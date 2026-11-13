// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist (shared modules)
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// R1 converter boundary (AGENTS §5 Q4): the shared host is ported, the
// converter database and vendor quirks are not. An empty registry yields the
// upstream "not found" fallback (definition-less devices).

pub mod definition;
pub mod exposes;
pub mod registry;

pub use definition::{Definition, DefinitionRef};
pub use exposes::{access, Expose, Exposes, Fingerprint, WhiteLabel};
pub use registry::{normalize_model_id, DefinitionRegistry};
