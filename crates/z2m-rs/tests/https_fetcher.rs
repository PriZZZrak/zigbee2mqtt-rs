// Hermetic HTTPS fetcher tests against a local TLS server (real rustls
// handshake, self-signed CA), plus one opt-in run against public mirrors.
//
// Covers: TLS, Content-Length, chunked transfer, redirects (same host and
// rejected off-host), truncated body, timeout, size cap, non-200 status and
// SHA-256 mismatch refusal at the cache layer.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use converter_artifact::{CacheConfig, Manifest, ModuleCache};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use sha2::{Digest, Sha256};
use z2m_rs::converter::http::{HttpsFetcher, HttpsFetcherConfig};

#[derive(Clone)]
enum Scripted {
    Bytes {
        status: u16,
        content_type: &'static str,
        body: Vec<u8>,
        chunked: bool,
    },
    Redirect(&'static str),
    Truncated {
        declared: usize,
        body: Vec<u8>,
    },
    Stall(Duration),
    NotFound,
}

impl Scripted {
    fn ok(body: &[u8]) -> Self {
        Self::Bytes {
            status: 200,
            content_type: "application/javascript",
            body: body.to_vec(),
            chunked: false,
        }
    }
    fn chunked(body: &[u8]) -> Self {
        Self::Bytes {
            status: 200,
            content_type: "application/javascript",
            body: body.to_vec(),
            chunked: true,
        }
    }
}

struct TestServer {
    address: SocketAddr,
    trust: PathBuf,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<(String, String, String)>>>,
    handle: Option<std::thread::JoinHandle<()>>,
    directory: PathBuf,
}

impl TestServer {
    fn start(routes: Vec<(&'static str, Scripted)>) -> Option<Self> {
        if std::process::Command::new("openssl")
            .arg("version")
            .output()
            .is_err()
        {
            return None;
        }
        let directory = std::env::temp_dir().join(format!(
            "z2m-https-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let cert = directory.join("cert.pem");
        let key = directory.join("key.pem");
        let generated = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "ec",
                "-pkeyopt",
                "ec_paramgen_curve:prime256v1",
                "-nodes",
                "-keyout",
                key.to_str().unwrap(),
                "-out",
                cert.to_str().unwrap(),
                "-days",
                "1",
                "-subj",
                "/CN=localhost",
                "-addext",
                "subjectAltName=DNS:localhost,IP:127.0.0.1",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-addext",
                "keyUsage=critical,digitalSignature",
                "-addext",
                "extendedKeyUsage=serverAuth",
            ])
            .output()
            .unwrap();
        assert!(
            generated.status.success(),
            "openssl: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        let provider = Arc::new(rustls_rustcrypto::provider());
        let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(
            std::fs::File::open(&cert).unwrap(),
        ))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let private_key = rustls_pemfile::private_key(&mut std::io::BufReader::new(
            std::fs::File::open(&key).unwrap(),
        ))
        .unwrap()
        .unwrap();
        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certs, private_key)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let routes = Arc::new(routes);
        let handle = {
            let stop = Arc::clone(&stop);
            let requests = Arc::clone(&requests);
            std::thread::spawn(move || {
                let config = Arc::new(config);
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = serve(stream, &config, &routes, &requests);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Some(Self {
            address,
            trust: cert,
            stop,
            requests,
            handle: Some(handle),
            directory,
        })
    }

    fn config(&self, max_bytes: u64, timeout: Duration) -> HttpsFetcherConfig {
        HttpsFetcherConfig {
            base_url: format!("https://127.0.0.1:{}/z2hc/", self.address.port()),
            max_bytes,
            timeout,
            max_redirects: 3,
            trust_pem: Some(self.trust.clone()),
            extra_asset_hosts: Vec::new(),
        }
    }

    fn requests(&self) -> Vec<(String, String, String)> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn serve(
    stream: TcpStream,
    config: &ServerConfig,
    routes: &[(&str, Scripted)],
    requests: &Mutex<Vec<(String, String, String)>>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let connection = ServerConnection::new(Arc::new(config.clone()))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let mut tls = StreamOwned::new(connection, stream);
    let mut request = Vec::new();
    let mut buffer = [0u8; 4096];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = tls.read(&mut buffer)?;
        if read == 0 {
            return Ok(());
        }
        request.extend_from_slice(&buffer[..read]);
        if request.len() > 16 * 1024 {
            return Ok(());
        }
    }
    let text = String::from_utf8_lossy(&request).to_string();
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();
    let mut host = String::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("host") {
                host = value.trim().to_owned();
            }
        }
    }
    requests.lock().unwrap().push((method, host, path.clone()));
    let scripted = routes
        .iter()
        .find(|(route, _)| path == **route)
        .map(|(_, scripted)| scripted.clone())
        .unwrap_or(Scripted::NotFound);
    match scripted {
        Scripted::Bytes {
            status,
            content_type,
            body,
            chunked,
        } => {
            let header = if chunked {
                format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                )
            } else {
                format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
            };
            tls.write_all(header.as_bytes())?;
            if chunked {
                for piece in body.chunks(7) {
                    tls.write_all(format!("{:x}\r\n", piece.len()).as_bytes())?;
                    tls.write_all(piece)?;
                    tls.write_all(b"\r\n")?;
                }
                tls.write_all(b"0\r\n\r\n")?;
            } else {
                tls.write_all(&body)?;
            }
        }
        Scripted::Redirect(location) => {
            tls.write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )?;
        }
        Scripted::Truncated { declared, body } => {
            tls.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )?;
            tls.write_all(&body)?;
        }
        Scripted::Stall(duration) => {
            std::thread::sleep(duration);
            tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")?;
        }
        Scripted::NotFound => {
            tls.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )?;
        }
    }
    let _ = tls.flush();
    tls.conn.send_close_notify();
    let _ = tls.conn.write_tls(&mut tls.sock);
    Ok(())
}

