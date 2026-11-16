// Original: .upstream/zigbee2mqtt/lib/util/onboarding.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Ported surface: data-directory preparation, configuration validation/migration
// gate, `/data`, `/submit` and `/submit-zip` handlers, the failure page payload
// and the ZIP extraction with the upstream path-traversal guards.
//
// Not ported: the Windfront onboarding page bundle (static assets are deferred
// to OpenWrt packaging); the HTTP loop serves `util::static_file_server`.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::util::data;
use crate::util::settings::Settings;
use crate::util::settings_schema;
use crate::util::static_file_server::{create_static_file_server, HttpRequest, HttpResponse};
use crate::util::stringify::stringify;
use crate::util::yaml;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const DEFAULT_URL: &str = "http://0.0.0.0:8080";

/// Upstream `getServerUrl()` (`onboarding.ts:14-16`).
pub fn server_url() -> (String, u16) {
    let raw = std::env::var("Z2M_ONBOARD_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    let rest = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
        .unwrap_or(&raw);
    let authority = rest.split('/').next().unwrap_or(rest);
    match authority.rsplit_once(':') {
        Some((host, port)) => (
            if host.is_empty() {
                "0.0.0.0".to_string()
            } else {
                host.to_string()
            },
            port.parse::<u16>().unwrap_or(8080),
        ),
        None => (authority.to_string(), 8080),
    }
}

/// Upstream `getZipEntryTargetPath(entryName)` (`onboarding.ts:18-34`).
pub fn get_zip_entry_target_path(entry_name: &str, base_path: &Path) -> Result<PathBuf, String> {
    let normalized = entry_name.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
    {
        return Err(format!("Invalid ZIP entry path '{entry_name}'"));
    }
    let target = base_path.join(&normalized);
    let relative = target
        .strip_prefix(base_path)
        .map_err(|_| format!("Unsafe ZIP entry path '{entry_name}'"))?;
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(format!("Unsafe ZIP entry path '{entry_name}'"));
    }
    Ok(target)
}

/// Upstream `extractZipDataToDataPath(zipContent)` (`onboarding.ts:36-58`).
pub fn extract_zip_data_to_data_path(zip_content: &[u8], base_path: &Path) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(zip_content)).map_err(|error| error.to_string())?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry
            .name()
            .map_err(|error| error.to_string())?
            .to_string();
        let target = get_zip_entry_target_path(&name, base_path)?;
        if name.ends_with('/') {
            std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|error| error.to_string())?;
        std::fs::write(&target, contents).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Upstream `OnboardData` (`types/api.ts`).
pub fn onboarding_data(
    settings: &Arc<Mutex<Settings>>,
    discovery: &zigbee_apiarist::adapter::adapter_discovery::DiscoveryContext,
) -> Value {
    let applied = settings.lock().unwrap().get();
    let devices = zigbee_apiarist::adapter::adapter_discovery::find_all_devices(discovery)
        .into_iter()
        .map(|adapter| {
            json!({
                "name": adapter.name,
                "path": adapter.path,
                "adapter": adapter.adapter.map(adapter_kind_name),
                "baudRate": adapter.baud_rate,
                "rtscts": adapter.rtscts,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "page": "form",
        "settings": applied,
        "settingsSchema": settings_schema::schema(),
        "devices": devices,
    })
}

fn adapter_kind_name(kind: zigbee_apiarist::adapter::adapter::AdapterKind) -> &'static str {
    use zigbee_apiarist::adapter::adapter::AdapterKind;
    match kind {
        AdapterKind::Deconz => "deconz",
        AdapterKind::Ember => "ember",
        AdapterKind::Ezsp => "ezsp",
        AdapterKind::ZStack => "zstack",
        AdapterKind::ZBoss => "zboss",
        AdapterKind::ZiGate => "zigate",
        AdapterKind::ZoH => "zoh",
    }
}

/// Upstream `onboard()` payload (`OnboardSubmitResponse`).
fn frontend_url(settings: &Arc<Mutex<Settings>>) -> Option<String> {
    if std::env::var("Z2M_ONBOARD_NO_REDIRECT").is_ok() {
        return None;
    }
    let applied = settings.lock().unwrap().get();
    let frontend = &applied["frontend"];
    if !frontend["enabled"].as_bool().unwrap_or(false) {
        return None;
    }
    if frontend["host"]
        .as_str()
        .is_some_and(|host| host.starts_with('/'))
    {
        return None;
    }
    let protocol = if frontend["ssl_cert"].as_str().is_some_and(|value| !value.is_empty())
        && frontend["ssl_key"].as_str().is_some_and(|value| !value.is_empty())
    {
        "https"
    } else {
        "http"
    };
    Some(format!(
        "{protocol}://{}:{}{}",
        frontend["host"].as_str().unwrap_or("localhost"),
        frontend["port"].as_u64().unwrap_or(8080),
        frontend["base_url"].as_str().unwrap_or("/")
    ))
}

/// Upstream `/submit` handler (`onboarding.ts:81-136`).
pub fn submit_settings(settings: &Arc<Mutex<Settings>>, body: &[u8]) -> (u16, Value) {
    let parsed: Value = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(error) => {
                return (
                    406,
                    json!({"success": false, "error": format!("Failed to parse request body: {error}")}),
                )
            }
        }
    };
    if let Err(error) = settings.lock().unwrap().apply(parsed) {
        return (406, json!({"success": false, "error": error}));
    }
    (
        200,
        json!({"success": true, "frontendUrl": frontend_url(settings)}),
    )
}

