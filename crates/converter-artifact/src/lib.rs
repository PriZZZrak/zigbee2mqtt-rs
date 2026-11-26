//! Fingerprint-aware converter artifact index and resolver.
//!
//! The artifact generator emits `index.z2idx1` and a compact JSON projection.
//! This crate parses both, resolves a device profile with the same semantics as
//! pinned `zigbee-herdsman-converters` (`findDefinition`/`findByDevice`) and
//! never loads a device module while resolving.

mod binary;
mod cache;
mod json;
mod lazy;
mod manifest;
mod model;
mod resolver;

pub use model::{
    ArtifactIndex, Definition, DeviceEndpoint, DeviceProfile, DeviceType, EndpointFingerprint,
    FileRecord, Fingerprint, RegexSpec, Resolution, WhiteLabel,
};
pub use resolver::normalize_model_id;

pub use binary::from_binary;
pub use cache::{
    CacheConfig, EnsureReport, FetchError, ModuleCache, ModuleFetcher, ModuleState, SpaceProbe,
    SyncFault,
};
pub use json::from_json;
pub use lazy::{
    DatabaseRecord, DeviceModule, LazyConfig, LazyLoader, StartupReport, read_database,
};
pub use manifest::{Manifest, ManifestError, ModuleRecord, SharedFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError(pub String);

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ArtifactError {}

pub type Result<T> = std::result::Result<T, ArtifactError>;
