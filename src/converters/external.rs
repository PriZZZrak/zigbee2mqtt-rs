use std::path::{Path, PathBuf};
use boa_engine::{Context, Source};
use tracing::{debug, warn};

pub struct ExternalConverter {
    dir: PathBuf,
    registry_url: String,
}

impl ExternalConverter {
    pub fn new(dir: impl AsRef<Path>, registry_url: impl Into<String>) -> Self {
        Self { dir: dir.as_ref().to_path_buf(), registry_url: registry_url.into() }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }

    pub fn load_js(&self, vendor_model: &str) -> Option<String> {
        let file = self.dir.join(format!("{}.js", vendor_model.replace('/', "_").replace(' ', "_")));
        std::fs::read_to_string(&file).ok()
    }

    pub async fn fetch_registry(&self) -> Option<serde_json::Value> {
        let path = self.registry_path();
        // Try local cache first (24h)
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(mtime) = meta.modified() {
                if mtime.elapsed().map(|d| d.as_secs() < 86400).unwrap_or(false) {
                    if let Ok(c) = std::fs::read_to_string(&path) {
                        if let Ok(v) = serde_json::from_str(&c) { return Some(v); }
                    }
                }
            }
        }
        let url = format!("{}/registry.json", self.registry_url.trim_end_matches('/'));
        // Try curl/wget/uclient-fetch without extra Rust deps (pure Rust, no ring)
        for prog in ["curl", "wget", "uclient-fetch"] {
            let out = match prog {
                "curl" => tokio::process::Command::new("curl").arg("-fsSL").arg(&url).output().await.ok(),
                "wget" => tokio::process::Command::new("wget").arg("-qO-").arg(&url).output().await.ok(),
                _ => tokio::process::Command::new("uclient-fetch").arg("-qO-").arg(&url).output().await.ok(),
            };
            if let Some(o) = out {
                if o.status.success() {
                    if let Ok(text) = String::from_utf8(o.stdout) {
                        let _ = std::fs::write(&path, &text);
                        if let Ok(v) = serde_json::from_str(&text) { return Some(v); }
                    }
                }
            }
        }
        warn!("registry fetch failed for {}", url);
        None
    }

    pub fn eval(&self, js: &str, data: &[u8]) -> Option<serde_json::Value> {
        let mut ctx = Context::default();
        let src = Source::from_bytes(js.as_bytes());
        match ctx.eval(src) {
            Ok(v) => {
                debug!("boa eval ok: {} -> {:?}", js.len(), v);
                if !data.is_empty() {
                    let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                    return Some(serde_json::json!({"raw": hex}));
                }
                None
            }
            Err(e) => {
                warn!("boa eval error from {}: {:?}", self.registry_url, e);
                None
            }
        }
    }

    pub fn registry_path(&self) -> PathBuf {
        self.dir.join("registry.json")
    }
}
