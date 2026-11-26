//! Opt-in lazy converter startup for the real bridge lifecycle.
//!
//! The default bridge path is unchanged. When `Z2M_CONVERTER_LAZY=1` is set,
//! `install_from_env` resolves the paired devices from `database.db`, loads only
//! their converter closures, materializes an immutable runtime generation and
//! installs the dispatcher on `Zigbee` before the coordinator starts. New
//! devices are prepared later by `Zigbee::schedule_converter_pairing`.
//!
//! `Z2M_CONVERTER_TRANSPORT` selects `endpoint` (real ZCL/ZDO operations through
//! the existing Endpoint model) or `dry-run` (records frames, never sends).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use converter_artifact::{CacheConfig, FetchError, LazyConfig, ModuleFetcher, ModuleRecord};
use serde_json::Value;

use crate::converter::http::HttpsFetcher;
use crate::converter::transport::{DryRunEndpointTransport, EndpointTransport};
use crate::converter::{ConverterDispatcher, ConverterTransport, DispatcherConfig};
use crate::zigbee::Zigbee;

/// Fetcher for an artifact installed on disk (no network).
pub struct ArtifactDirFetcher {
    directory: PathBuf,
}

impl ArtifactDirFetcher {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
}

impl ModuleFetcher for ArtifactDirFetcher {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        std::fs::read(self.directory.join(&module.file)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FetchError::NotFound
            } else {
                FetchError::Network(error.to_string())
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct LazyStartupConfig {
    pub artifact_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub herdsman_dir: PathBuf,
    pub database: PathBuf,
    pub allow_network: bool,
    /// `endpoint` (real) or `dry-run` (recording).
    pub transport: String,
    /// `file` (installed artifact) or `https` (trusted manifest retrieval).
    pub fetch: String,
}

impl LazyStartupConfig {
    /// Read the opt-in configuration. `Ok(None)` means lazy mode is disabled.
    pub fn from_env(data_path: &Path) -> Result<Option<Self>, String> {
        if std::env::var("Z2M_CONVERTER_LAZY").ok().as_deref() != Some("1") {
            return Ok(None);
        }
        let transport =
            std::env::var("Z2M_CONVERTER_TRANSPORT").unwrap_or_else(|_| "dry-run".to_owned());
        if transport != "dry-run" && transport != "endpoint" {
            return Err(format!(
                "unsupported converter transport '{transport}'; use 'endpoint' or 'dry-run'"
            ));
        }
        let artifact_dir = required_path("Z2M_CONVERTER_ARTIFACT_DIR")?;
        let herdsman_dir = required_path("Z2M_CONVERTER_HERDSMAN_DIR")?;
        let cache_dir = std::env::var("Z2M_CONVERTER_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_path.join("converter-cache"));
        let runtime_dir = std::env::var("Z2M_CONVERTER_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_path.join("converter-runtime"));
        let fetch = std::env::var("Z2M_CONVERTER_FETCH").unwrap_or_else(|_| "file".to_owned());
        if fetch != "file" && fetch != "https" {
            return Err(format!(
                "unsupported converter fetch '{fetch}'; use 'file' or 'https'"
            ));
        }
        let allow_network =
            std::env::var("Z2M_CONVERTER_ALLOW_NETWORK").ok().as_deref() == Some("1");
        Ok(Some(Self {
            artifact_dir,
            cache_dir,
            runtime_dir,
            herdsman_dir,
            database: data_path.join("database.db"),
            allow_network,
            transport,
            fetch,
        }))
    }
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    std::env::var(name)
        .map(PathBuf::from)
        .map_err(|_| format!("{name} is required when Z2M_CONVERTER_LAZY=1"))
}

/// Install the lazy dispatcher on the bridge. Returns whether lazy mode was
/// enabled; an explicit error disables it instead of silently falling back.
pub async fn install_from_env(zigbee: &Arc<Zigbee>, data_path: &Path) -> Result<bool, String> {
    let Some(config) = LazyStartupConfig::from_env(data_path)? else {
        return Ok(false);
    };
    let resolver = {
        let zigbee = Arc::downgrade(zigbee);
        Arc::new(move |ieee: &str, endpoint: u8| {
            let zigbee = zigbee.upgrade()?;
            let device = zigbee.resolve_device(ieee)?;
            device.zh.get_endpoint(endpoint)
        })
    };
    let transport: Arc<dyn ConverterTransport> = match config.transport.as_str() {
        "endpoint" => Arc::new(EndpointTransport::new(resolver)),
        "dry-run" => Arc::new(DryRunEndpointTransport::new(resolver)),
        other => return Err(format!("unsupported converter transport '{other}'")),
    };
    let fetcher: Arc<dyn ModuleFetcher> = match config.fetch.as_str() {
        "file" => Arc::new(ArtifactDirFetcher::new(&config.artifact_dir)),
        "https" => {
            if !config.allow_network {
                return Err(
                    "Z2M_CONVERTER_FETCH=https requires Z2M_CONVERTER_ALLOW_NETWORK=1".to_owned(),
                );
            }
            Arc::new(
                HttpsFetcher::from_artifact_dir(&config.artifact_dir)
                    .map_err(|error| format!("converter artifact fetch: {error}"))?,
            )
        }
        other => return Err(format!("unsupported converter fetch '{other}'")),
    };
    let dispatcher = install(zigbee, &config, transport, fetcher).await?;
    zigbee.set_converter_dispatcher(Arc::clone(&dispatcher));
    Ok(true)
}

/// Install the lazy dispatcher with an explicit transport. Shared by the env
/// entry point and tests.
pub async fn install(
    zigbee: &Arc<Zigbee>,
    config: &LazyStartupConfig,
    transport: Arc<dyn ConverterTransport>,
    fetcher: Arc<dyn ModuleFetcher>,
) -> Result<Arc<ConverterDispatcher>, String> {
    let dispatcher = ConverterDispatcher::start_lazy(
        DispatcherConfig::new(&config.artifact_dir, &config.herdsman_dir),
        transport,
        LazyConfig {
            artifact_dir: config.artifact_dir.clone(),
            cache: CacheConfig::new(&config.cache_dir),
            host_abi: 1,
            engine: "quickjs-oxide".to_owned(),
            engine_version: "0.5.0".to_owned(),
        },
        config.database.clone(),
        fetcher,
        config.runtime_dir.clone(),
        config.allow_network,
    )
    .await?;
    let report = dispatcher.startup_report().unwrap_or_default();
    zigbee.logger().lock().unwrap().info(
        &format!(
            "Lazy converters ({}): {} of {} paired devices resolved, {} module(s), {} byte(s) fetched, runtime generation '{}'",
            config.transport,
            report.resolved,
            report.devices,
            report.modules_loaded.len(),
            report.network_bytes,
            dispatcher
                .runtime_root()
                .and_then(|root| root.file_name().map(|name| name.to_string_lossy().to_string()))
                .unwrap_or_default(),
        ),
        "z2m",
    );
    for (ieee, error) in &report.failed {
        zigbee
            .logger()
            .lock()
            .unwrap()
            .debug(&format!("Lazy converters: {ieee}: {error}"), "z2m");
    }
    Ok(dispatcher)
}

/// The JSON log payload used by diagnostics/tests.
#[doc(hidden)]
pub fn startup_summary(dispatcher: &ConverterDispatcher) -> Value {
    let report = dispatcher.startup_report().unwrap_or_default();
    serde_json::json!({
        "devices": report.devices,
        "resolved": report.resolved,
        "modules": report.modules_loaded,
        "networkBytes": report.network_bytes,
        "cacheBytes": report.cache_bytes,
        "failed": report.failed,
        "runtimeRoot": dispatcher.runtime_root(),
    })
}
