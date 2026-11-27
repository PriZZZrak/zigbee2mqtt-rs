//! Production-shaped HTTPS module fetcher.
//!
//! URLs come only from the trusted manifest's retrieval base plus the trusted
//! module file path. Device data never contributes a URL. The cache manager
//! verifies SHA-256, size and ABI before activation; this module only performs
//! bounded HTTPS GET with timeouts, redirect limits and a response-size cap.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use converter_artifact::{FetchError, ModuleFetcher, ModuleRecord};
use rustls::pki_types::ServerName;
use rustls::{ClientConnection, RootCertStore, StreamOwned};

#[derive(Clone, Debug)]
pub struct HttpsFetcherConfig {
    pub base_url: String,
    pub max_bytes: u64,
    pub timeout: Duration,
    pub max_redirects: usize,
    /// Additional trusted CA bundle (PEM). Production leaves this empty and
    /// uses the OS trust store; tests point it at a local test CA.
    pub trust_pem: Option<std::path::PathBuf>,
    /// Additional redirect targets trusted by embedding code (tests). The
    /// installed manifest cannot extend this list.
    pub extra_asset_hosts: Vec<String>,
}

impl HttpsFetcherConfig {
    pub fn from_manifest(manifest: &converter_artifact::Manifest) -> Result<Self, String> {
        let base = manifest
            .modules
            .retrieval
            .clone()
            .ok_or_else(|| "manifest has no modules.retrieval URL".to_owned())?;
        Ok(Self {
            base_url: base,
            max_bytes: 16 << 20,
            timeout: Duration::from_secs(15),
            max_redirects: 3,
            trust_pem: None,
            extra_asset_hosts: Vec::new(),
        })
    }
}

pub struct HttpsFetcher {
    config: HttpsFetcherConfig,
    tls: Arc<rustls::ClientConfig>,
}

impl HttpsFetcher {
    /// Build from an installed artifact directory. The manifest is the only
    /// source of the retrieval URL and of the module list.
    pub fn from_artifact_dir(directory: &std::path::Path) -> Result<Self, String> {
        let manifest = converter_artifact::Manifest::load(&directory.join("manifest.json"))
            .map_err(|error| error.to_string())?;
        Self::new(HttpsFetcherConfig::from_manifest(&manifest)?)
    }

