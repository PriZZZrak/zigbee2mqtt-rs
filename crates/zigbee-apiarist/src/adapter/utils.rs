// Original: .upstream/zigbee-herdsman/src/adapter/utils.ts
// TODO: port remaining adapter helpers.

/// Upstream `parseTcpPath()` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpPath {
    pub host: String,
    pub port: u16,
}

pub fn is_tcp_path(path: &str) -> bool {
    path.starts_with("tcp://") || path.starts_with("socket://")
}

/// Upstream `parseTcpPath(path)`, accepts IPv4/IPv6/hostname.
pub fn parse_tcp_path(path: &str) -> TcpPath {
    try_parse_tcp_path(path).expect("Invalid TCP path")
}

pub fn try_parse_tcp_path(path: &str) -> Result<TcpPath, ()> {
    let rest = path
        .strip_prefix("tcp://")
        .or_else(|| path.strip_prefix("socket://"))
        .ok_or(())?;
    if rest.starts_with('[') {
        let end = rest.find(']').ok_or(())?;
        let host = rest[1..end].to_string();
        let port = rest
            .get(end + 1..)
            .and_then(|remainder| remainder.strip_prefix(':'))
            .and_then(|port| port.parse::<u16>().ok())
            .ok_or(())?;
        return Ok(TcpPath { host, port });
    }
    let (host, port) = rest.rsplit_once(':').ok_or(())?;
    if host.is_empty() || host.contains(':') {
        return Err(());
    }
    let port = port.parse::<u16>().map_err(|_| ())?;
    Ok(TcpPath {
        host: host.to_string(),
        port,
    })
}

pub fn read_backup() {
    todo!()
}
