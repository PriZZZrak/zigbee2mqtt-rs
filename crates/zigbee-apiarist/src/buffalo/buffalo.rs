// Original: .upstream/zigbee-herdsman/src/buffalo/buffalo.ts
// TODO: port remaining helpers and bounds diagnostics.

#[allow(non_snake_case)]
pub struct Buffalo {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl Buffalo {
    pub fn new(buffer: Vec<u8>, position: usize) -> Self {
        Self { position, buffer }
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

    pub fn is_more(&self) -> bool {
        self.position < self.buffer.len()
    }

    fn write_slice(&mut self, values: &[u8]) {
        if self.position + values.len() > self.buffer.len() {
            self.buffer.resize(self.position + values.len(), 0);
        }
        self.buffer[self.position..self.position + values.len()].copy_from_slice(values);
        self.position += values.len();
    }

    fn read_slice(&mut self, length: usize) -> Vec<u8> {
        let end = (self.position + length).min(self.buffer.len());
        let values = self.buffer[self.position..end].to_vec();
        self.position = end;
        values
    }

    fn write_unsigned(&mut self, value: u64, length: usize) {
        let bytes = value.to_le_bytes();
        self.write_slice(&bytes[..length]);
    }

    fn read_unsigned(&mut self, length: usize) -> u64 {
        let bytes = self.read_slice(length);
        let mut value = 0u64;
        for (index, byte) in bytes.iter().enumerate() {
            value |= u64::from(*byte) << (index * 8);
        }
        value
    }

    fn read_signed(&mut self, length: usize) -> i64 {
        let value = self.read_unsigned(length) as i128;
        let bits = length * 8;
        let sign_bit = 1i128 << (bits - 1);
        if value & sign_bit != 0 {
            (value - (1i128 << bits)) as i64
        } else {
            value as i64
        }
    }

    pub fn write_uint8(&mut self, value: u64) {
        self.write_unsigned(value, 1);
    }
    pub fn read_uint8(&mut self) -> u64 {
        self.read_unsigned(1)
    }
    pub fn write_uint16(&mut self, value: u64) {
        self.write_unsigned(value, 2);
    }
    pub fn read_uint16(&mut self) -> u64 {
        self.read_unsigned(2)
    }
    pub fn write_uint24(&mut self, value: u64) {
        self.write_unsigned(value, 3);
    }
    pub fn read_uint24(&mut self) -> u64 {
        self.read_unsigned(3)
    }
    pub fn write_uint32(&mut self, value: u64) {
        self.write_unsigned(value, 4);
    }
    pub fn read_uint32(&mut self) -> u64 {
        self.read_unsigned(4)
    }
    pub fn write_uint40(&mut self, value: u64) {
        self.write_unsigned(value, 5);
    }
    pub fn read_uint40(&mut self) -> u64 {
        self.read_unsigned(5)
    }
    pub fn write_uint48(&mut self, value: u64) {
        self.write_unsigned(value, 6);
    }
    pub fn read_uint48(&mut self) -> u64 {
        self.read_unsigned(6)
    }
    pub fn write_uint56(&mut self, value: u64) {
        self.write_unsigned(value, 7);
    }
    pub fn read_uint56(&mut self) -> u64 {
        self.read_unsigned(7)
    }
    pub fn write_uint64(&mut self, value: u64) {
        self.write_unsigned(value, 8);
    }
    pub fn read_uint64(&mut self) -> u64 {
        self.read_unsigned(8)
    }

    pub fn write_int8(&mut self, value: i64) {
        self.write_unsigned(value as u64, 1);
    }
    pub fn read_int8(&mut self) -> i64 {
        self.read_signed(1)
    }
    pub fn write_int16(&mut self, value: i64) {
        self.write_unsigned(value as u64, 2);
    }
    pub fn read_int16(&mut self) -> i64 {
        self.read_signed(2)
    }
    pub fn write_int24(&mut self, value: i64) {
        self.write_unsigned(value as u64, 3);
    }
    pub fn read_int24(&mut self) -> i64 {
        self.read_signed(3)
    }
    pub fn write_int32(&mut self, value: i64) {
        self.write_unsigned(value as u64, 4);
    }
    pub fn read_int32(&mut self) -> i64 {
        self.read_signed(4)
    }
    pub fn write_int40(&mut self, value: i64) {
        self.write_unsigned(value as u64, 5);
    }
    pub fn read_int40(&mut self) -> i64 {
        self.read_signed(5)
    }
    pub fn write_int48(&mut self, value: i64) {
        self.write_unsigned(value as u64, 6);
    }
    pub fn read_int48(&mut self) -> i64 {
        self.read_signed(6)
    }
    pub fn write_int56(&mut self, value: i64) {
        self.write_unsigned(value as u64, 7);
    }
    pub fn read_int56(&mut self) -> i64 {
        self.read_signed(7)
    }
    pub fn write_int64(&mut self, value: i64) {
        self.write_unsigned(value as u64, 8);
    }
    pub fn read_int64(&mut self) -> i64 {
        self.read_signed(8)
    }

    pub fn write_float_le(&mut self, value: f64) {
        self.write_slice(&(value as f32).to_le_bytes());
    }
    pub fn read_float_le(&mut self) -> f64 {
        let bytes = self.read_slice(4);
        let mut array = [0u8; 4];
        array[..bytes.len()].copy_from_slice(&bytes);
        f64::from(f32::from_le_bytes(array))
    }
    pub fn write_double_le(&mut self, value: f64) {
        self.write_slice(&value.to_le_bytes());
    }
    pub fn read_double_le(&mut self) -> f64 {
        let bytes = self.read_slice(8);
        let mut array = [0u8; 8];
        array[..bytes.len()].copy_from_slice(&bytes);
        f64::from_le_bytes(array)
    }

    pub fn write_ieee_addr(&mut self, value: &str) {
        let hex = value.trim_start_matches("0x");
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        let chars: Vec<char> = hex.chars().collect();
        let mut index = chars.len();
        while index >= 2 {
            let pair: String = chars[index - 2..index].iter().collect();
            bytes.push(u8::from_str_radix(&pair, 16).unwrap_or(0));
            index -= 2;
        }
        self.write_slice(&bytes);
    }

    pub fn read_ieee_addr(&mut self) -> String {
        let bytes = self.read_slice(8);
        let hex = bytes
            .iter()
            .rev()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("0x{hex}")
    }

    pub fn write_buffer(&mut self, values: &[u8], length: usize) {
        let take = length.min(values.len());
        self.write_slice(&values[..take]);
    }

    pub fn read_buffer(&mut self, length: usize) -> Vec<u8> {
        self.read_slice(length)
    }

    pub fn write_list_uint8(&mut self, values: &[i64]) {
        for value in values {
            self.write_uint8(*value as u64);
        }
    }
    pub fn read_list_uint8(&mut self, length: usize) -> Vec<i64> {
        (0..length).map(|_| self.read_uint8() as i64).collect()
    }
    pub fn write_list_uint16(&mut self, values: &[i64]) {
        for value in values {
            self.write_uint16(*value as u64);
        }
    }
    pub fn read_list_uint16(&mut self, length: usize) -> Vec<i64> {
        (0..length).map(|_| self.read_uint16() as i64).collect()
    }
    pub fn write_list_uint24(&mut self, values: &[i64]) {
        for value in values {
            self.write_uint24(*value as u64);
        }
    }
    pub fn read_list_uint24(&mut self, length: usize) -> Vec<i64> {
        (0..length).map(|_| self.read_uint24() as i64).collect()
    }
    pub fn write_list_uint32(&mut self, values: &[i64]) {
        for value in values {
            self.write_uint32(*value as u64);
        }
    }
    pub fn read_list_uint32(&mut self, length: usize) -> Vec<i64> {
        (0..length).map(|_| self.read_uint32() as i64).collect()
    }

    pub fn write_utf8_string(&mut self, value: &str) {
        self.write_slice(value.as_bytes());
    }
    pub fn read_utf8_string(&mut self, length: usize) -> String {
        String::from_utf8_lossy(&self.read_slice(length)).to_string()
    }
}