fn fetch(server: &TestServer, path: &str) -> Result<Vec<u8>, converter_artifact::FetchError> {
    let config = server.config(1 << 20, Duration::from_secs(2));
    let fetcher = HttpsFetcher::new(config).unwrap();
    converter_artifact::ModuleFetcher::fetch(&fetcher, &module_record(path, &[]))
}

fn module_record(file: &str, bytes: &[u8]) -> converter_artifact::ModuleRecord {
    converter_artifact::ModuleRecord {
        id: file.trim_end_matches(".js").to_owned(),
        file: file.to_owned(),
        asset: Some(file.to_owned()),
        bytes: bytes.len() as u64,
        sha256: hash(bytes),
        definitions: 0,
        requires: Vec::new(),
    }
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn tls_content_length_and_chunked_bodies() {
    let Some(server) = TestServer::start(vec![
        ("/z2hc/plain.js", Scripted::ok(b"plain-body")),
        ("/z2hc/chunked.js", Scripted::chunked(b"chunked-body!")),
    ]) else {
        return;
    };
    assert_eq!(fetch(&server, "plain.js").unwrap(), b"plain-body");
    assert_eq!(fetch(&server, "chunked.js").unwrap(), b"chunked-body!");
    let requests = server.requests();
    assert_eq!(requests[0].0, "GET");
    assert!(requests[0]
        .1
        .ends_with(&format!(":{}", server.address.port())));
    assert_eq!(requests[0].2, "/z2hc/plain.js");
    assert_eq!(requests[1].2, "/z2hc/chunked.js");
}

#[test]
fn same_host_redirect_is_followed_and_cross_host_is_rejected() {
    let Some(server) = TestServer::start(vec![
        ("/z2hc/redirect.js", Scripted::Redirect("/z2hc/plain.js")),
        (
            "/z2hc/offhost.js",
            Scripted::Redirect("https://example.org/elsewhere.js"),
        ),
        ("/z2hc/plain.js", Scripted::ok(b"plain-body")),
    ]) else {
        return;
    };
    assert_eq!(fetch(&server, "redirect.js").unwrap(), b"plain-body");
    let error = fetch(&server, "offhost.js").unwrap_err();
    assert!(format!("{error:?}").contains("trusted host"), "{error:?}");
}

#[test]
fn truncated_body_stalls_and_http_errors_are_explicit() {
    let Some(server) = TestServer::start(vec![
        (
            "/z2hc/truncated.js",
            Scripted::Truncated {
                declared: 100,
                body: b"only-ten!!".to_vec(),
            },
        ),
        ("/z2hc/stall.js", Scripted::Stall(Duration::from_secs(2))),
        ("/z2hc/missing.js", Scripted::NotFound),
    ]) else {
        return;
    };
    let error = fetch(&server, "truncated.js").unwrap_err();
    assert!(format!("{error:?}").contains("truncated"), "{error:?}");
    let config = server.config(1 << 20, Duration::from_millis(150));
    let fetcher = HttpsFetcher::new(config).unwrap();
    let started = Instant::now();
    let error =
        converter_artifact::ModuleFetcher::fetch(&fetcher, &module_record("stall.js", b"ok"))
            .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "timeout was not enforced: {:?}",
        started.elapsed()
    );
    assert!(matches!(error, converter_artifact::FetchError::Network(_)));
    let error = fetch(&server, "missing.js").unwrap_err();
    assert!(format!("{error:?}").contains("404"), "{error:?}");
}

