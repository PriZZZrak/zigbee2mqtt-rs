// Original: .upstream/zigbee-herdsman/src/zspec/utils.ts
// TODO: port remaining helpers.

pub fn channels_to_uint32_mask(channels: &[u8]) -> u32 {
    channels.iter().fold(0u32, |mask, channel| {
        if (11..=26).contains(channel) {
            mask | (1u32 << u32::from(*channel))
        } else {
            mask
        }
    })
}

pub fn uint32_mask_to_channels(mask: u32) -> Vec<u8> {
    (11..=26)
        .filter(|channel| mask & (1u32 << channel) != 0)
        .collect()
}

pub fn is_broadcast_address(address: u16) -> bool {
    matches!(address, 0xfffb | 0xfffc | 0xfffd | 0xffff)
}

pub fn eui64_le_buffer_to_hex(buffer: &[u8]) -> String {
    let mut reversed = buffer.to_vec();
    reversed.reverse();
    format!("0x{}", hex(&reversed))
}

pub fn eui64_be_buffer_to_hex(buffer: &[u8]) -> String {
    format!("0x{}", hex(buffer))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn crc16_x25(data: &[u8]) -> u16 {
    crc16(data, 0x1021, 0xffff, 0xffff, true)
}

pub fn crc16_xmodem(data: &[u8]) -> u16 {
    crc16(data, 0x1021, 0, 0, false)
}

pub fn crc16_ccitt(data: &[u8]) -> u16 {
    crc16(data, 0x1021, 0, 0, true)
}

pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    crc16(data, 0x1021, 0xffff, 0, false)
}

fn crc16(data: &[u8], poly: u16, init: u16, xorout: u16, reflected: bool) -> u16 {
    if reflected {
        let mut crc = init;
        for &byte in data {
            crc ^= u16::from(byte);
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ reflect16(poly);
                } else {
                    crc >>= 1;
                }
            }
        }
        crc ^ xorout
    } else {
        let mut crc = init;
        for &byte in data {
            crc ^= u16::from(byte) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ poly;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc ^ xorout
    }
}

fn reflect16(mut value: u16) -> u16 {
    let mut result = 0u16;
    for _ in 0..16 {
        result = (result << 1) | (value & 1);
        value >>= 1;
    }
    result
}

pub fn aes128_mmo_hash(data: &[u8]) -> Vec<u8> {
    let mut result = [0u8; 16];
    let mut position = 0usize;

    while data.len() - position >= 16 {
        let mut block = [0u8; 16];
        block.copy_from_slice(&data[position..position + 16]);
        aes128_mmo_update(&mut result, &block);
        position += 16;
    }

    let mut temp = [0u8; 16];
    let remaining = data.len() - position;
    temp[..remaining].copy_from_slice(&data[position..]);
    temp[remaining] = 0x80;

    if 16 - remaining < 3 {
        aes128_mmo_update(&mut result, &temp);
        temp = [0u8; 16];
    }

    temp[14] = ((data.len() >> 5) & 0xff) as u8;
    temp[15] = ((data.len() << 3) & 0xff) as u8;
    aes128_mmo_update(&mut result, &temp);

    result.to_vec()
}

fn aes128_mmo_update(result: &mut [u8; 16], block: &[u8; 16]) {
    let encrypted = aes128_encrypt_block(result, block);
    for i in 0..16 {
        result[i] = encrypted[i] ^ block[i];
    }
}

const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

fn aes128_encrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
    let mut round_keys = [0u8; 176];
    round_keys[..16].copy_from_slice(key);
    let mut bytes_generated = 16;

    for rcon in RCON.iter().skip(1) {
        let mut temp = [
            round_keys[bytes_generated - 4],
            round_keys[bytes_generated - 3],
            round_keys[bytes_generated - 2],
            round_keys[bytes_generated - 1],
        ];
        temp.rotate_left(1);
        for value in &mut temp {
            *value = SBOX[*value as usize];
        }
        temp[0] ^= *rcon;

        for i in 0..4 {
            round_keys[bytes_generated] = round_keys[bytes_generated - 16] ^ temp[i];
            bytes_generated += 1;
        }
        for _ in 0..3 {
            for i in 0..4 {
                round_keys[bytes_generated] =
                    round_keys[bytes_generated - 16] ^ round_keys[bytes_generated - 4];
                bytes_generated += 1;
            }
        }
    }

    let mut state = *block;
    add_round_key(&mut state, &round_keys[0..16]);

    for round in 1..10 {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, &round_keys[round * 16..round * 16 + 16]);
    }

    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, &round_keys[160..176]);
    state
}

fn add_round_key(state: &mut [u8; 16], key: &[u8]) {
    for i in 0..16 {
        state[i] ^= key[i];
    }
}

fn sub_bytes(state: &mut [u8; 16]) {
    for value in state.iter_mut() {
        *value = SBOX[*value as usize];
    }
}

fn shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    // State layout is column-major: index = row + 4 * column.
    for row in 0..4 {
        for column in 0..4 {
            state[row + 4 * column] = original[row + 4 * ((column + row) % 4)];
        }
    }
}

fn xtime(value: u8) -> u8 {
    if value & 0x80 != 0 {
        (value << 1) ^ 0x1b
    } else {
        value << 1
    }
}

fn mix_columns(state: &mut [u8; 16]) {
    for column in 0..4 {
        let i = 4 * column;
        let a = [state[i], state[i + 1], state[i + 2], state[i + 3]];
        state[i] = xtime(a[0]) ^ (xtime(a[1]) ^ a[1]) ^ a[2] ^ a[3];
        state[i + 1] = a[0] ^ xtime(a[1]) ^ (xtime(a[2]) ^ a[2]) ^ a[3];
        state[i + 2] = a[0] ^ a[1] ^ xtime(a[2]) ^ (xtime(a[3]) ^ a[3]);
        state[i + 3] = (xtime(a[0]) ^ a[0]) ^ a[1] ^ a[2] ^ xtime(a[3]);
    }
}
