// Original: .upstream/zigbee2mqtt/lib/util/settings.ts
// TODO: port public API (see original file).

// pub fn CURRENT_VERSION() { todo!() }
// pub fn LOG_LEVELS() { todo!() }
// pub fn addDevice() { todo!() }
// pub fn addGroup() { todo!() }
// pub fn apply() { todo!() }
// pub fn blockDevice() { todo!() }
// pub fn changeEntityOptions() { todo!() }
// pub fn changeFriendlyName() { todo!() }
// pub fn defaults() { todo!() }
// pub fn get() { todo!() }
// pub fn getDevice() { todo!() }
// pub fn getGroup() { todo!() }
// pub fn getPersistedSettings() { todo!() }
// pub fn reRead() { todo!() }
// pub fn removeDevice() { todo!() }
// pub fn removeGroup() { todo!() }
// pub fn set() { todo!() }
// pub fn setOnboarding() { todo!() }
// pub fn testing() { todo!() }
// pub fn validate() { todo!() }
// pub fn validateNonRequired() { todo!() }
// pub fn write() { todo!() }
// pub fn writeMinimalDefaults() { todo!() }
pub fn current_version() {
    todo!()
}
pub fn log_levels() {
    todo!()
}
pub fn add_device() {
    todo!()
}
pub fn add_group() {
    todo!()
}
pub fn apply() {
    todo!()
}
pub fn block_device() {
    todo!()
}
pub fn change_entity_options() {
    todo!()
}
pub fn change_friendly_name() {
    todo!()
}
pub fn defaults() {
    todo!()
}
pub fn get() {
    todo!()
}
pub fn get_device() {
    todo!()
}
pub fn get_group() {
    todo!()
}
pub fn get_persisted_settings() {
    todo!()
}
pub fn re_read() {
    todo!()
}
pub fn remove_device() {
    todo!()
}
pub fn remove_group() {
    todo!()
}
pub fn set() {
    todo!()
}
pub fn set_onboarding() {
    todo!()
}
pub fn testing() {
    todo!()
}
pub fn validate() {
    todo!()
}
pub fn validate_non_required() {
    todo!()
}
pub fn write() {
    todo!()
}
pub fn write_minimal_defaults() {
    todo!()
}

/// Minimal settings document used by the ported tests.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub data: serde_json::Value,
}

impl Settings {
    pub fn new(data: serde_json::Value) -> Self {
        Self { data }
    }

    /// Upstream: `settings.reRead()`.
    pub fn re_read(&mut self) {}

    pub fn get(&self) -> &serde_json::Value {
        &self.data
    }

    /// Upstream: `settings.validate()`.
    pub fn validate(&self) -> Vec<String> {
        todo!()
    }

    /// Upstream: `settings.blockDevice(ieeeAddr)`.
    pub fn block_device(&mut self, _ieee_address: &str) {
        todo!()
    }

    /// Upstream: `settings.changeFriendlyName(from, to)`.
    pub fn change_friendly_name(&mut self, _from: &str, _to: &str) -> Result<(), String> {
        todo!()
    }
}
