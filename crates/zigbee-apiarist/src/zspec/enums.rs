// Original: .upstream/zigbee-herdsman/src/zspec/enums.ts

/// Upstream `BroadcastAddress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BroadcastAddress {
    /// Low power routers only.
    LOW_POWER_ROUTERS = 0xfffb,
    /// All routers and coordinator.
    DEFAULT = 0xfffc,
    /// `macRxOnWhenIdle = TRUE` (all non-sleepy devices).
    RX_ON_WHEN_IDLE = 0xfffd,
    /// All devices in PAN (including sleepy end devices).
    SLEEPY = 0xffff,
}
