// Original: .upstream/zigbee-herdsman/src/adapter/utils.ts

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

/// Upstream `readBackup(path)`.
pub fn read_backup(path: &str) -> Result<Option<serde_json::Value>, String> {
    if !std::path::Path::new(path).exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "[BACKUP] Coordinator backup is corrupted. This can happen due to filesystem corruption. To re-create the backup, delete '{path}' and start again. ({error})"
        )
    })?;
    serde_json::from_str(&content).map(Some).map_err(|error| {
        format!(
            "[BACKUP] Coordinator backup is corrupted. This can happen due to filesystem corruption. To re-create the backup, delete '{path}' and start again. ({error})"
        )
    })
}
