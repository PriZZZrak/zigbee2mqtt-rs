// Original: .upstream/zigbee-herdsman/src/adapter/zoh/adapter/utils.ts
// TODO: port remaining ZoH utils.

pub fn big_u_int64_to_buffer_be(value: u64) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

pub fn big_u_int64_to_buffer_le(value: u64) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

pub fn big_u_int64_to_hex_be(value: u64) -> String {
    format!("{value:016x}")
}
