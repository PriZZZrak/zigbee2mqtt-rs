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