/// Upstream `/submit-zip` handler (`onboarding.ts:138-180`).
pub fn submit_zip(base_path: &Path, body: &[u8]) -> (u16, Value) {
    if body.is_empty() {
        return (
            406,
            json!({"success": false, "error": "Invalid ZIP payload: missing content"}),
        );
    }
    let decoded = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.decode(body)
    };
    let decoded = match decoded {
        Ok(decoded) => decoded,
        Err(error) => {
            return (406, json!({"success": false, "error": error.to_string()}));
        }
    };
    match extract_zip_data_to_data_path(&decoded, base_path) {
        Ok(()) => (200, json!({"success": true, "frontendUrl": null})),
        Err(error) => (406, json!({"success": false, "error": error})),
    }
}

/// Upstream `startFailureServer()` `/data` payload.
pub fn failure_payload(errors: &[String]) -> Value {
    json!({"page": "failure", "errors": errors})
}

fn json_response(status: u16, payload: &Value) -> HttpResponse {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    HttpResponse {
        status,
        headers,
        body: stringify(payload).into_bytes(),
    }
}

/// Upstream onboarding HTTP routing (`onboarding.ts:69-186`), minus the static
/// asset bundle.
pub fn handle_request(
    settings: &Arc<Mutex<Settings>>,
    base_path: &Path,
    request: &HttpRequest,
    body: &[u8],
    static_server: &crate::util::static_file_server::StaticFileServer,
) -> HttpResponse {
    route(settings, base_path, request, body, static_server).0
}

/// Routing plus whether the onboarding flow is complete (`onboarding.ts:69-186`).
fn route(
    settings: &Arc<Mutex<Settings>>,
    base_path: &Path,
    request: &HttpRequest,
    body: &[u8],
    static_server: &crate::util::static_file_server::StaticFileServer,
) -> (HttpResponse, bool) {
    let pathname = request
        .target
        .split(['?', '#'])
        .next()
        .unwrap_or("/");
    if request.method == "GET" && pathname == "/data" {
        let discovery = zigbee_apiarist::adapter::adapter_discovery::DiscoveryContext::default();
        let payload = onboarding_data(settings, &discovery);
        return (json_response(200, &payload), false);
    }
    if request.method == "POST" && pathname == "/submit" {
        let (status, payload) = submit_settings(settings, body);
        return (json_response(status, &payload), status == 200);
    }
    if request.method == "POST" && pathname == "/submit-zip" {
        let (status, payload) = submit_zip(base_path, body);
        return (json_response(status, &payload), status == 200);
    }
    (static_server.handle(request), false)
}

/// Upstream `onboard()` (`onboarding.ts:276-325`).
pub async fn onboard(
    settings: &Arc<Mutex<Settings>>,
    data_path: &Path,
    force_server: bool,
) -> Result<bool, String> {
    if !data_path.exists() {
        std::fs::create_dir_all(data_path).map_err(|error| error.to_string())?;
    }
    let configuration_path = data_path.join("configuration.yaml");
    let conf_exists = configuration_path.exists();

    if conf_exists {
        if let Err(error) = yaml::read(&configuration_path) {
            return Err(format!(
                "Your configuration file: '{}' is invalid ({error})",
                configuration_path.display()
            ));
        }
        let errors = settings.lock().unwrap().validate_non_required();
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        settings.lock().unwrap().write();
    } else {
        settings.lock().unwrap().write();
    }

    let needs_server = std::env::var("Z2M_ONBOARD_NO_SERVER").is_err()
        && (std::env::var("Z2M_ONBOARD_FORCE_RUN").is_ok() || !conf_exists || force_server);
    if needs_server {
        settings.lock().unwrap().set_onboarding(true);
        // The HTTP loop is started by `main`; the data-directory state is ready.
    }

    settings.lock().unwrap().re_read();
    let errors = settings.lock().unwrap().validate();
    if errors.is_empty() {
        Ok(true)
    } else {
        Err(errors.join("\n"))
    }
}

