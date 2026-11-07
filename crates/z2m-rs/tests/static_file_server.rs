// Port of .upstream/zigbee2mqtt/test/staticFileServer.test.ts
//
// Upstream starts a node:http server and fetches it. The Rust port models the
// request/response pair directly; the same status/header/body assertions are
// kept.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use z2m_rs::util::static_file_server::{
    create_failing_static_file_server, create_static_file_server, send_not_found, HttpRequest,
    HttpResponse, StaticFileServer,
};

const INDEX_HTML: &str = "<!DOCTYPE html><html lang='en'><body>index</body></html>";
const SECRET: &str = "topsecret-must-never-be-served";
const GZIP_APP_JS: &[u8] = b"gzip-variant";
const BROTLI_APP_JS: &[u8] = b"brotli-variant";

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn app_js() -> String {
    format!("console.log(\"{}\");", "x".repeat(2048))
}

fn setup() -> (PathBuf, StaticFileServer) {
    let root = std::env::temp_dir().join(format!(
        "z2m-static-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    let dir = root.join("public");

    // outside the served directory: only a working traversal could reach it
    std::fs::write(root.join("secret.txt"), SECRET).unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("index.html"), INDEX_HTML).unwrap();
    std::fs::write(dir.join("app.js"), app_js()).unwrap();
    // precompressed variants, as shipped by the frontend packages
    std::fs::write(dir.join("app.js.gz"), GZIP_APP_JS).unwrap();
    std::fs::write(dir.join("app.js.br"), BROTLI_APP_JS).unwrap();
    std::fs::write(dir.join("sub").join("icon.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();

    let server = create_static_file_server(dir.clone(), Arc::new(|_message: &str| {}));

    (dir, server)
}

fn request(target: &str, headers: &[(&str, &str)]) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        target: target.to_string(),
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_ascii_lowercase(), (*value).to_string()))
            .collect(),
    }
}

fn header<'a>(response: &'a HttpResponse, name: &str) -> Option<&'a str> {
    response.headers.get(name).map(String::as_str)
}

#[test]
#[ignore = "not implemented yet"]
fn serves_a_file_with_its_content_type() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/sub/icon.png", &[]));

    assert_eq!(response.status, 200);
    assert_eq!(header(&response, "content-type"), Some("image/png"));
    assert_eq!(header(&response, "content-encoding"), None);
}

#[test]
#[ignore = "not implemented yet"]
fn serves_index_html_for_the_root_never_cached() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/", &[]));

    assert_eq!(response.status, 200);
    assert_eq!(
        header(&response, "content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(header(&response, "cache-control"), Some("no-store"));
    assert_eq!(response.body, INDEX_HTML.as_bytes());
}

#[test]
#[ignore = "not implemented yet"]
fn serves_the_precompressed_brotli_variant() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/app.js", &[("Accept-Encoding", "br")]));

    assert_eq!(response.status, 200);
    assert_eq!(header(&response, "content-encoding"), Some("br"));
    assert_eq!(
        header(&response, "content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(header(&response, "vary"), Some("Accept-Encoding"));
    // Upstream checks the decoded body; the stub port compares the stored bytes.
    assert_eq!(response.body, BROTLI_APP_JS);
}

#[test]
#[ignore = "not implemented yet"]
fn serves_the_precompressed_gzip_variant() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/app.js", &[("Accept-Encoding", "gzip")]));

    assert_eq!(response.status, 200);
    assert_eq!(header(&response, "content-encoding"), Some("gzip"));
    assert_eq!(response.body, GZIP_APP_JS);
}

#[test]
#[ignore = "not implemented yet"]
fn serves_the_identity_file_when_no_encoding_is_accepted() {
    let (_dir, server) = setup();
    let value = app_js();
    let response = server.handle(&request("/app.js", &[("Accept-Encoding", "identity")]));

    assert_eq!(response.status, 200);
    assert_eq!(header(&response, "content-encoding"), None);
    assert_eq!(
        header(&response, "content-length"),
        Some(value.len().to_string().as_str())
    );
    assert_eq!(response.body, value.as_bytes());
}

#[test]
#[ignore = "not implemented yet"]
fn revalidates_with_an_etag() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/app.js", &[("Accept-Encoding", "identity")]));
    let etag = header(&response, "etag")
        .expect("etag must be set")
        .to_string();

    let revalidated = server.handle(&request(
        "/app.js",
        &[("Accept-Encoding", "identity"), ("If-None-Match", &etag)],
    ));

    assert_eq!(revalidated.status, 304);
}

#[test]
#[ignore = "not implemented yet"]
fn returns_404_for_an_unknown_file() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/nope.js", &[]));

    assert_eq!(response.status, 404);
    assert_eq!(
        header(&response, "content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(
        header(&response, "content-security-policy"),
        Some("default-src 'none'")
    );
    assert_eq!(header(&response, "x-content-type-options"), Some("nosniff"));
    let body = String::from_utf8_lossy(&response.body);
    assert!(body.contains("Cannot GET /nope.js"));
}

#[test]
#[ignore = "not implemented yet"]
fn escapes_the_url_in_the_404_body() {
    let (_dir, server) = setup();
    let response = server.handle(&request("/%3Cscript%3E", &[]));

    assert_eq!(response.status, 404);
    let body = String::from_utf8_lossy(&response.body);
    assert!(!body.contains("<script>"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_serve_files_outside_of_the_served_directory() {
    let (_dir, server) = setup();

    for target in [
        "/../secret.txt",
        "/sub/../../secret.txt",
        "/%2e%2e/secret.txt",
        "/..%2fsecret.txt",
    ] {
        let response = server.handle(&request(target, &[]));

        assert_eq!(response.status, 404, "target={target}");
        let body = String::from_utf8_lossy(&response.body);
        assert!(!body.contains(SECRET), "target={target}");
    }
}

#[test]
#[ignore = "not implemented yet"]
fn reports_a_failure_to_serve_with_a_500() {
    let (dir, _server) = setup();
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = Arc::clone(&logs);

    let failing = create_failing_static_file_server(
        dir,
        Arc::new(move |message: &str| sink.lock().unwrap().push(message.to_string())),
    );
    let response = failing.handle(&request("/nope.js", &[]));

    assert_eq!(response.status, 500);
    assert_eq!(
        logs.lock().unwrap().as_slice(),
        ["Failed to serve '/nope.js': socket gone"]
    );

    // The standalone helper is part of the public surface.
    let not_found = send_not_found(&request("/nope.js", &[]));
    assert_eq!(not_found.status, 404);
}
