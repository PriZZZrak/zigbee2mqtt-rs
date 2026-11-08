// Original: .upstream/zigbee-herdsman/src/controller/touchlink.ts
//
// Controller-side Touchlink orchestration. Adapter inter-PAN primitives are
// supplied through `TouchlinkAdapter` (implemented by the concrete adapter).

use serde_json::Value;

pub trait TouchlinkAdapter: Send + Sync {
    async fn scan(&self, channel: u8) -> Result<Vec<Value>, String>;
    async fn identify(&self, channel: u8, ieee_address: u64) -> Result<(), String>;
    async fn factory_reset(&self, channel: u8, ieee_address: u64) -> Result<(), String>;
    async fn set_channel_inter_pan(&self, channel: u8) -> Result<(), String>;
    async fn restore_channel_inter_pan(&self) -> Result<(), String>;
}

pub struct Touchlink<A: TouchlinkAdapter> {
    adapter: A,
    current_channel: Option<u8>,
    locked: bool,
}

impl<A: TouchlinkAdapter> Touchlink<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            current_channel: None,
            locked: false,
        }
    }

    /// Upstream `setChannelInterPAN(channel)`.
    pub async fn set_channel_inter_pan(&mut self, channel: u8) -> Result<(), String> {
        self.adapter.set_channel_inter_pan(channel).await?;
        self.current_channel = Some(channel);
        Ok(())
    }

    /// Upstream `restoreChannelInterPAN()`.
    pub async fn restore_channel_inter_pan(&mut self) -> Result<(), String> {
        self.adapter.restore_channel_inter_pan().await?;
        self.current_channel = None;
        Ok(())
    }

    /// Upstream `lock()` / `unlock()`.
    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Upstream `scan()`.
    pub async fn scan(&mut self) -> Result<Vec<Value>, String> {
        if self.locked {
            return Err("Touchlink is locked".to_string());
        }
        let channel = self.current_channel.unwrap_or(11);
        self.adapter.scan(channel).await
    }

    /// Upstream `identify(entity)`.
    pub async fn identify(&mut self, ieee_address: u64) -> Result<(), String> {
        let channel = self.current_channel.unwrap_or(11);
        self.adapter.identify(channel, ieee_address).await
    }

    /// Upstream `factoryReset(entity)`.
    pub async fn factory_reset(&mut self, ieee_address: u64) -> Result<(), String> {
        let channel = self.current_channel.unwrap_or(11);
        self.adapter.factory_reset(channel, ieee_address).await
    }

    /// Upstream `factoryResetFirst()`: scan, then factory reset the first device.
    pub async fn factory_reset_first(&mut self) -> Result<Option<Value>, String> {
        let devices = self.scan().await?;
        let Some(first) = devices.first().cloned() else {
            return Ok(None);
        };
        let ieee = first
            .get("ieeeAddr")
            .and_then(Value::as_str)
            .and_then(|value| u64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
            .or_else(|| first.get("ieeeAddr").and_then(Value::as_u64))
            .unwrap_or(0);
        self.factory_reset(ieee).await?;
        Ok(Some(first))
    }
}
