// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist (shared modules)
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// Rust replacement for the part of `zigbee-herdsman-converters` that
// zigbee2mqtt needs to run. AGENTS §5 Q4: the converter database, vendor
// quirks and converter-authoring tooling are not ported. An empty
// registry yields the upstream "not found" fallback (definition-less devices).
//
// The crate depends only on `zigbee-apiarist` (herdsman types); it must not
// depend on the bridge crate.

mod definition;
mod exposes;
mod registry;

pub use definition::{
    to_zigbee, ConvertData, ConvertMeta, ConverterFuture, Definition, DefinitionRef,
    ConfigureFn, FromZigbeeConverter, OnEventBaseData, OnEventEvent, OnEventFn, PublishCallback, ToMeta, ToTarget,
    ToZigbeeConverter, ToZigbeeFn, ToZigbeeResult,
};
pub use exposes::{access, Expose, Exposes, Fingerprint, WhiteLabel};
pub use registry::{normalize_model_id, DefinitionRegistry};
