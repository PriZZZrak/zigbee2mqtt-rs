// Original: .upstream/zigbee-herdsman/src/zspec/zcl/buffaloZcl.ts
// TODO: port BuffaloZcl.

#[derive(Debug, Clone)]
pub struct BuffaloZcl {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZcl {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    pub fn get_written(&self) -> Vec<u8> {
        self.buffer[..self.position.min(self.buffer.len())].to_vec()
    }

    pub fn write(
        &mut self,
        data_type: u32,
        value: &serde_json::Value,
        options: &serde_json::Value,
    ) {
        todo!()
    }

    pub fn read(&mut self, data_type: u32, options: &serde_json::Value) -> serde_json::Value {
        todo!()
    }

    pub fn write_buffer(&mut self, values: &[u8], length: usize) {
        todo!()
    }

    pub fn read_buffer(&mut self, length: usize) -> Vec<u8> {
        todo!()
    }

    pub fn write_structured_selector(
        &mut self,
        value: &serde_json::Value,
        options: &serde_json::Value,
    ) {
        todo!()
    }

    pub fn read_structured_selector(&mut self, options: &serde_json::Value) -> serde_json::Value {
        todo!()
    }
}