    pub fn new(config: HttpsFetcherConfig) -> Result<Self, String> {
        let _ = rustls::crypto::CryptoProvider::install_default(rustls_rustcrypto::provider());
        let provider = Arc::new(rustls_rustcrypto::provider());
        let mut roots = RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs().certs {
            let _ = roots.add(cert);
        }
        if let Some(path) = &config.trust_pem {
            let mut reader = std::io::BufReader::new(
                std::fs::File::open(path).map_err(|error| format!("trust pem: {error}"))?,
            );
            for cert in rustls_pemfile::certs(&mut reader) {
                roots
                    .add(cert.map_err(|error| format!("trust pem: {error}"))?)
                    .map_err(|error| format!("trust pem: {error}"))?;
            }
        }
        let tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|error| error.to_string())?
            .with_root_certificates(roots)
            .with_no_client_auth();
        // Validate the trusted base once, so a bad manifest fails closed.
        let base = parse_https(&config.base_url)?;
        if !base.path.ends_with('/') {
            return Err("modules.retrieval must end with '/'".to_owned());
        }
        Ok(Self {
            config,
            tls: Arc::new(tls),
        })
    }

    pub fn config(&self) -> &HttpsFetcherConfig {
        &self.config
    }

    fn fetch_path(&self, path: &str) -> Result<Vec<u8>, FetchError> {
        let deadline = std::time::Instant::now() + self.config.timeout;
        // An empty path means "the trusted base itself"; manifests always use
        // non-empty module files. The base is validated and cannot be supplied
        // by device data.
        if path.starts_with('/')
            || path.contains("..")
            || path.contains('\\')
            || path.chars().any(|character| character.is_control())
        {
            return Err(FetchError::Network("unsafe module path".to_owned()));
        }
        let base = parse_https(&self.config.base_url).map_err(FetchError::Network)?;
        let mut url = format!(
            "https://{}{}{}",
            base.host,
            if base.port == 443 {
                String::new()
            } else {
                format!(":{}", base.port)
            },
            base.path
        );
        url.push_str(path);
        let mut redirects = 0;
        let mut current = url;
        loop {
            let parsed = parse_https_redirect(&current).map_err(FetchError::Network)?;
            if !redirect_allowed(&base, &parsed, &self.config.extra_asset_hosts) {
                return Err(FetchError::Network("redirect left trusted host".to_owned()));
            }
            match self.get_once(&parsed, deadline)? {
                Response::Body(bytes) => return Ok(bytes),
                Response::Redirect(location) => {
                    redirects += 1;
                    if redirects > self.config.max_redirects {
                        return Err(FetchError::Network("too many redirects".to_owned()));
                    }
                    current = if location.starts_with("https://") {
                        location
                    } else if location.starts_with('/') {
                        format!("https://{}{}", parsed.host_with_port(), location)
                    } else {
                        return Err(FetchError::Network(
                            "relative/non-https redirect rejected".to_owned(),
                        ));
                    };
                }
            }
        }
    }

    fn get_once(
        &self,
        url: &ParsedUrl,
        deadline: std::time::Instant,
    ) -> Result<Response, FetchError> {
        let address = (url.host.as_str(), url.port)
            .to_socket_addrs()
            .map_err(|error| FetchError::Network(error.to_string()))?
            .next()
            .ok_or_else(|| FetchError::Network("no resolved address".to_owned()))?;
        let tcp = TcpStream::connect_timeout(&address, self.config.timeout)
            .map_err(|error| FetchError::Network(error.to_string()))?;
        tcp.set_read_timeout(Some(self.config.timeout)).ok();
        tcp.set_write_timeout(Some(self.config.timeout)).ok();
        let server = ServerName::try_from(url.host.clone())
            .map_err(|error| FetchError::Network(error.to_string()))?;
        let connection = ClientConnection::new(Arc::clone(&self.tls), server)
            .map_err(|error| FetchError::Network(error.to_string()))?;
        let mut stream = StreamOwned::new(connection, tcp);
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: z2m-rs-converter-cache/1\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n",
            url.path,
            url.host_with_port()
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|error| FetchError::Network(error.to_string()))?;
        stream.flush().ok();
        let mut response = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut header_end = None;
        while response.len() <= 32 * 1024 {
            if std::time::Instant::now() >= deadline {
                return Err(FetchError::Network("total timeout exceeded".to_owned()));
            }
            let read = stream
                .read(&mut chunk)
                .map_err(|error| FetchError::Network(error.to_string()))?;
            if read == 0 {
                break;
            }
            response.extend_from_slice(&chunk[..read]);
            if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(index + 4);
                break;
            }
        }
        let header_end =
            header_end.ok_or_else(|| FetchError::Network("incomplete HTTP headers".to_owned()))?;
        let headers = String::from_utf8_lossy(&response[..header_end]).to_string();
        let mut lines = headers.split("\r\n");
        let status_line = lines.next().unwrap_or_default();
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| FetchError::Network("invalid HTTP status".to_owned()))?;
        let mut content_length = None;
        let mut chunked = false;
        let mut location = None;
        for line in lines {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            match name.as_str() {
                "content-length" => content_length = value.parse::<u64>().ok(),
                "transfer-encoding" if value.eq_ignore_ascii_case("chunked") => chunked = true,
                "location" => location = Some(value.to_owned()),
                _ => {}
            }
        }
        if (300..400).contains(&status) {
            return location
                .map(Response::Redirect)
                .ok_or_else(|| FetchError::Network("redirect without Location".to_owned()));
        }
        if status != 200 {
            return Err(FetchError::Network(format!("HTTP status {status}")));
        }
        if content_length.is_some_and(|length| length > self.config.max_bytes) {
            return Err(FetchError::TooLarge);
        }
        let body = &response[header_end..];
        if let Some(length) = content_length {
            if body.len() as u64 >= length {
                return Ok(Response::Body(body[..length as usize].to_vec()));
            }
            let mut bytes = body.to_vec();
            let remaining = length.saturating_sub(bytes.len() as u64);
            let mut limited = stream.take(remaining);
            limited
                .read_to_end(&mut bytes)
                .map_err(|error| FetchError::Network(error.to_string()))?;
            if std::time::Instant::now() >= deadline {
                return Err(FetchError::Network("total timeout exceeded".to_owned()));
            }
            if bytes.len() as u64 != length {
                return Err(FetchError::Network("truncated response".to_owned()));
            }
            return Ok(Response::Body(bytes));
        }
        if chunked {
            return decode_chunked(body, &mut stream, self.config.max_bytes, deadline)
                .map(Response::Body);
        }
        let mut bytes = body.to_vec();
        let mut limited = stream.take(self.config.max_bytes.saturating_sub(bytes.len() as u64) + 1);
        limited
            .read_to_end(&mut bytes)
            .map_err(|error| FetchError::Network(error.to_string()))?;
        if std::time::Instant::now() >= deadline {
            return Err(FetchError::Network("total timeout exceeded".to_owned()));
        }
        if bytes.len() as u64 > self.config.max_bytes {
            return Err(FetchError::TooLarge);
        }
        Ok(Response::Body(bytes))
    }
}

