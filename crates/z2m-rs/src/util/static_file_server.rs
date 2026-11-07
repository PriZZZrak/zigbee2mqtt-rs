// Original: .upstream/zigbee2mqtt/lib/util/staticFileServer.ts
// TODO: port the actual file-serving implementation.
//
// Upstream wires `createStaticFileServer(dir, logError)` into `node:http`.
// Rust tests model the request/response pair directly so the same behaviour
// can be asserted without starting a socket.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub struct StaticFileServer {
    pub dir: PathBuf,
    pub log_error: Arc<dyn Fn(&str) + Send + Sync>,
    /// Models upstream's `response.setHeader` throwing inside the failure test.
    pub fail_on_content_security_policy: bool,
}

impl StaticFileServer {
    /// Upstream: `createStaticFileServer(dir, logError)` used as the http handler.
    pub fn handle(&self, request: &HttpRequest) -> HttpResponse {
        todo!()
    }
}

pub fn create_static_file_server(
    dir: PathBuf,
    log_error: Arc<dyn Fn(&str) + Send + Sync>,
) -> StaticFileServer {
    todo!()
}

/// Test-only mirror of the upstream handler whose `setHeader` throws.
pub fn create_failing_static_file_server(
    dir: PathBuf,
    log_error: Arc<dyn Fn(&str) + Send + Sync>,
) -> StaticFileServer {
    todo!()
}

/// Upstream: `sendNotFound(request, response)`.
pub fn send_not_found(request: &HttpRequest) -> HttpResponse {
    todo!()
}