/// Test helper mirroring upstream `startFailureServer`'s ready state.
pub fn onboarding_ready(data_path: &Path) -> bool {
    data_path.exists() && data_path.join("configuration.yaml").exists()
}

/// Upstream `data.getPath()` default for the onboarding handlers.
pub fn default_data_path() -> PathBuf {
    data::get_path()
}

/// Minimal HTTP/1.1 request parser used by the onboarding loop
/// (`Content-Length` bodies only, like the upstream handlers require).
pub fn read_http_request(
    stream: &mut impl Read,
) -> std::io::Result<(HttpRequest, Vec<u8>)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break Some(position);
        }
        if buffer.len() > 1 << 20 {
            break None;
        }
    };
    let Some(header_end) = header_end else {
        return Ok((HttpRequest::default(), Vec::new()));
    };
    let headers_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = headers_text.split("\r\n");
    let mut request = HttpRequest::default();
    if let Some(first) = lines.next() {
        let mut parts = first.split_whitespace();
        request.method = parts.next().unwrap_or("").to_string();
        request.target = parts.next().unwrap_or("/").to_string();
    }
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse::<usize>().unwrap_or(0);
            }
            request
                .headers
                .insert(name.to_ascii_lowercase(), value.to_string());
        }
    }
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            // Upstream express emits a request stream error for a truncated
            // body; the handler must not accept a partial upload.
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "request body shorter than Content-Length",
            ));
        }
        body.extend_from_slice(&chunk[..read]);
    }
    Ok((request, body))
}

/// Write an `HttpResponse` back to the socket.
pub fn write_http_response(
    stream: &mut impl Write,
    response: &HttpResponse,
) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        404 => "Not Found",
        406 => "Not Acceptable",
        _ => "Internal Server Error",
    };
    let mut text = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        reason,
        response.body.len()
    );
    for (name, value) in &response.headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str("\r\n");
    stream.write_all(text.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

async fn read_request_async(
    stream: &mut tokio::net::TcpStream,
) -> std::io::Result<(HttpRequest, Vec<u8>)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break Some(position);
        }
        if buffer.len() > 1 << 20 {
            break None;
        }
    };
    let Some(header_end) = header_end else {
        return Ok((HttpRequest::default(), Vec::new()));
    };
    let headers_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = headers_text.split("\r\n");
    let mut request = HttpRequest::default();
    if let Some(first) = lines.next() {
        let mut parts = first.split_whitespace();
        request.method = parts.next().unwrap_or("").to_string();
        request.target = parts.next().unwrap_or("/").to_string();
    }
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse::<usize>().unwrap_or(0);
            }
            request
                .headers
                .insert(name.to_ascii_lowercase(), value.to_string());
        }
    }
    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    Ok((request, body))
}

async fn write_response_async(
    stream: &mut tokio::net::TcpStream,
    response: &HttpResponse,
) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        404 => "Not Found",
        406 => "Not Acceptable",
        _ => "Internal Server Error",
    };
    let mut text = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        reason,
        response.body.len()
    );
    for (name, value) in &response.headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str("\r\n");
    stream.write_all(text.as_bytes()).await?;
    stream.write_all(&response.body).await?;
    stream.flush().await
}

