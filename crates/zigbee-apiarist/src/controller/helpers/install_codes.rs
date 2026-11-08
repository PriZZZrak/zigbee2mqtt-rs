// Original: .upstream/zigbee-herdsman/src/controller/helpers/installCodes.ts

pub fn check_install_code(code: &[u8], adjust: bool) -> (Vec<u8>, Option<String>) {
    const INSTALL_CODE_SIZES: [usize; 4] = [18, 8, 10, 14];
    const CRC_SIZE: usize = 2;

    for code_size in INSTALL_CODE_SIZES {
        if code.len() == code_size {
            let crc = crate::zspec::utils::crc16_x25(&code[..code.len() - CRC_SIZE]);
            let crc_low = (crc & 0xff) as u8;
            let crc_high = (crc >> 8) as u8;

            if code[code.len() - 2] != crc_low || code[code.len() - 1] != crc_high {
                if adjust {
                    let mut out = code.to_vec();
                    let len = out.len();
                    out[len - 2] = crc_low;
                    out[len - 1] = crc_high;
                    return (out, Some("invalid CRC".to_string()));
                }
                panic!("Install code {} failed CRC validation", hex(code));
            }

            return (code.to_vec(), None);
        }

        if code.len() == code_size - CRC_SIZE {
            if adjust {
                let crc = crate::zspec::utils::crc16_x25(code);
                let mut out = code.to_vec();
                out.extend_from_slice(&crc.to_le_bytes());
                return (out, Some("missing CRC".to_string()));
            }
            panic!("Install code {} failed CRC validation", hex(code));
        }
    }

    panic!("Install code {} has invalid size", hex(code));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn parse_install_code(install_code: &str) -> (String, String) {
    fn alnum_len(input: &str) -> usize {
        input
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric())
            .count()
    }

    fn ieee(value: &str) -> String {
        format!("0x{}", value.to_lowercase())
    }

    // `Z:<ieee16>$I:<key>`
    if let Some((_, rest)) = install_code.split_once("Z:") {
        let length = alnum_len(rest);
        if length == 16 {
            if let Some((ieee_part, key)) = rest[16..].split_once("$I:") {
                let key_length = alnum_len(key);
                if ieee_part.is_empty() && key_length > 0 {
                    return (ieee(&rest[..16]), key[..key_length].to_string());
                }
            }
        }
    }

    // `|ieee16|key|`
    let piped = install_code.trim_matches('|');
    if let Some((ieee_part, key)) = piped.split_once('|') {
        if ieee_part.len() == 16
            && ieee_part
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
            && !key.is_empty()
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return (ieee(ieee_part), key.to_string());
        }
    }

    // `G$M:...$A:<ieee16>$I:<key>`
    if let Some((_, rest)) = install_code.split_once("$A:") {
        if rest.len() > 16 && rest.is_char_boundary(16) {
            let ieee_part = &rest[..16];
            if let Some(key) = rest[16..].strip_prefix("$I:") {
                if ieee_part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
                    && !key.is_empty()
                    && key
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric())
                {
                    return (ieee(ieee_part), key.to_string());
                }
            }
        }
    }

    // `HUE:Z:<key> M:<ieee16>`
    if let Some(rest) = install_code.strip_prefix("HUE:Z:") {
        if let Some((key, model)) = rest.split_once(" M:") {
            if model.len() == 16
                && model
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
                && !key.is_empty()
                && key
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
            {
                return (ieee(model), key.to_string());
            }
        }
    }

    // Long QR payloads.
    if install_code.len() == 95 || install_code.len() == 91 {
        let key_start = install_code.len() - if install_code.len() == 95 { 36 } else { 32 };
        let ieee_part = &install_code[key_start - 19..key_start - 3];
        let key = &install_code[key_start..];
        if ieee_part
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return (ieee(ieee_part), key.to_string());
        }
    }

    panic!(
        "Unsupported install code, got {} chars, expected 95 or 91 chars, or known format",
        install_code.len()
    )
}
