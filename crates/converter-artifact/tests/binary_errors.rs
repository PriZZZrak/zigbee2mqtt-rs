use converter_artifact::from_binary;

#[test]
fn malformed_index_is_rejected_without_panicking() {
    assert!(from_binary(b"not-an-index").is_err());
    let mut header = b"Z2IDX1".to_vec();
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&[0, 0]);
    header.extend_from_slice(&1_000_001u32.to_le_bytes()); // implausible strings
    header.extend_from_slice(&0u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    assert!(from_binary(&header).is_err());
}