#[test]
fn size_cap_is_enforced_from_headers() {
    let Some(server) = TestServer::start(vec![("/z2hc/big.js", Scripted::ok(&vec![b'x'; 4096]))])
    else {
        return;
    };
    let config = server.config(64, Duration::from_secs(2));
    let fetcher = HttpsFetcher::new(config).unwrap();
    let error = converter_artifact::ModuleFetcher::fetch(&fetcher, &module_record("big.js", b""))
        .unwrap_err();
    assert!(matches!(error, converter_artifact::FetchError::TooLarge));
}

#[test]
fn hash_mismatch_never_activates_a_cached_module() {
    let Some(server) = TestServer::start(vec![
        ("/z2hc/ok.js", Scripted::ok(b"good-module")),
        ("/z2hc/bad.js", Scripted::ok(b"evil-module")),
    ]) else {
        return;
    };
    let manifest_value = serde_json::json!({
        "manifestSchema": 1,
        "artifactId": "https-test",
        "upstream": {},
        "host": {"abi": 1, "engine": "quickjs-oxide", "engineVersion": "0.5.0", "zclCatalogSha256": "00", "requiredFeatures": []},
        "index": {"file": "index.z2idx1", "format": "z2idx1", "version": 1, "bytes": 0, "sha256": hash(b""), "jsonBytes": 0, "jsonSha256": hash(b""), "lookupKeys": 0, "definitions": 0, "candidates": 0},
        "graph": {"file": "graph.json", "bytes": 0, "sha256": hash(b"")},
        "shared": {"root": "dist", "files": [], "bytes": 0, "sha256": hash(b"")},
        "modules": {"format": "cjs", "packageIncluded": false, "retrieval": format!("https://127.0.0.1:{}/z2hc/", server.address.port()), "entries": [
            {"id": "ok", "file": "ok.js", "bytes": 11, "sha256": hash(b"good-module"), "definitions": 1, "requires": []},
            {"id": "bad", "file": "bad.js", "bytes": 11, "sha256": hash(b"good-module"), "definitions": 1, "requires": []}
        ]},
        "compatibility": {"minHostAbi": 1, "maxHostAbi": 1}
    });
    let manifest =
        Manifest::parse(serde_json::to_vec(&manifest_value).unwrap().as_slice()).unwrap();
    let root = std::env::temp_dir().join(format!("z2m-https-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let cache_config = CacheConfig::new(&root);
    let cache = ModuleCache::open(cache_config.clone(), manifest).unwrap();
    let config = server.config(1 << 20, Duration::from_secs(2));
    let fetcher = HttpsFetcher::new(config).unwrap();
    cache
        .ensure(&["ok".to_owned()], &fetcher, &cache_config, true)
        .unwrap();
    assert_eq!(cache.load_ready("ok").unwrap(), b"good-module");
    let error = cache
        .ensure(&["bad".to_owned()], &fetcher, &cache_config, true)
        .unwrap_err();
    assert!(
        error.to_string().contains("sha256") || error.to_string().contains("hash"),
        "{error}"
    );
    assert!(cache.load_ready("bad").is_err());
    let _ = std::fs::remove_dir_all(&root);
}

/// Opt-in against public mirrors: `cargo test -p z2m-rs --test https_fetcher
/// -- --ignored`. Requires outbound HTTPS.
#[test]
#[ignore]
fn public_mirror_tls_redirect_and_stream() {
    let fetch_url = |base: &str, file: &str, max: u64, redirects: usize| {
        let config = HttpsFetcherConfig {
            base_url: base.to_owned(),
            max_bytes: max,
            timeout: Duration::from_secs(20),
            max_redirects: redirects,
            trust_pem: None,
            extra_asset_hosts: Vec::new(),
        };
        let fetcher = HttpsFetcher::new(config).unwrap();
        converter_artifact::ModuleFetcher::fetch(&fetcher, &module_record(file, b""))
    };
    // Public root store + real TLS handshake.
    let body = fetch_url("https://example.com/", "", 64 << 10, 0).unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("Example Domain"), "unexpected body: {text}");
    // Two same-host redirects.
    let body = fetch_url("https://httpbin.org/", "redirect/2", 64 << 10, 3).unwrap();
    assert!(!body.is_empty());
    // Chunked transfer.
    let body = fetch_url("https://httpbin.org/", "stream/3", 64 << 10, 0).unwrap();
    assert!(String::from_utf8_lossy(&body).contains("\"id\""));
}

/// Opt-in real GitHub Releases asset: validates CDN redirects and public TLS
/// without an API token. Run with `--ignored`.
#[test]
#[ignore]
fn public_github_release_asset_follows_cdn_redirects() {
    let config = HttpsFetcherConfig {
        base_url: "https://github.com/cli/cli/releases/download/v2.63.2/".to_owned(),
        max_bytes: 1 << 20,
        timeout: Duration::from_secs(30),
        max_redirects: 3,
        trust_pem: None,
        extra_asset_hosts: Vec::new(),
    };
    let fetcher = HttpsFetcher::new(config).unwrap();
    let mut module = module_record("unused", b"");
    module.asset = Some("gh_2.63.2_checksums.txt".to_owned());
    let body = converter_artifact::ModuleFetcher::fetch(&fetcher, &module).unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("gh_2.63.2_linux_amd64.tar.gz"), "{text}");
}

