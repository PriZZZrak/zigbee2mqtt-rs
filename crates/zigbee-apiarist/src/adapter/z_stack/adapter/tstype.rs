// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/tstype.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `ZnpVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ZnpVersion {
    ZStack12 = 0,
    ZStack3x0 = 1,
    ZStack30x = 2,
}

impl ZnpVersion {
    pub fn from_product(product: u64) -> Option<Self> {
        match product {
            0 => Some(Self::ZStack12),
            1 => Some(Self::ZStack3x0),
            2 => Some(Self::ZStack30x),
            _ => None,
        }
    }
}
