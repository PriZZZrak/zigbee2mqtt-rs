// Original: .upstream/zigbee-herdsman/src/controller/helpers/installCodes.ts
// TODO: port public API from the original file.

// pub fn checkInstallCode() { todo!() }
// pub fn parseInstallCode() { todo!() }

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
    todo!()
}
