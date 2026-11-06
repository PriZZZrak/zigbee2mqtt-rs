// Original: .upstream/zigbee-herdsman/src/buffalo/buffalo.ts
// TODO: port Buffalo.

#[allow(non_snake_case)]
pub struct Buffalo {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl Buffalo {
    pub fn new(buffer: Vec<u8>, position: usize) -> Self {
        Self { position, buffer }
    }

    pub fn get_position(&self) -> usize { todo!() }
    pub fn set_position(&mut self, position: usize) { todo!() }
    pub fn get_buffer(&self) -> Vec<u8> { todo!() }
    pub fn get_written(&self) -> Vec<u8> { todo!() }
    pub fn is_more(&self) -> bool { todo!() }

    pub fn write_uint8(&mut self, value: u64) { todo!() }
    pub fn read_uint8(&mut self) -> u64 { todo!() }
    pub fn write_uint16(&mut self, value: u64) { todo!() }
    pub fn read_uint16(&mut self) -> u64 { todo!() }
    pub fn write_uint24(&mut self, value: u64) { todo!() }
    pub fn read_uint24(&mut self) -> u64 { todo!() }
    pub fn write_uint32(&mut self, value: u64) { todo!() }
    pub fn read_uint32(&mut self) -> u64 { todo!() }
    pub fn write_uint40(&mut self, value: u64) { todo!() }
    pub fn read_uint40(&mut self) -> u64 { todo!() }
    pub fn write_uint48(&mut self, value: u64) { todo!() }
    pub fn read_uint48(&mut self) -> u64 { todo!() }
    pub fn write_uint56(&mut self, value: u64) { todo!() }
    pub fn read_uint56(&mut self) -> u64 { todo!() }
    pub fn write_uint64(&mut self, value: u64) { todo!() }
    pub fn read_uint64(&mut self) -> u64 { todo!() }

    pub fn write_int8(&mut self, value: i64) { todo!() }
    pub fn read_int8(&mut self) -> i64 { todo!() }
    pub fn write_int16(&mut self, value: i64) { todo!() }
    pub fn read_int16(&mut self) -> i64 { todo!() }
    pub fn write_int24(&mut self, value: i64) { todo!() }
    pub fn read_int24(&mut self) -> i64 { todo!() }
    pub fn write_int32(&mut self, value: i64) { todo!() }
    pub fn read_int32(&mut self) -> i64 { todo!() }
    pub fn write_int40(&mut self, value: i64) { todo!() }
    pub fn read_int40(&mut self) -> i64 { todo!() }
    pub fn write_int48(&mut self, value: i64) { todo!() }
    pub fn read_int48(&mut self) -> i64 { todo!() }
    pub fn write_int56(&mut self, value: i64) { todo!() }
    pub fn read_int56(&mut self) -> i64 { todo!() }
    pub fn write_int64(&mut self, value: i64) { todo!() }
    pub fn read_int64(&mut self) -> i64 { todo!() }

    pub fn write_float_le(&mut self, value: f64) { todo!() }
    pub fn read_float_le(&mut self) -> f64 { todo!() }
    pub fn write_double_le(&mut self, value: f64) { todo!() }
    pub fn read_double_le(&mut self) -> f64 { todo!() }

    pub fn write_ieee_addr(&mut self, value: &str) { todo!() }
    pub fn read_ieee_addr(&mut self) -> String { todo!() }

    pub fn write_buffer(&mut self, values: &[u8], length: usize) { todo!() }
    pub fn read_buffer(&mut self, length: usize) -> Vec<u8> { todo!() }

    pub fn write_list_uint8(&mut self, values: &[i64]) { todo!() }
    pub fn read_list_uint8(&mut self, length: usize) -> Vec<i64> { todo!() }
    pub fn write_list_uint16(&mut self, values: &[i64]) { todo!() }
    pub fn read_list_uint16(&mut self, length: usize) -> Vec<i64> { todo!() }
    pub fn write_list_uint24(&mut self, values: &[i64]) { todo!() }
    pub fn read_list_uint24(&mut self, length: usize) -> Vec<i64> { todo!() }
    pub fn write_list_uint32(&mut self, values: &[i64]) { todo!() }
    pub fn read_list_uint32(&mut self, length: usize) -> Vec<i64> { todo!() }

    pub fn write_utf8_string(&mut self, value: &str) { todo!() }
    pub fn read_utf8_string(&mut self, length: usize) -> String { todo!() }
}
