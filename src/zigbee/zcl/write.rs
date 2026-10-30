
pub fn write_attributes_payload(
    sequence: u8,
    mfr_code: Option<u16>,
    attrs: &[(u16, u8, &[u8])],
) -> Vec<u8> {
    let mut frame = Vec::new();
    let fc = if mfr_code.is_some() { 0x14u8 } else { 0x10u8 };
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