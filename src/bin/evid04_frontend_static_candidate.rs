//! Static frontend differential driver (harness only).
//!
//! Serves the same fixture tree as the upstream `srvx` reference dump and
//! records status/headers/body for a representative set of requests: index,
//! assets (plain/gzip/brotli/preference), HEAD, byte ranges, extensionless
//! resolution, a conditional request and the 404 page.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use zigbee2mqtt_rs::config::MqttConfig;
use zigbee2mqtt_rs::devices::DeviceRegistry;
use zigbee2mqtt_rs::frontend::{self, ServeOptions};
use zigbee2mqtt_rs::mqtt::fake_broker::FakeBroker;
use zigbee2mqtt_rs::mqtt::MqttBridge;

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-frontend-static-candidate: {message}");
    std::process::exit(2);
}

async fn request(port: u16, method: &str, path: &str, headers: &[(&str, &str)]) -> Value {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap_or_else(|error| fail(format!("connect: {error}")));
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| fail("malformed HTTP response"));
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let body = &response[split + 4..];
    let mut lines = head.split("\r\n");
    let status: u16 = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let mut response_headers = Map::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            response_headers.insert(key.trim().to_ascii_lowercase(), json!(value.trim()));
        }
    }
    json!({
        "status": status,
        "headers": response_headers,
        "body": zigbee2mqtt_rs::frontend::ws::base64_encode(body),
    })
}

/// Pin file mtimes so ETag/Last-Modified are identical on both sides.
fn pin_mtime(path: &std::path::Path) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap_or_else(|error| fail(format!("open {}: {error}", path.display())));
    let pinned = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    file.set_modified(SystemTime::from(pinned))
        .unwrap_or_else(|error| fail(format!("set mtime {}: {error}", path.display())));
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let broker = FakeBroker::start().await;
    let cfg = MqttConfig {
        server: "127.0.0.1".into(),
        port: broker.port,
        base_topic: "zigbee2mqtt".into(),
        client_id: format!("z2m-frontend-static-{}", std::process::id()),
        ..MqttConfig::default()
    };
    let (events, receiver) = tokio::sync::broadcast::channel(16);
    let (mqtt, _commands) = MqttBridge::connect_with_events(&cfg, Some(events))
        .await
        .unwrap_or_else(|error| fail(format!("mqtt connect: {error}")));

    let dir = std::env::temp_dir().join(format!("z2m-frontend-static-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<html>index</html>").unwrap();
    std::fs::write(dir.join("about.html"), "<html>about</html>").unwrap();
    std::fs::write(dir.join("assets").join("app.js"), "console.log('app');").unwrap();
    std::fs::write(dir.join("assets").join("app.js.br"), "compressed-js").unwrap();
    std::fs::write(dir.join("assets").join("app.js.gz"), "gzipped-js").unwrap();
    for name in [
        "index.html",
        "about.html",
        "assets/app.js",
        "assets/app.js.br",
        "assets/app.js.gz",
    ] {
        pin_mtime(&dir.join(name));
    }

    let options = ServeOptions {
        host: Some("127.0.0.1".into()),
        port: 0,
        base_url: String::new(),
        auth_token: None,
        disable_ui_serving: false,
        static_dir: dir.clone(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| fail(format!("bind: {error}")));
    let port = listener
        .local_addr()
        .unwrap_or_else(|error| fail(format!("local addr: {error}")))
        .port();
    let devices = std::sync::Arc::new(DeviceRegistry::new());
    tokio::spawn(async move {
        let _ = frontend::serve_on(
            listener,
            options,
            "zigbee2mqtt".into(),
            devices,
            receiver,
            mqtt,
        )
        .await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut responses = Map::new();
    responses.insert("index".into(), request(port, "GET", "/", &[]).await);
    responses.insert(
        "asset".into(),
        request(port, "GET", "/assets/app.js", &[]).await,
    );
    responses.insert(
        "asset-br".into(),
        request(port, "GET", "/assets/app.js", &[("Accept-Encoding", "br")]).await,
    );
    responses.insert(
        "asset-gzip".into(),
        request(
            port,
            "GET",
            "/assets/app.js",
            &[("Accept-Encoding", "gzip")],
        )
        .await,
    );
    responses.insert(
        "asset-preference".into(),
        request(
            port,
            "GET",
            "/assets/app.js",
            &[("Accept-Encoding", "gzip, br")],
        )
        .await,
    );
    responses.insert("head-index".into(), request(port, "HEAD", "/", &[]).await);
    responses.insert(
        "range".into(),
        request(port, "GET", "/assets/app.js", &[("Range", "bytes=0-4")]).await,
    );
    responses.insert(
        "range-unsatisfiable".into(),
        request(port, "GET", "/assets/app.js", &[("Range", "bytes=100-")]).await,
    );
    responses.insert(
        "extensionless".into(),
        request(port, "GET", "/about", &[]).await,
    );
    responses.insert(
        "missing".into(),
        request(port, "GET", "/nope.txt", &[]).await,
    );

    // Conditional GET derived from the observed validator, exactly like a
    // browser would replay it.
    if let Some(etag) = responses
        .get("asset")
        .and_then(|response| response.get("headers"))
        .and_then(|headers| headers.get("etag"))
        .and_then(|etag| etag.as_str())
    {
        responses.insert(
            "conditional".into(),
            request(port, "GET", "/assets/app.js", &[("If-None-Match", etag)]).await,
        );
    }

    if let Some(last_modified) = responses
        .get("asset")
        .and_then(|response| response.get("headers"))
        .and_then(|headers| headers.get("last-modified"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
    {
        responses.insert(
            "conditional-since".into(),
            request(
                port,
                "GET",
                "/assets/app.js",
                &[("If-Modified-Since", &last_modified)],
            )
            .await,
        );
        responses.insert(
            "range-if-range".into(),
            request(
                port,
                "GET",
                "/assets/app.js",
                &[("Range", "bytes=0-4"), ("If-Range", &last_modified)],
            )
            .await,
        );
    }
    responses.insert(
        "range-stale-if-range".into(),
        request(
            port,
            "GET",
            "/assets/app.js",
            &[("Range", "bytes=0-4"), ("If-Range", "\"stale\"")],
        )
        .await,
    );

    println!(
        "{}",
        serde_json::to_string(&json!({ "responses": responses })).unwrap()
    );
    std::fs::remove_dir_all(&dir).ok();
}