impl ModuleFetcher for HttpsFetcher {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        // GitHub release assets are flat files; the manifest carries both the
        // package path (`file`) and the release asset name (`asset`).
        self.fetch_path(module.asset.as_deref().unwrap_or(&module.file))
    }
}

/// GitHub redirects release downloads to its CDN. Only the documented asset
/// hosts are accepted, and only when the trusted base is github.com. Any other
/// host is rejected; `extra` is a code-level test seam, never manifest data.
fn redirect_allowed(base: &ParsedUrl, target: &ParsedUrl, extra: &[String]) -> bool {
    const GITHUB_ASSET_HOSTS: &[&str] = &[
        "release-assets.githubusercontent.com",
        "objects.githubusercontent.com",
        "github-releases.githubusercontent.com",
    ];
    let trusted_extra = extra
        .iter()
        .any(|host| host.eq_ignore_ascii_case(&target.host));
    if target.host == base.host {
        return target.port == base.port || trusted_extra;
    }
    if base.host == "github.com"
        && target.port == 443
        && GITHUB_ASSET_HOSTS.contains(&target.host.as_str())
    {
        return true;
    }
    trusted_extra
}

#[derive(Debug)]
struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}
impl ParsedUrl {
    fn host_with_port(&self) -> String {
        if self.port == 443 {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}
fn parse_https(url: &str) -> Result<ParsedUrl, String> {
    parse_https_inner(url, false)
}

/// GitHub redirects to signed CDN URLs that carry a query string. Query
/// strings are accepted only for redirect targets; the trusted manifest base
/// stays query/fragment free.
fn parse_https_redirect(url: &str) -> Result<ParsedUrl, String> {
    parse_https_inner(url, true)
}

fn parse_https_inner(url: &str, allow_query: bool) -> Result<ParsedUrl, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| "only HTTPS artifact URLs are allowed".to_owned())?;
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty()
        || authority.contains('@')
        || authority.contains('?')
        || authority.contains('#')
    {
        return Err("invalid artifact authority".to_owned());
    }
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        (
            host.to_ascii_lowercase(),
            port.parse::<u16>()
                .map_err(|_| "invalid artifact port".to_owned())?,
        )
    } else {
        (authority.to_ascii_lowercase(), 443)
    };
    if host.is_empty() || host.chars().any(|character| character.is_control()) {
        return Err("invalid artifact host".to_owned());
    }
    let path = if authority_end == rest.len() {
        "/".to_owned()
    } else {
        rest[authority_end..].to_owned()
    };
    if path.contains('#') || (!allow_query && path.contains('?')) {
        return Err("artifact URL must not contain query/fragment".to_owned());
    }
    Ok(ParsedUrl { host, port, path })
}
#[derive(Debug)]
enum Response {
    Body(Vec<u8>),
    Redirect(String),
}
fn decode_chunked(
    initial: &[u8],
    stream: &mut impl Read,
    max: u64,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, FetchError> {
    let mut input = initial.to_vec();
    let mut output = Vec::new();
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(FetchError::Network("total timeout exceeded".to_owned()));
        }
        let Some(index) = input.windows(2).position(|window| window == b"\r\n") else {
            let mut buffer = [0u8; 4096];
            let read = stream
                .read(&mut buffer)
                .map_err(|error| FetchError::Network(error.to_string()))?;
            if read == 0 {
                return Err(FetchError::Network("truncated chunked body".to_owned()));
            }
            input.extend_from_slice(&buffer[..read]);
            continue;
        };
        let line = String::from_utf8_lossy(&input[..index]).to_string();
        let size = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| FetchError::Network("invalid chunk size".to_owned()))?;
        input.drain(..index + 2);
        if size == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(size) as u64 > max {
            return Err(FetchError::TooLarge);
        }
        while input.len() < size + 2 {
            let mut buffer = [0u8; 4096];
            let read = stream
                .read(&mut buffer)
                .map_err(|error| FetchError::Network(error.to_string()))?;
            if read == 0 {
                return Err(FetchError::Network("truncated chunk".to_owned()));
            }
            input.extend_from_slice(&buffer[..read]);
        }
        output.extend_from_slice(&input[..size]);
        input.drain(..size + 2);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_https;
    #[test]
    fn artifact_urls_are_https_and_host_scoped() {
        assert!(parse_https("https://artifacts.example/z2hc/").is_ok());
        assert!(parse_https("http://artifacts.example/").is_err());
        assert!(parse_https("https://user@artifacts.example/").is_err());
        assert!(parse_https("https://artifacts.example/path?x=1").is_err());
    }
    #[test]
    fn github_asset_redirects_are_allowlisted() {
        use super::{redirect_allowed, ParsedUrl};
        let base = ParsedUrl {
            host: "github.com".to_owned(),
            port: 443,
            path: "/PriZZZrak/zigbee2mqtt-rs/releases/download/converter-v1/".to_owned(),
        };
        let asset = ParsedUrl {
            host: "release-assets.githubusercontent.com".to_owned(),
            port: 443,
            path: "/github-production-release-asset/x".to_owned(),
        };
        assert!(redirect_allowed(&base, &asset, &[]));
        let objects = ParsedUrl {
            host: "objects.githubusercontent.com".to_owned(),
            port: 443,
            path: "/x".to_owned(),
        };
        assert!(redirect_allowed(&base, &objects, &[]));
        let evil = ParsedUrl {
            host: "evil.example".to_owned(),
            port: 443,
            path: "/x".to_owned(),
        };
        assert!(!redirect_allowed(&base, &evil, &[]));
        let http = ParsedUrl {
            host: "release-assets.githubusercontent.com".to_owned(),
            port: 80,
            path: "/x".to_owned(),
        };
        assert!(!redirect_allowed(&base, &http, &[]));
        let local = ParsedUrl {
            host: "127.0.0.1".to_owned(),
            port: 8443,
            path: "/x".to_owned(),
        };
        assert!(redirect_allowed(&base, &local, &["127.0.0.1".to_owned()]));
    }

    #[test]
    fn signed_cdn_queries_are_allowed_only_for_redirects() {
        use super::{parse_https, parse_https_redirect};
        assert!(parse_https("https://github.com/a/b/releases/download/x/m.js?token=1").is_err());
        let redirect =
            parse_https_redirect("https://release-assets.githubusercontent.com/x.js?sp=r&sig=abc")
                .unwrap();
        assert!(redirect.path.contains("?sp=r&sig=abc"));
        assert!(parse_https_redirect("https://host/x.js#fragment").is_err());
    }

    #[test]
    fn module_paths_cannot_escape() {
        let config = crate::converter::http::HttpsFetcherConfig {
            base_url: "https://artifacts.example/z2hc/".to_owned(),
            max_bytes: 10,
            timeout: std::time::Duration::from_secs(1),
            max_redirects: 1,
            trust_pem: None,
            extra_asset_hosts: Vec::new(),
        };
        let fetcher = crate::converter::http::HttpsFetcher::new(config).unwrap();
        assert!(fetcher.fetch_path("../etc/passwd").is_err());
        assert!(fetcher.fetch_path("/etc/passwd").is_err());
    }
}
