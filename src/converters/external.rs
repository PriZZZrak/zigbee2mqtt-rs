use std::path::{Path, PathBuf};
use tracing::debug;

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
        let file = self.dir.join(format!("{}.js", vendor_model.replace('/', "_")));
        std::fs::read_to_string(file).ok()
    }

    pub fn eval(&self, js: &str, _data: &[u8]) -> Option<serde_json::Value> {
        debug!("external eval: {} bytes from {}", js.len(), self.registry_url);
        // TODO: boa_engine Context::new, eval js — pure Rust, no C
        None
    }
}
