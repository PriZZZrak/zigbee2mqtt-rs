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
        _options: &serde_json::Value,
    ) {
        crate::zspec::zcl::zcl_frame::write_data_type(&mut self.buffer, data_type as u8, value);
        self.position = self.buffer.len();
    }

    pub fn read(&mut self, data_type: u32, _options: &serde_json::Value) -> serde_json::Value {
        crate::zspec::zcl::zcl_frame::read_data_type(
            &self.buffer,
            &mut self.position,
            data_type as u8,
        )
    }

    pub fn write_buffer(&mut self, values: &[u8], length: usize) {
        let take = length.min(values.len());
        if self.position + take > self.buffer.len() {
            self.buffer.resize(self.position + take, 0);
        }
        self.buffer[self.position..self.position + take].copy_from_slice(&values[..take]);
        self.position += take;
    }

    pub fn read_buffer(&mut self, length: usize) -> Vec<u8> {
        let end = (self.position + length).min(self.buffer.len());
        let values = self.buffer[self.position..end].to_vec();
        self.position = end;
        values
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