const CONVERTER_RELEASE: &str = "https://github.com/PriZZZrak/zigbee2mqtt-rs/releases/download/converter-vz2hc-26.125.0-zh-11.2.0-abi1/";

/// Opt-in against the published converter Release: fetches a real module and
/// checks its manifest SHA-256. Run with `--ignored`.
#[test]
#[ignore]
fn public_converter_release_module_matches_manifest() {
    let config = HttpsFetcherConfig {
        base_url: CONVERTER_RELEASE.to_owned(),
        max_bytes: 16 << 20,
        timeout: Duration::from_secs(30),
        max_redirects: 3,
        trust_pem: None,
        extra_asset_hosts: Vec::new(),
    };
    let fetcher = HttpsFetcher::new(config).unwrap();
    let mut module = module_record("tuya-1", b"");
    module.asset = Some("tuya-1.js".to_owned());
    let bytes = converter_artifact::ModuleFetcher::fetch(&fetcher, &module).unwrap();
    assert_eq!(bytes.len(), 174084);
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        digest,
        "055da5279646d4113df5e8f05965fdcdf2148cbfe3f73db6a71ab36d73a1ab55"
    );
}

/// Opt-in end-to-end cache fetch from the published Release: the ModuleCache
/// downloads the real module, verifies size/SHA-256 and activates it.
#[test]
#[ignore]
fn public_converter_release_module_activates_in_cache() {
    let root = std::env::temp_dir().join(format!("z2m-release-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let manifest = serde_json::json!({
        "manifestSchema": 1,
        "artifactId": "release-cache-test",
        "upstream": {},
        "host": {"abi": 1, "engine": "quickjs-oxide", "engineVersion": "0.5.0", "zclCatalogSha256": "00", "requiredFeatures": []},
        "index": {"file": "index.z2idx1", "format": "z2idx1", "version": 1, "bytes": 0, "sha256": hash(b""), "jsonBytes": 0, "jsonSha256": hash(b""), "lookupKeys": 0, "definitions": 0, "candidates": 0},
        "graph": {"file": "graph.json", "bytes": 0, "sha256": hash(b"")},
        "shared": {"root": "dist", "files": [], "bytes": 0, "sha256": hash(b"")},
        "modules": {"format": "cjs", "packageIncluded": false, "retrieval": CONVERTER_RELEASE, "entries": [
            {"id": "tuya-1", "file": "dist/devices/tuya-1.js", "asset": "tuya-1.js", "bytes": 174084, "sha256": "055da5279646d4113df5e8f05965fdcdf2148cbfe3f73db6a71ab36d73a1ab55", "definitions": 1, "requires": []}
        ]},
        "compatibility": {"minHostAbi": 1, "maxHostAbi": 1}
    });
    let manifest = Manifest::parse(serde_json::to_vec(&manifest).unwrap().as_slice()).unwrap();
    let cache_config = CacheConfig::new(&root);
    let cache = ModuleCache::open(cache_config.clone(), manifest).unwrap();
    let config = HttpsFetcherConfig {
        base_url: CONVERTER_RELEASE.to_owned(),
        max_bytes: 16 << 20,
        timeout: Duration::from_secs(30),
        max_redirects: 3,
        trust_pem: None,
        extra_asset_hosts: Vec::new(),
    };
    let fetcher = HttpsFetcher::new(config).unwrap();
    let report = cache
        .ensure(&["tuya-1".to_owned()], &fetcher, &cache_config, true)
        .unwrap();
    assert_eq!(report.fetched, 1);
    assert_eq!(cache.load_ready("tuya-1").unwrap().len(), 174084);
    let _ = std::fs::remove_dir_all(&root);
}
