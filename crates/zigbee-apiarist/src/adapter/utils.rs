// Original: .upstream/zigbee-herdsman/src/adapter/utils.ts
// TODO: port public API from the original file.

/// Upstream `parseTcpPath()` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpPath {
    pub host: String,
    pub port: u16,
}

pub fn is_tcp_path(_path: &str) -> bool {
    todo!()
}

/// Upstream `parseTcpPath(path)`, accepts IPv4/IPv6/hostname.
pub fn parse_tcp_path(_path: &str) -> TcpPath {
    todo!()
}

pub fn read_backup() {
    todo!()
}
