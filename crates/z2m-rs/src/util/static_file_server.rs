// Original: .upstream/zigbee2mqtt/lib/util/staticFileServer.ts
// TODO: port range/HEAD handling and conditional headers beyond ETag.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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
        if self.fail_on_content_security_policy {
            (self.log_error)(&format!(
                "Failed to serve '{}': socket gone",
                request.target
            ));
            return HttpResponse {
                status: 500,
                ..HttpResponse::default()
            };
        }

        let Some(relative) = decode_path(&request.target) else {
            return send_not_found(request);
        };
        if relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return send_not_found(request);
        }

        let requested = if relative.as_os_str().is_empty() {
            self.dir.join("index.html")
        } else {
            self.dir.join(&relative)
        };

        let resolved = match requested.canonicalize() {
            Ok(path) => path,
            Err(_) => return send_not_found(request),
        };
        if !resolved.starts_with(self.dir.canonicalize().unwrap_or_else(|_| self.dir.clone())) {
            return send_not_found(request);
        }

        let is_root = request.target == "/" || request.target.is_empty();
        let plain_content_type = content_type(&resolved).to_string();
        let (path, encoding) = choose_encoding(&resolved, request);
        let body = match std::fs::read(&path) {
            Ok(body) => body,
            Err(_) => return send_not_found(request),
        };

        let metadata = std::fs::metadata(&path).ok();
        let etag = metadata
            .as_ref()
            .map(|meta| {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0);
                format!("\"{}-{}\"", meta.len(), modified)
            })
            .unwrap_or_default();

        if request
            .headers
            .get("if-none-match")
            .is_some_and(|value| value == &etag)
        {
            return HttpResponse {
                status: 304,
                headers: BTreeMap::from([("etag".to_string(), etag)]),
                body: Vec::new(),
            };
        }

        let mut headers = BTreeMap::new();
        headers.insert("content-type".to_string(), plain_content_type);
        headers.insert("etag".to_string(), etag);
        if is_root {
            headers.insert("cache-control".to_string(), "no-store".to_string());
        }
        if let Some(encoding) = encoding {
            headers.insert("content-encoding".to_string(), encoding.to_string());
            headers.insert("vary".to_string(), "Accept-Encoding".to_string());
        } else {
            headers.insert("content-length".to_string(), body.len().to_string());
        }

        HttpResponse {
            status: 200,
            headers,
            body,
        }
    }
}

pub fn create_static_file_server(
    dir: PathBuf,
    log_error: Arc<dyn Fn(&str) + Send + Sync>,
) -> StaticFileServer {
    StaticFileServer {
        dir,
        log_error,
        fail_on_content_security_policy: false,
    }
}

/// Test-only mirror of the upstream handler whose `setHeader` throws.
pub fn create_failing_static_file_server(
    dir: PathBuf,
    log_error: Arc<dyn Fn(&str) + Send + Sync>,
) -> StaticFileServer {
    StaticFileServer {
        dir,
        log_error,
        fail_on_content_security_policy: true,
    }
}

/// Upstream: `sendNotFound(request, response)`.
pub fn send_not_found(request: &HttpRequest) -> HttpResponse {
    let body = format!("Cannot GET {}", request.target);
    HttpResponse {
        status: 404,
        headers: BTreeMap::from([
            (
                "content-type".to_string(),
                "text/html; charset=utf-8".to_string(),
            ),
            (
                "content-security-policy".to_string(),
                "default-src 'none'".to_string(),
            ),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
        ]),
        body: body.into_bytes(),
    }
}

fn decode_path(target: &str) -> Option<PathBuf> {
    let target = target.split(['?', '#']).next().unwrap_or(target);
    let mut bytes = Vec::with_capacity(target.len());
    let mut index = 0;
    let raw = target.as_bytes();
    while index < raw.len() {
        if raw[index] == b'%' && index + 2 < raw.len() {
            let hex = std::str::from_utf8(&raw[index + 1..index + 3]).ok()?;
            let byte = u8::from_str_radix(hex, 16).ok()?;
            bytes.push(byte);
            index += 3;
        } else {
            bytes.push(raw[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(bytes).ok()?;
    if decoded.contains('\0') {
        return None;
    }
    Some(PathBuf::from(decoded.trim_start_matches('/')))
}

fn choose_encoding(path: &Path, request: &HttpRequest) -> (PathBuf, Option<&'static str>) {
    let accepted = request
        .headers
        .get("accept-encoding")
        .map(String::as_str)
        .unwrap_or("");

    if accepted
        .split(',')
        .any(|item| item.trim().starts_with("br"))
    {
        let brotli = PathBuf::from(format!("{}.br", path.display()));
        if brotli.exists() {
            return (brotli, Some("br"));
        }
    }
    if accepted
        .split(',')
        .any(|item| item.trim().starts_with("gzip"))
    {
        let gzip = PathBuf::from(format!("{}.gz", path.display()));
        if gzip.exists() {
            return (gzip, Some("gzip"));
        }
    }

    (path.to_path_buf(), None)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
