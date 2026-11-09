// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/index.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

pub mod entries;
pub mod r#struct;
pub mod table;

pub use entries::nib::nib;

/// Convenience for `nib(buffer)` with an owned buffer (upstream accepts a Buffer).
pub fn nib_from(buffer: Vec<u8>) -> r#struct::Struct {
    nib(Some(&buffer))
}
