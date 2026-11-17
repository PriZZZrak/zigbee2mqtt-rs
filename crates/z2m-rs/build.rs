// Captures the Rust toolchain version so `bridge/info.os.node_version` can
// report the real runtime instead of a placeholder. Upstream reports Node's
// `process.version`; there is no Node runtime here.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|stdout| {
            let mut parts = stdout.split_whitespace();
            let name = parts.next()?.to_string();
            let version = parts.next()?.to_string();
            Some(format!("{name} {version}"))
        })
        .expect("rustc --version must be available at build time");
    println!("cargo:rustc-env=Z2M_RUSTC_VERSION={version}");

    // The bridge reports its own version and commit (upstream reads package.json
    // and git at runtime; a packaged binary has neither).
    let package_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=Z2M_VERSION={package_version}");

    let commit = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=Z2M_COMMIT={commit}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    // Upstream parity version (dev tree only; the packaged binary keeps our own).
    let upstream = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.upstream/zigbee2mqtt/package.json");
    if let Ok(contents) = std::fs::read_to_string(&upstream) {
        if let Some(version) = contents
            .split("\"version\"")
            .nth(1)
            .and_then(|rest| rest.split('"').nth(1))
        {
            println!("cargo:rustc-env=Z2M_UPSTREAM_VERSION={version}");
        }
    }
}