/// Serve onboarding until a request resolves the flow (`onboarding.ts:60-236`).
pub async fn run_server(
    settings: Arc<Mutex<Settings>>,
    data_path: PathBuf,
    static_dir: PathBuf,
) -> Result<bool, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (host, port) = server_url();
    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .map_err(|error| error.to_string())?;
    let server = Arc::new(create_static_file_server(
        static_dir,
        Arc::new(|message: &str| eprintln!("{message}")),
    ));
    let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };
        let finished_flag = completed.clone();
        let settings = settings.clone();
        let data_path = data_path.clone();
        let server = server.clone();
        tokio::spawn(async move {
            let Ok((request, body)) = read_request_async(&mut stream).await else {
                return;
            };
            let (response, finished) =
                route(&settings, &data_path, &request, &body, &server);
            let _ = write_response_async(&mut stream, &response).await;
            if finished {
                finished_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        if completed.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
    }
    Ok(true)
}

/// Upstream `startFailureServer()` (`onboarding.ts:238-274`).
pub async fn run_failure_server(errors: Vec<String>, static_dir: PathBuf) -> Result<(), String> {
    let (host, port) = server_url();
    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .map_err(|error| error.to_string())?;
    let server = Arc::new(create_static_file_server(
        static_dir,
        Arc::new(|message: &str| eprintln!("{message}")),
    ));
    let errors = Arc::new(errors);
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };
        let server = server.clone();
        let errors = errors.clone();
        tokio::spawn(async move {
            let Ok((request, _body)) = read_request_async(&mut stream).await else {
                return;
            };
            let pathname = request.target.split(['?', '#']).next().unwrap_or("/");
            let response = if request.method == "GET" && pathname == "/data" {
                json_response(200, &failure_payload(&errors))
            } else if request.method == "POST" && pathname == "/submit" {
                HttpResponse {
                    status: 200,
                    ..HttpResponse::default()
                }
            } else {
                server.handle(&request)
            };
            let _ = write_response_async(&mut stream, &response).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_traversal_entries() {
        let base = std::path::Path::new("/tmp/z2m-onboard-base");
        assert!(get_zip_entry_target_path("../evil", base).is_err());
        assert!(get_zip_entry_target_path("/etc/passwd", base).is_err());
        assert!(get_zip_entry_target_path("a/../../b", base).is_err());
        assert!(get_zip_entry_target_path("", base).is_err());
        assert_eq!(
            get_zip_entry_target_path("configuration.yaml", base).unwrap(),
            base.join("configuration.yaml")
        );
    }

    #[test]
    fn extracts_zip_into_the_data_path() {
        let dir = std::env::temp_dir().join(format!(
            "z2m-onboard-zip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            writer
                .start_file(
                    "configuration.yaml",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(b"homeassistant: {enabled: true}").unwrap();
            writer.finish().unwrap();
        }
        extract_zip_data_to_data_path(&buffer.into_inner(), &dir).unwrap();
        let contents = std::fs::read_to_string(dir.join("configuration.yaml")).unwrap();
        assert_eq!(contents, "homeassistant: {enabled: true}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn static_server(dir: &Path) -> crate::util::static_file_server::StaticFileServer {
        create_static_file_server(dir.to_path_buf(), Arc::new(|_: &str| {}))
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "z2m-onboard-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn data_endpoint_returns_settings_schema_and_devices() {
        let dir = temp_dir("data");
        let settings = Arc::new(Mutex::new(Settings::from_config(json!({
            "mqtt": {"server": "localhost"},
        }))));
        let request = HttpRequest {
            method: "GET".to_string(),
            target: "/data?x=1".to_string(),
            headers: Default::default(),
        };
        let response = handle_request(&settings, &dir, &request, &[], &static_server(&dir));
        assert_eq!(response.status, 200);
        assert_eq!(
            response.headers.get("Content-Type").map(String::as_str),
            Some("application/json")
        );
        let payload: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload["page"], "form");
        assert_eq!(payload["settings"]["mqtt"]["server"], "localhost");
        assert!(payload["settingsSchema"]["properties"]["mqtt"].is_object());
        assert!(payload["devices"].is_array());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn submit_endpoint_applies_settings() {
        let dir = temp_dir("submit");
        let settings = Arc::new(Mutex::new(Settings::from_config(json!({
            "mqtt": {"server": "localhost"},
        }))));
        let request = HttpRequest {
            method: "POST".to_string(),
            target: "/submit".to_string(),
            headers: Default::default(),
        };
        let body = json!({"frontend": {"enabled": true, "port": 8081}}).to_string();
        let response = handle_request(
            &settings,
            &dir,
            &request,
            body.as_bytes(),
            &static_server(&dir),
        );
        assert_eq!(response.status, 200);
        let payload: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload["success"], true);
        assert_eq!(payload["frontendUrl"], "http://localhost:8081/");
        assert_eq!(
            settings.lock().unwrap().get()["frontend"]["port"],
            json!(8081)
        );

        // Invalid body is rejected with the upstream 406 payload.
        let response = handle_request(
            &settings,
            &dir,
            &request,
            b"not json",
            &static_server(&dir),
        );
        assert_eq!(response.status, 406);
        let payload: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload["success"], false);
        assert!(payload["error"].as_str().unwrap().contains("parse"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn submit_zip_endpoint_extracts_and_requires_content() {
        let dir = temp_dir("submit-zip");
        let settings = Arc::new(Mutex::new(Settings::from_config(json!({
            "mqtt": {"server": "localhost"},
        }))));
        let request = HttpRequest {
            method: "POST".to_string(),
            target: "/submit-zip".to_string(),
            headers: Default::default(),
        };

        let response = handle_request(&settings, &dir, &request, b"", &static_server(&dir));
        assert_eq!(response.status, 406);
        let payload: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload["error"], "Invalid ZIP payload: missing content");

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            writer
                .start_file(
                    "configuration.yaml",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(b"advanced: {}").unwrap();
            writer.finish().unwrap();
        }
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
        let response = handle_request(
            &settings,
            &dir,
            &request,
            encoded.as_bytes(),
            &static_server(&dir),
        );
        assert_eq!(response.status, 200);
        assert_eq!(
            std::fs::read_to_string(dir.join("configuration.yaml")).unwrap(),
            "advanced: {}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failure_payload_has_the_failure_page() {
        assert_eq!(
            failure_payload(&["bad".to_string()]),
            json!({"page": "failure", "errors": ["bad"]})
        );
    }
}
