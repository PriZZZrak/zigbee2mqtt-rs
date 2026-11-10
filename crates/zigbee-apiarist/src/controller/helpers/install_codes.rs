// Original: .upstream/zigbee-herdsman/src/controller/helpers/installCodes.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::zspec::consts::{INSTALL_CODE_CRC_SIZE, INSTALL_CODE_SIZES};
use crate::zspec::utils::crc16_x25;

fn is_alnum(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

fn alnum_prefix_length(value: &str) -> usize {
    value
        .chars()
        .take_while(|character| is_alnum(*character))
        .count()
}

fn first_alnum(value: &str, length: usize) -> Option<&str> {
    if value.len() < length || !value.is_char_boundary(length) {
        return None;
    }
    let candidate = &value[..length];
    if candidate.chars().all(is_alnum) {
        Some(candidate)
    } else {
        None
    }
}

fn ieee(value: &str) -> String {
    format!("0x{}", value.to_lowercase())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Upstream `parseInstallCode(installCode)` — installCodes.ts:16.
pub fn parse_install_code(install_code: &str) -> (String, String) {
    // Widely adopted (Ubisys, Danfoss, Inovelli, Ledvance):
    // `Z:<ieee16>$I:<key>` (unanchored).
    let mut search = 0usize;
    while let Some(offset) = install_code[search..].find("Z:") {
        let start = search + offset + 2;
        if let Some(rest) = install_code.get(start..) {
            if let Some(ieee_part) = first_alnum(rest, 16) {
                if let Some(key_part) = rest[16..].strip_prefix("$I:") {
                    let key_length = alnum_prefix_length(key_part);
                    if key_length > 0 {
                        if let Some(key) = first_alnum(key_part, key_length) {
                            return (ieee(ieee_part), key.to_string());
                        }
                    }
                }
            }
        }
        search = start;
        if search >= install_code.len() {
            break;
        }
    }

    // Pipe-separated (Muller-Licht, Innr) / pipe-enclosed (frient).
    let piped = install_code.strip_prefix('|').unwrap_or(install_code);
    if let Some((ieee_part, rest)) = piped.split_once('|') {
        let key = rest.strip_suffix('|').unwrap_or(rest);
        if ieee_part.len() == 16
            && ieee_part.chars().all(is_alnum)
            && !key.is_empty()
            && key.chars().all(is_alnum)
        {
            return (ieee(ieee_part), key.to_string());
        }
    }

    // Aqara: `G$M:...$A:<ieee16>$I:<key>` (anchored, greedy `.+`).
    if let Some(rest) = install_code.strip_prefix("G$M:") {
        let positions = rest
            .match_indices("$A:")
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        for position in positions.into_iter().rev() {
            if position == 0 {
                continue;
            }
            let after = &rest[position + 3..];
            if let Some(ieee_part) = first_alnum(after, 16) {
                if let Some(key) = after[16..].strip_prefix("$I:") {
                    if !key.is_empty() && key.chars().all(is_alnum) {
                        return (ieee(ieee_part), key.to_string());
                    }
                }
            }
        }
    }

    // Hue: `HUE:Z:<key> M:<ieee16>` (rest after the IEEE is ignored).
    if let Some(rest) = install_code.strip_prefix("HUE:Z:") {
        let key_length = alnum_prefix_length(rest);
        if key_length > 0 {
            if let Some(after) = rest
                .get(key_length..)
                .and_then(|value| value.strip_prefix(" M:"))
            {
                if let Some(ieee_part) = first_alnum(after, 16) {
                    return (ieee(ieee_part), rest[..key_length].to_string());
                }
            }
        }
    }

    // Long QR payloads (95 or 91 chars).
    let characters = install_code.chars().collect::<Vec<_>>();
    let length = characters.len();
    if length == 95 || length == 91 {
        let key_start = length - if length == 95 { 36 } else { 32 };
        let ieee_part = characters[key_start - 19..key_start - 3]
            .iter()
            .collect::<String>();
        let key = characters[key_start..].iter().collect::<String>();
        return (ieee(&ieee_part), key);
    }

    panic!(
        "Unsupported install code, got {} chars, expected 95 or 91 chars, or known format",
        length
    )
}

/// Upstream `checkInstallCode(code, adjust = true)` — installCodes.ts:69.
pub fn check_install_code(code: &[u8], adjust: bool) -> (Vec<u8>, Option<&'static str>) {
    let crc_low_index = code.len() - INSTALL_CODE_CRC_SIZE;
    let crc_high_index = code.len() - INSTALL_CODE_CRC_SIZE + 1;

    for code_size in INSTALL_CODE_SIZES {
        if code.len() == code_size {
            // Install code has CRC: check it, and replace if invalid.
            let crc = crc16_x25(&code[..code.len() - 2]);
            let crc_high = (crc >> 8) as u8;
            let crc_low = (crc & 0xff) as u8;

            if code[crc_low_index] != crc_low || code[crc_high_index] != crc_high {
                // 8 is the smallest valid length, so this is always reached for
                // an 8-length code with an invalid CRC.
                if adjust {
                    let mut out = code.to_vec();
                    out[crc_low_index] = crc_low;
                    out[crc_high_index] = crc_high;
                    return (out, Some("invalid CRC"));
                }
                panic!("Install code {} failed CRC validation", hex(code));
            }

            return (code.to_vec(), None);
        }

        if code.len() == code_size - INSTALL_CODE_CRC_SIZE {
            if adjust {
                // Install code is missing the CRC.
                let crc = crc16_x25(code);
                let mut out = code.to_vec();
                out.extend_from_slice(&crc.to_le_bytes());
                return (out, Some("missing CRC"));
            }
            panic!("Install code {} failed CRC validation", hex(code));
        }
    }

    panic!("Install code {} has invalid size", hex(code));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect()
    }

    /// Upstream `utils.test.ts` "Checks install codes of all lengths".
    #[test]
    fn checks_install_codes_of_all_lengths() {
        let panics = |code: Vec<u8>| {
            std::panic::catch_unwind(move || check_install_code(&code, false)).is_err()
        };

        assert!(panics(bytes("001122")));

        let code8_valid = bytes("83FED3407A932B70");
        let code8_invalid = bytes("FFFED3407A939723");
        let code8_invalid_fixed = bytes("FFFED3407A93DE84");
        let code8_missing = bytes("83FED3407A93");

        assert_eq!(
            check_install_code(&code8_valid, true),
            (code8_valid.clone(), None)
        );
        assert_eq!(
            check_install_code(&code8_invalid, true),
            (code8_invalid_fixed, Some("invalid CRC"))
        );
        assert!(panics(code8_invalid.clone()));
        assert_eq!(
            check_install_code(&code8_missing, true),
            (code8_valid.clone(), Some("missing CRC"))
        );
        assert!(panics(code8_missing));

        let code10_valid = bytes("83FED3407A93972397FC");
        let code10_invalid = bytes("FFFED3407A939723A5C6");
        let code10_invalid_fixed = bytes("FFFED3407A9397238C4F");
        let code10_missing = bytes("83FED3407A939723");
        let code10_missing_fixed = bytes("83FED3407A932B70");

        assert_eq!(
            check_install_code(&code10_valid, true),
            (code10_valid.clone(), None)
        );
        assert_eq!(
            check_install_code(&code10_invalid, true),
            (code10_invalid_fixed, Some("invalid CRC"))
        );
        assert!(panics(code10_invalid));
        assert_eq!(
            check_install_code(&code10_missing, true),
            (code10_missing_fixed, Some("invalid CRC"))
        );

        let code14_valid = bytes("83FED3407A939723A5C639FF4C12");
        let code14_invalid = bytes("FFFED3407A939723A5C639FF4C12");
        let code14_invalid_fixed = bytes("FFFED3407A939723A5C639FFDE74");
        let code14_missing = bytes("83FED3407A939723A5C639FF");

        assert_eq!(
            check_install_code(&code14_valid, true),
            (code14_valid.clone(), None)
        );
        assert_eq!(
            check_install_code(&code14_invalid, true),
            (code14_invalid_fixed, Some("invalid CRC"))
        );
        assert_eq!(
            check_install_code(&code14_missing, true),
            (code14_valid.clone(), Some("missing CRC"))
        );

        let code18_valid = bytes("83FED3407A939723A5C639B26916D505C3B5");
        let code18_invalid = bytes("FFFED3407A939723A5C639B26916D505C3B5");
        let code18_invalid_fixed = bytes("FFFED3407A939723A5C639B26916D505EEB1");
        let code18_missing = bytes("83FED3407A939723A5C639B26916D505");

        assert_eq!(
            check_install_code(&code18_valid, true),
            (code18_valid.clone(), None)
        );
        assert_eq!(
            check_install_code(&code18_invalid, true),
            (code18_invalid_fixed, Some("invalid CRC"))
        );
        assert_eq!(
            check_install_code(&code18_missing, true),
            (code18_valid.clone(), Some("missing CRC"))
        );
    }

    #[test]
    fn parses_known_install_code_formats() {
        assert_eq!(
            parse_install_code("Z:0011223344556677$I:00112233445566778899AABBCCDDEEFF"),
            (
                "0x0011223344556677".to_string(),
                "00112233445566778899AABBCCDDEEFF".to_string()
            )
        );
        assert_eq!(
            parse_install_code("|0011223344556677|00112233445566778899AABBCCDDEEFF|"),
            (
                "0x0011223344556677".to_string(),
                "00112233445566778899AABBCCDDEEFF".to_string()
            )
        );
        assert_eq!(
            parse_install_code("0011223344556677|00112233445566778899AABBCCDDEEFF"),
            (
                "0x0011223344556677".to_string(),
                "00112233445566778899AABBCCDDEEFF".to_string()
            )
        );
        assert_eq!(
            parse_install_code("G$M:some$A:0011223344556677$I:00112233445566778899AABBCCDDEEFF"),
            (
                "0x0011223344556677".to_string(),
                "00112233445566778899AABBCCDDEEFF".to_string()
            )
        );
        assert_eq!(
            parse_install_code(
                "HUE:Z:00112233445566778899AABBCCDDEEFF M:0011223344556677 trailing"
            ),
            (
                "0x0011223344556677".to_string(),
                "00112233445566778899AABBCCDDEEFF".to_string()
            )
        );
    }
}
