// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/buffaloZnp.ts
// TODO: port BuffaloZnp.

pub struct BuffaloZnp {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZnp {
    pub fn new(buffer: Vec<u8>, position: usize) -> Self {
        Self { position, buffer }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    /// Upstream: `read(type, options)`.
    pub fn read(&mut self, parameter_type: u32, options: &serde_json::Value) -> serde_json::Value {
        todo!()
    }

    /// Upstream: `write(type, value, options)`.
    pub fn write(
        &mut self,
        parameter_type: u32,
        value: &serde_json::Value,
        options: &serde_json::Value,
    ) {
        todo!()
    }
}
