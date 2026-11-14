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
}
