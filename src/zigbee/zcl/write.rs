pub fn write_attributes_payload(
    sequence: u8,
    mfr_code: Option<u16>,
    attrs: &[(u16, u8, &[u8])],
) -> Vec<u8> {
    write_attributes_payload_with_options(sequence, mfr_code, true, attrs)
}

/// Write Attributes frame with an explicit disable-default-response flag.
pub fn write_attributes_payload_with_options(
    sequence: u8,
    mfr_code: Option<u16>,
    disable_default_response: bool,
    attrs: &[(u16, u8, &[u8])],
) -> Vec<u8> {
    let mut frame = Vec::new();
    let mut fc = 0x00u8;
    if mfr_code.is_some() {
        fc |= 0x04;
    }
    if disable_default_response {
        fc |= 0x10;
    }
    frame.push(fc);
    if let Some(code) = mfr_code {
        frame.push((code & 0xFF) as u8);
        frame.push((code >> 8) as u8);
    }
    frame.push(sequence);
    frame.push(0x02);
    for (attr_id, data_type, value) in attrs {
        frame.push((attr_id & 0xFF) as u8);
        frame.push((attr_id >> 8) as u8);
        frame.push(*data_type);
        frame.extend_from_slice(value);
    }
    frame
}
