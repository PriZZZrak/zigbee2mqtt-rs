use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use boa_engine::{Context, Source};
use serde::Deserialize;
use tracing::{debug, warn};

/// On-demand external converters (zigbee-herdsman-converters dist JS).
///
/// Upstream contract (pinned converters rev, see
/// docs/parity/evidence/CONVERTER-external.md):
/// - device files are CommonJS (`require("../converters/fromZigbee")`,
///   `require("zigbee-herdsman")`); `Fz.Converter.convert(model, msg,
///   publish, options, meta)` with
///   `msg = {cluster, type, data, linkquality, endpoint}`,
///   `meta = {state, device, deviceExposesChanged}`.
/// - definition selection: models-index `lookup` by normalized modelID,
///   single-candidate fast path, else fingerprint priority, else zigbeeModel
///   equality (whiteLabel override applied by caller).
///
/// What this implements: a minimal CJS loader (relative requires +
/// `zigbee-herdsman` stub fed with the *published* zcl.json values, never
/// hand-copied), definition selection by exact model, and real
/// `fromZigbee` dispatch for `attributeReport`/`readResponse` frames.
/// `toZigbee` (convertSet) is NOT implemented yet.
pub struct ExternalConverter {
    dir: PathBuf,
    registry_url: String,
    runtime: Arc<Mutex<Option<mpsc::Sender<ConverterJob>>>>,
}

/// One ZCL operation requested by a converter `convertSet`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvertSetOp {
    Write {
        cluster: u16,
        manufacturer_code: Option<u16>,
        disable_default_response: bool,
        attributes: Vec<(String, serde_json::Value)>,
    },
    Command {
        cluster: u16,
        command: u8,
        manufacturer_code: Option<u16>,
        disable_default_response: bool,
        payload: serde_json::Value,
    },
    Read {
        cluster: u16,
        attributes: Vec<String>,
    },
    /// Recorded so callers can log an unsupported request instead of silently
    /// claiming the converter succeeded.
    Unsupported { kind: String },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConvertSetOutcome {
    pub handled: bool,
    pub state: serde_json::Map<String, serde_json::Value>,
    pub ops: Vec<ConvertSetOp>,
}

enum ConverterJob {
    ConvertSet {
        model: String,
        endpoint: u8,
        endpoint_name: Option<String>,
        key: String,
        value: serde_json::Value,
        device: ExternalDeviceInfo,
        state: serde_json::Map<String, serde_json::Value>,
        reply: mpsc::Sender<Option<ConvertSetOutcome>>,
    },
    Convert {
        model: String,
        cluster_id: u16,
        msg_type: String,
        reports: Vec<(u16, serde_json::Value)>,
        device: ExternalDeviceInfo,
        message: IncomingMessageInfo,
        state: serde_json::Map<String, serde_json::Value>,
        reply: mpsc::Sender<Option<serde_json::Map<String, serde_json::Value>>>,
    },
    ConvertData {
        model: String,
        cluster_id: u16,
        msg_type: String,
        data: serde_json::Map<String, serde_json::Value>,
        device: ExternalDeviceInfo,
        message: IncomingMessageInfo,
        state: serde_json::Map<String, serde_json::Value>,
        reply: mpsc::Sender<Option<serde_json::Map<String, serde_json::Value>>>,
    },
}

struct ConverterSession {
    ctx: Context,
    file: String,
    index: usize,
    zcl: ZclMaps,
    types: crate::converters::zcl_types::ZclTypesFile,
}

/// Minimal device identity for the `meta.device` stub.
#[derive(Debug, Clone, Default)]
pub struct ExternalDeviceInfo {
    pub model_id: Option<String>,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub ieee_addr: String,
    pub power_source: Option<String>,
    pub device_type: Option<String>,
    pub software_build_id: Option<String>,
    pub application_version: Option<u16>,
    pub date_code: Option<String>,
    pub hardware_version: Option<u16>,
    pub stack_version: Option<u8>,
    pub zcl_version: Option<u8>,
    pub endpoint_ids: Vec<u8>,
}

impl ExternalDeviceInfo {
    /// Build the subset of device identity the bridge currently knows.
    pub fn from_device(device: &crate::devices::Device) -> Self {
        Self {
            model_id: device.model.clone(),
            manufacturer_id: device.manufacturer_id,
            manufacturer_name: device.manufacturer.clone(),
            ieee_addr: device.ieee_addr.as_hex(),
            power_source: device.power_source.clone(),
            device_type: Some(device.device_type().to_string()),
            software_build_id: device.sw_build_id.clone(),
            application_version: None,
            date_code: None,
            hardware_version: None,
            stack_version: None,
            zcl_version: None,
            endpoint_ids: device
                .endpoints
                .iter()
                .map(|endpoint| endpoint.endpoint)
                .collect(),
        }
    }
}

/// Per-message fields needed by upstream converters.
#[derive(Debug, Clone, Copy)]
pub struct IncomingMessageInfo {
    pub endpoint: u8,
    pub linkquality: u8,
    pub group_id: u16,
}

impl Default for IncomingMessageInfo {
    fn default() -> Self {
        Self {
            endpoint: 1,
            linkquality: 0,
            group_id: 0,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct RegistryFile {
    #[serde(default)]
    models: HashMap<String, RegistryModel>,
    /// Normalized Zigbee modelID -> ordered `[file, index]` candidates.
    #[serde(default)]
    lookup: HashMap<String, Vec<(String, usize)>>,
    #[serde(default)]
    fingerprints: Vec<RegistryFingerprint>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct RegistryModel {
    file: String,
    index: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryFingerprint {
    file: String,
    index: usize,
    #[serde(default)]
    fingerprint: Fingerprint,
}

/// Fingerprint fields used by the upstream selector. Unknown JSON fields are
/// ignored; fields the Rust device model cannot provide are treated as a
/// non-match, never as an implicit match.
#[derive(Debug, Clone, Default, Deserialize)]
struct Fingerprint {
    #[serde(default, rename = "modelID")]
    model_id: Option<String>,
    #[serde(default, rename = "manufacturerName")]
    manufacturer_name: Option<String>,
    #[serde(default, rename = "manufacturerID")]
    manufacturer_id: Option<u16>,
    #[serde(default, rename = "powerSource")]
    power_source: Option<String>,
    #[serde(default, rename = "softwareBuildID")]
    software_build_id: Option<String>,
    #[serde(default, rename = "applicationVersion")]
    application_version: Option<u16>,
    #[serde(default, rename = "dateCode")]
    date_code: Option<String>,
    #[serde(default, rename = "hardwareVersion")]
    hardware_version: Option<u16>,
    #[serde(default, rename = "stackVersion")]
    stack_version: Option<u8>,
    #[serde(default, rename = "zclVersion")]
    zcl_version: Option<u8>,
    #[serde(default, rename = "type")]
    device_type: Option<String>,
    /// Upstream allows a RegExp here; JSON serialisation then yields `{}`.
    /// Any presence is treated as "unsupported match" below.
    #[serde(default, rename = "ieeeAddr")]
    ieee_addr: Option<serde_json::Value>,
    #[serde(default)]
    endpoints: Option<Vec<FingerprintEndpoint>>,
    #[serde(default)]
    priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct FingerprintEndpoint {
    #[serde(rename = "ID")]
    id: u8,
}

fn normalize_model_id(model: &str) -> String {
    let end = model.find('\0').unwrap_or(model.len());
    model[..end].trim().to_string()
}

fn lookup_candidates<'a>(
    registry: &'a RegistryFile,
    model: &str,
) -> Option<&'a Vec<(String, usize)>> {
    let key = if model.trim().is_empty() {
        "null".to_string()
    } else {
        model.to_lowercase()
    };
    if let Some(candidates) = registry.lookup.get(&key) {
        return Some(candidates);
    }
    registry.lookup.get(&normalize_model_id(&key))
}

fn expected_matches<T: PartialEq>(expected: &Option<T>, actual: Option<&T>) -> bool {
    match expected {
        None => true,
        Some(expected) => actual == Some(expected),
    }
}

fn fingerprint_matches(fingerprint: &Fingerprint, device: &ExternalDeviceInfo) -> bool {
    // The upstream fingerprint allows a regex for ieeeAddr. Rust has no regex
    // dependency here, so a fingerprint that requires one is not matched rather
    // than being guessed.
    if fingerprint.ieee_addr.is_some() {
        return false;
    }
    if !expected_matches(&fingerprint.model_id, device.model_id.as_ref()) {
        return false;
    }
    if !expected_matches(
        &fingerprint.manufacturer_name,
        device.manufacturer_name.as_ref(),
    ) {
        return false;
    }
    if !expected_matches(
        &fingerprint.manufacturer_id,
        device.manufacturer_id.as_ref(),
    ) {
        return false;
    }
    if !expected_matches(&fingerprint.power_source, device.power_source.as_ref()) {
        return false;
    }
    if !expected_matches(
        &fingerprint.software_build_id,
        device.software_build_id.as_ref(),
    ) {
        return false;
    }
    if !expected_matches(
        &fingerprint.application_version,
        device.application_version.as_ref(),
    ) {
        return false;
    }
    if !expected_matches(&fingerprint.date_code, device.date_code.as_ref()) {
        return false;
    }
    if !expected_matches(
        &fingerprint.hardware_version,
        device.hardware_version.as_ref(),
    ) {
        return false;
    }
    if !expected_matches(&fingerprint.stack_version, device.stack_version.as_ref()) {
        return false;
    }
    if !expected_matches(&fingerprint.zcl_version, device.zcl_version.as_ref()) {
        return false;
    }
    if !expected_matches(&fingerprint.device_type, device.device_type.as_ref()) {
        return false;
    }
    if let Some(endpoints) = &fingerprint.endpoints {
        let expected: Vec<u8> = endpoints.iter().map(|endpoint| endpoint.id).collect();
        if expected != device.endpoint_ids {
            return false;
        }
    }
    true
}

/// Select `[file, index]` exactly as `findDefinition` does for the identity
/// fields available to the Rust runtime.
fn select_definition(
    registry: &RegistryFile,
    model: &str,
    device: &ExternalDeviceInfo,
) -> Option<(String, usize)> {
    let candidates = match lookup_candidates(registry, model) {
        Some(candidates) => candidates,
        None => {
            // Legacy registry layout (pre-lookup) keyed product model strings.
            // Only used when the whole registry has no lookup index; falling
            // back per-key would reintroduce the product-model/modelID mixup.
            if !registry.lookup.is_empty() {
                return None;
            }
            let entry = registry.models.get(&model.to_lowercase())?;
            return Some((entry.file.clone(), entry.index));
        }
    };
    if candidates.len() == 1 {
        return candidates.first().cloned();
    }
    let mut best: Option<(i32, (String, usize))> = None;
    for candidate in candidates {
        for fingerprint in &registry.fingerprints {
            if fingerprint.file != candidate.0 || fingerprint.index != candidate.1 {
                continue;
            }
            if !fingerprint_matches(&fingerprint.fingerprint, device) {
                continue;
            }
            let priority = fingerprint.fingerprint.priority.unwrap_or(0);
            if best
                .as_ref()
                .is_none_or(|(best_priority, _)| priority > *best_priority)
            {
                best = Some((priority, candidate.clone()));
            }
        }
    }
    if let Some((_, selected)) = best {
        return Some(selected);
    }
    // Upstream then compares the candidate zigbeeModel with the lookup key,
    // which cannot be re-read here. Every lookup candidate contains this key,
    // so the first candidate is the source-equivalent fallback.
    candidates.first().cloned()
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ZclMaps {
    #[serde(default, rename = "manufacturerCodes")]
    manufacturer_codes: HashMap<String, u16>,
    #[serde(default, rename = "dataTypes")]
    data_types: HashMap<String, u16>,
    #[serde(default)]
    clusters: HashMap<String, String>,
    #[serde(default)]
    attributes: HashMap<String, HashMap<String, String>>,
}

impl ExternalConverter {
    pub fn new(dir: impl AsRef<Path>, registry_url: impl Into<String>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            registry_url: registry_url.into(),
            runtime: Arc::new(Mutex::new(None)),
        }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }

    pub fn registry_path(&self) -> PathBuf {
        self.dir.join("registry.json")
    }

    /// Resolve an attribute name to its numeric ID for a cluster.
    pub fn attribute_id(&self, cluster_id: u16, name: &str) -> Option<u16> {
        let zcl = self.load_zcl()?;
        let cluster = zcl.clusters.get(&cluster_id.to_string())?;
        zcl.attributes
            .get(cluster)?
            .iter()
            .find_map(|(id, value)| (value == name).then(|| id.parse::<u16>().ok()).flatten())
    }

    /// Resolve an attribute ID to the published name for a cluster.
    pub fn attribute_name(&self, cluster_id: u16, id: u16) -> Option<String> {
        let zcl = self.load_zcl()?;
        let cluster = zcl.clusters.get(&cluster_id.to_string())?;
        zcl.attributes.get(cluster)?.get(&id.to_string()).cloned()
    }

    fn zcl_path(&self) -> PathBuf {
        self.dir.join("zcl.json")
    }

    /// Version-root URL, e.g. `.../herdsman-converters/latest`.
    /// (Older configs pointed registry_url at the registry.json file itself;
    /// both shapes are accepted.)
    fn root_url(&self) -> String {
        let u = self.registry_url.trim_end_matches('/');
        u.strip_suffix("/registry.json").unwrap_or(u).to_string()
    }

    fn load_registry(&self) -> Option<RegistryFile> {
        let text = std::fs::read_to_string(self.registry_path()).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Attribute/command type metadata published alongside `zcl.json`.
    ///
    /// A missing file yields an empty table: `convertSet` still records the
    /// symbolic operations, but encoding refuses to guess data types.
    /// Resolve the selected definition (`file`, `index`) for diagnostics and
    /// differential tooling.
    #[must_use]
    pub fn definition_for(
        &self,
        model: &str,
        device: &ExternalDeviceInfo,
    ) -> Option<(String, usize)> {
        let registry = self.load_registry()?;
        select_definition(&registry, model, device)
    }

    /// Public accessor used by the bridge to encode `convertSet` operations.
    #[must_use]
    pub fn zcl_types(&self) -> crate::converters::zcl_types::ZclTypesFile {
        self.load_zcl_types()
    }

    fn load_zcl_types(&self) -> crate::converters::zcl_types::ZclTypesFile {
        std::fs::read_to_string(self.dir.join("zcl-commands.json"))
            .ok()
            .and_then(|text| crate::converters::zcl_types::ZclTypesFile::from_json(&text).ok())
            .unwrap_or_default()
    }

    fn load_zcl(&self) -> Option<ZclMaps> {
        let text = std::fs::read_to_string(self.zcl_path()).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Resolve a registry-provided relative path inside the cache.
    ///
    /// Registry/fingerprint metadata is untrusted input: absolute paths and any
    /// `..` component must never escape the converter cache directory.
    fn cache_path(&self, file: &str) -> Option<PathBuf> {
        if file.is_empty() || file.contains('\\') {
            return None;
        }
        let relative = Path::new(file);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return None;
        }
        Some(self.dir.join(relative))
    }

    fn load_js(&self, file: &str) -> Option<String> {
        // `file` is a dist-relative path like `devices/lytko.js` or a bare
        // basename for the legacy flat layout.
        let cached = self.cache_path(file)?;
        for candidate in [cached, self.dir.join("devices").join(file)] {
            if let Ok(src) = std::fs::read_to_string(&candidate) {
                return Some(src);
            }
        }
        None
    }

    pub async fn fetch_registry(&self) -> Option<serde_json::Value> {
        let path = self.registry_path();
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(mtime) = meta.modified() {
                if mtime
                    .elapsed()
                    .map(|d| d.as_secs() < 86400)
                    .unwrap_or(false)
                {
                    if let Ok(c) = std::fs::read_to_string(&path) {
                        if let Ok(v) = serde_json::from_str(&c) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        let url = format!("{}/registry.json", self.root_url());
        let text = fetch_url(&url).await?;
        let _ = std::fs::write(&path, &text);
        serde_json::from_str(&text).ok()
    }

    pub async fn fetch_zcl(&self) -> Option<()> {
        let path = self.zcl_path();
        if path.exists() {
            return Some(());
        }
        let url = format!("{}/zcl.json", self.root_url());
        let text = fetch_url(&url).await?;
        std::fs::write(&path, text).ok()
    }

    /// Fetch a dist-relative JS file (e.g. `devices/lytko.js`) into the local
    /// mirror, preserving the dist layout so relative `require()` keeps working.
    pub async fn fetch_js(&self, file: &str) -> Option<String> {
        if let Some(cached) = self.load_js(file) {
            return Some(cached);
        }
        let dest = self.cache_path(file)?;
        let url = format!("{}/{}", self.root_url(), file.trim_start_matches('/'));
        let text = fetch_url(&url).await?;
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&dest, &text);
        Some(text)
    }

    /// Ensure the device module plus its transitive relative dependencies
    /// and zcl.json are present locally (bounded: max 80 files). Returns the
    /// dist-relative device file (e.g. `devices/lytko.js`) on success.
    pub async fn ensure_device_files(
        &self,
        model: &str,
        device: &ExternalDeviceInfo,
    ) -> Option<String> {
        let registry = self.load_registry()?;
        let (file, _) = select_definition(&registry, model, device)?;
        let mut pending = vec![format!("devices/{file}")];
        let mut seen = std::collections::HashSet::new();
        let mut fetched = 0;
        while let Some(path) = pending.pop() {
            if !seen.insert(path.clone()) {
                continue;
            }
            if fetched >= 80 {
                warn!("external converter: dependency cap reached for {model}");
                return None;
            }
            let src = match self.load_js(&path) {
                Some(s) => s,
                None => match self.fetch_js(&path).await {
                    Some(s) => {
                        fetched += 1;
                        s
                    }
                    None => {
                        warn!("external converter: cannot fetch {path}");
                        return None;
                    }
                },
            };
            for spec in required_specs(&src) {
                if spec == "zigbee-herdsman" {
                    continue;
                }
                if let Some(resolved) = resolve_relative(&path, &spec) {
                    if !seen.contains(&resolved) {
                        pending.push(resolved);
                    }
                }
            }
        }
        if self.load_zcl().is_none() {
            self.fetch_zcl().await?;
            if self.load_zcl().is_none() {
                warn!("external converter: cannot fetch zcl.json");
                return None;
            }
        }
        Some(format!("devices/{file}"))
    }

    /// Real fromZigbee dispatch for one received frame.
    ///
    /// - `model`: device model string used for registry lookup (exact,
    ///   case-insensitive). Fingerprint tie-breaking is NOT implemented yet:
    ///   see evidence CONVERTER-external.md.
    /// - `msg_type`: `"attributeReport"` (ZCL 0x0A) or `"readResponse"` (0x01).
    /// - `reports`: `(attr_id, json_value)` pairs for this frame.
    ///
    /// Returns merged state key/values, or None when nothing matched.
    pub fn convert_from_zigbee(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        reports: &[(u16, serde_json::Value)],
        device: &ExternalDeviceInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        self.convert_from_zigbee_with_message(
            model,
            cluster_id,
            msg_type,
            reports,
            device,
            &IncomingMessageInfo::default(),
            state,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn convert_with_data(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        data_obj: serde_json::Map<String, serde_json::Value>,
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let registry = self.load_registry()?;
        let (file, index) = select_definition(&registry, model, device)?;
        let zcl = self.load_zcl()?;
        let cluster_name = zcl.clusters.get(&cluster_id.to_string())?.clone();

        let module_src = self.load_js(&format!("devices/{file}")).or_else(|| {
            // Legacy flat mirror (pre-layout change).
            self.load_js(&file)
        })?;

        let mut ctx = Context::default();
        let sources = collect_sources(&self.dir, &module_src, &file, &zcl)?;
        let bootstrap = format!(
            r#"
            const __sources = {sources};
            if (__sources["__builtin__/runtime-prelude"]) new Function(__sources["__builtin__/runtime-prelude"])();
            const __modules = {{}};
            function __resolve(from, spec) {{
                if (spec === "zigbee-herdsman" || spec.startsWith("zigbee-herdsman/")) return "__builtin__/" + spec;
                if (spec.startsWith("converters/") || spec.startsWith("lib/")) {{
                    let rel = spec;
                    if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                    return rel;
                }}
                if (spec.startsWith("zigbee-herdsman-converters/")) {{
                    let rel = spec.slice("zigbee-herdsman-converters/".length);
                    if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                    return rel;
                }}
                if (!spec.startsWith(".")) return "__external__/" + spec;
                if (spec.startsWith(".")) {{
                    const parts = from.split("/").slice(0, -1).concat(spec.split("/"));
                    const out = [];
                    for (const p of parts) {{
                        if (p === "" || p === ".") continue;
                        if (p === "..") {{ out.pop(); continue; }}
                        out.push(p);
                    }}
                    let joined = out.join("/");
                    // Same rule as Rust resolve_relative: extensionless => .js
                    if (!/\.[a-z0-9]+$/i.test(joined)) joined += ".js";
                    return joined;
                }}
                return spec;
            }}
            function __require(from) {{
                return function(spec) {{
                    const key = __resolve(from, spec);
                    if (!__sources[key] && key.startsWith("__external__/")) {{
                        // Unknown npm packages are exposed as empty modules so the
                        // package still loads; using them fails at runtime.
                        __sources[key] = "module.exports = {{}};";
                    }}
                    if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                    if (!__modules[key]) {{
                        const module = {{exports: {{}}}};
                        __modules[key] = module;
                        try {{
                            const fn_ = new Function("require", "module", "exports", __sources[key]);
                            fn_(__require(key), module, module.exports);
                        }} catch (error) {{
                            throw new Error("while loading '" + key + "': " + String(error));
                        }}
                    }}
                    return __modules[key].exports;
                }};
            }}
            function __prepareDefinition(definition) {{
                if (!definition || !Array.isArray(definition.extend)) return definition;
                let meta = definition.meta ? Object.assign({{}}, definition.meta) : undefined;
                let endpoint = definition.endpoint;
                const prepared = Object.assign({{}}, definition, {{
                    fromZigbee: [...(definition.fromZigbee || [])],
                    toZigbee: [...(definition.toZigbee || [])],
                    exposes: typeof definition.exposes === "function" ? definition.exposes : [...(definition.exposes || [])],
                    options: [...(definition.options || [])],
                }});
                for (const extend of definition.extend) {{
                    if (!extend || !extend.isModernExtend) continue;
                    if (extend.fromZigbee) prepared.fromZigbee.push(...extend.fromZigbee);
                    if (extend.toZigbee) prepared.toZigbee.push(...extend.toZigbee);
                    if (Array.isArray(prepared.exposes) && extend.exposes) prepared.exposes.push(...extend.exposes);
                    if (extend.options) prepared.options.push(...extend.options);
                    // Upstream: Object.assign({{}}, ext.meta, meta) -> definition wins.
                    if (extend.meta) meta = Object.assign({{}}, extend.meta, meta);
                    if (extend.endpoint && !endpoint) endpoint = extend.endpoint;
                }}
                if (meta) prepared.meta = meta;
                if (endpoint) prepared.endpoint = endpoint;
                // Upstream prepends generic toZigbee converters before device-specific ones.
                try {{
                    const tz = __require("devices/__entry__.js")("converters/toZigbee.js");
                    const generic = [tz.scene_store, tz.scene_recall, tz.scene_add, tz.scene_remove,
                        tz.scene_remove_all, tz.scene_rename, tz.read, tz.write, tz.command,
                        tz.factory_reset, tz.zcl_command].filter(Boolean);
                    if (generic.length) prepared.toZigbee = generic.concat(prepared.toZigbee);
                }} catch (error) {{}}
                return prepared;
            }}
            "#,
            sources = serde_json::to_string(&sources).unwrap_or_default(),
        );
        if ctx.eval(Source::from_bytes(bootstrap.as_bytes())).is_err() {
            warn!("external converter: bootstrap eval failed");
            return None;
        }

        // Load the device module (its `definitions` export) lazily inside the
        // dispatch script below; here we only verify the module is loadable
        // by evaluating its source once (syntax + top-level side effects).
        let check = format!(
            r#"(function() {{
                try {{ __require("devices/__entry__.js")("./{file}"); return "ok"; }}
                catch (error) {{
                    return "ERR:" + String(error) + "\n" + String((error && error.stack) || "");
                }}
            }})()"#,
            file = file,
        );
        match eval_string(&mut ctx, &check) {
            Some(result) if result == "ok" => {}
            Some(error) => {
                warn!("external converter: cannot load {file}: {error}");
                return None;
            }
            None => {
                warn!("external converter: cannot load {file}: evaluation failed");
                return None;
            }
        }

        // Build msg + meta as JSON, then run dispatch fully inside JS so that
        // converter functions receive real objects (not strings).
        let msg = serde_json::json!({
            "cluster": cluster_name,
            "type": msg_type,
            "data": data_obj,
            "linkquality": message.linkquality,
            "groupID": message.group_id,
            "endpoint": {
                "ID": message.endpoint,
                "deviceIeeeAddress": device.ieee_addr,
            },
        });
        let meta = serde_json::json!({
            "state": state,
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
                "type": device.device_type,
                "softwareBuildID": device.software_build_id,
                "applicationVersion": device.application_version,
                "dateCode": device.date_code,
                "hardwareVersion": device.hardware_version,
                "stackVersion": device.stack_version,
                "zclVersion": device.zcl_version,
                "endpoints": device.endpoint_ids.iter().map(|id| serde_json::json!({"ID": id})).collect::<Vec<_>>(),
            },
            "options": {},
        });
        let script = format!(
            r#"(function() {{
              try {{
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const msg = JSON.parse({msg});
                const meta = JSON.parse({meta});
const __norm = (v) => {{
  if (v === null || v === undefined) return v;
  if (ArrayBuffer.isView(v) || Array.isArray(v)) return Array.from(v, __norm);
  if (typeof v === "object") {{ const o = {{}}; for (const k of Object.keys(v)) o[k] = __norm(v[k]); return o; }}
  return v;
}};
                const incomingDevice = meta.device;
                const device = globalThis.__device || (globalThis.__device = incomingDevice);
                Object.assign(device, incomingDevice);
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                meta.device = device;
                msg.endpoint.getDevice = () => device;
                msg.endpoint.constructor = {{name: "Endpoint"}};
                msg.device = device;
                device.constructor = {{name: "Device"}};
                const __debug = {debug};
                let __matched = 0;
                const __errors = [];
                const __published = [];
                const publish = (p) => {{ __published.push(p); }};
                const out = {{}};
                for (const c of (definition.fromZigbee || [])) {{
                    try {{
                        const cl = c.cluster;
                        const tys = Array.isArray(c.type) ? c.type : [c.type];
                        if ((cl === msg.cluster || String(cl) === String({cluster_id})) && tys.includes(msg.type)) {{
                            __matched += 1;
                            const r = c.convert(definition, msg, publish, {{}}, meta);
                            if (r !== undefined && r !== null) {{
                                const o = (typeof r === "object") ? r : {{}};
                                for (const k of Object.keys(o)) out[k] = o[k];
                            }}
                        }}
                    }} catch (e) {{
                        // One bad converter must not kill the frame; surfaced in debug output.
                        __errors.push(String(e && (e.stack || e.message) || e));
                    }}
                }}
                for (const p of __published) {{
                    if (p && typeof p === "object") {{
                        for (const k of Object.keys(p)) out[k] = __norm(p[k]);
                    }}
                }}
                if (__debug) out.__debug = {{matched: __matched, errors: __errors}};
                return JSON.stringify(out);
              }} catch (error) {{
                return JSON.stringify({{__fatal: String((error && (error.stack || error.message)) || error)}});
              }}
            }})()"#,
            file = file,
            index = index,
            msg = serde_json::to_string(&serde_json::to_string(&msg).unwrap()).unwrap(),
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
            cluster_id = cluster_id,
            debug = std::env::var("EVID04_DEBUG").is_ok(),
        );
        let out_str = eval_string(&mut ctx, &script)?;
        let out: serde_json::Value = serde_json::from_str(&out_str).ok()?;
        let map = out.as_object().cloned()?;
        if map.is_empty() {
            return None;
        }
        Some(map)
    }
}

const TIMERS_JS: &str = r#"
globalThis.__timers = globalThis.__timers || [];
globalThis.__timerSeq = globalThis.__timerSeq || 0;
globalThis.__now = Date.now();
globalThis.setTimeout = function(fn, delay) {
    const id = ++globalThis.__timerSeq;
    globalThis.__timers.push({id, due: globalThis.__now + (delay || 0), fn, cancelled: false});
    return id;
};
globalThis.clearTimeout = function(id) {
    for (const timer of globalThis.__timers) if (timer.id === id) timer.cancelled = true;
};
globalThis.__runDueTimers = function(now) {
    if (now > globalThis.__now) globalThis.__now = now;
    let executed = 0;
    for (const timer of globalThis.__timers) {
        if (!timer.cancelled && timer.due <= globalThis.__now) {
            timer.cancelled = true;
            try { timer.fn(); } catch (error) {}
            executed += 1;
        }
    }
    return executed;
};
globalThis.__pumpTimers = function(maxAdvance) {
    const pending = globalThis.__timers.filter((timer) => !timer.cancelled).sort((a, b) => a.due - b.due);
    if (pending.length === 0) return 0;
    const next = pending[0];
    if (next.due - globalThis.__now > maxAdvance) return 0;
    globalThis.__now = Math.max(globalThis.__now, next.due);
    return globalThis.__runDueTimers(globalThis.__now);
};
"#;

impl ExternalConverter {
    fn open_session(&self, model: &str, device: &ExternalDeviceInfo) -> Option<ConverterSession> {
        let registry = self.load_registry()?;
        let (file, index) = select_definition(&registry, model, device)?;
        let zcl = self.load_zcl()?;
        let module_src = self
            .load_js(&format!("devices/{file}"))
            .or_else(|| self.load_js(&file))?;
        let sources = collect_sources(&self.dir, &module_src, &file, &zcl)?;
        let mut ctx = Context::default();
        let bootstrap = format!("{}{}", cjs_bootstrap(&sources, &zcl), TIMERS_JS);
        if ctx.eval(Source::from_bytes(bootstrap.as_bytes())).is_err() {
            warn!("external converter: session bootstrap failed");
            return None;
        }
        let check = format!(r#"__require("devices/__entry__.js")("./{file}")"#);
        if ctx.eval(Source::from_bytes(check.as_bytes())).is_err() {
            warn!("external converter: cannot load {file}");
            return None;
        }
        Some(ConverterSession {
            ctx,
            file,
            index,
            zcl,
            types: self.load_zcl_types(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    /// Convert a frame whose payload is described as attribute reports.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_from_zigbee_with_message(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        reports: &[(u16, serde_json::Value)],
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let zcl = self.load_zcl()?;
        let cluster_name = zcl.clusters.get(&cluster_id.to_string())?.clone();
        let data_obj: serde_json::Map<String, serde_json::Value> = reports
            .iter()
            .map(|(id, value)| {
                let name = zcl
                    .attributes
                    .get(&cluster_name)
                    .and_then(|attributes| attributes.get(&id.to_string()))
                    .cloned()
                    .unwrap_or_else(|| format!("attr_{id}"));
                (name, value.clone())
            })
            .collect();
        self.convert_with_data(
            model, cluster_id, msg_type, data_obj, device, message, state,
        )
    }

    /// Convert a frame whose `msg.data` is already structured (cluster commands
    /// such as Tuya data reports, which are not attribute reports).
    #[allow(clippy::too_many_arguments)]
    pub fn convert_from_zigbee_with_data(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        self.convert_with_data(
            model,
            cluster_id,
            msg_type,
            data.clone(),
            device,
            message,
            state,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_session(
        &self,
        session: &mut ConverterSession,
        cluster_id: u16,
        msg_type: &str,
        reports: &[(u16, serde_json::Value)],
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let cluster_name = session.zcl.clusters.get(&cluster_id.to_string())?.clone();
        let data_obj: serde_json::Map<String, serde_json::Value> = reports
            .iter()
            .map(|(id, value)| {
                let name = session
                    .zcl
                    .attributes
                    .get(&cluster_name)
                    .and_then(|attributes| attributes.get(&id.to_string()))
                    .cloned()
                    .unwrap_or_else(|| format!("attr_{id}"));
                (name, value.clone())
            })
            .collect();
        self.dispatch_session_data(
            session, cluster_id, msg_type, data_obj, device, message, state,
        )
    }

    /// Dispatch a frame whose `msg.data` is already structured (cluster
    /// commands such as Tuya datapoint reports).
    #[allow(clippy::too_many_arguments)]
    fn dispatch_session_data(
        &self,
        session: &mut ConverterSession,
        cluster_id: u16,
        msg_type: &str,
        data_obj: serde_json::Map<String, serde_json::Value>,
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let cluster_name = session.zcl.clusters.get(&cluster_id.to_string())?.clone();
        let msg = serde_json::json!({
            "cluster": cluster_name,
            "type": msg_type,
            "data": data_obj,
            "linkquality": message.linkquality,
            "groupID": message.group_id,
            "endpoint": {"ID": message.endpoint, "deviceIeeeAddress": device.ieee_addr},
        });
        let meta = serde_json::json!({
            "state": state,
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
                "type": device.device_type,
                "softwareBuildID": device.software_build_id,
                "applicationVersion": device.application_version,
                "dateCode": device.date_code,
                "hardwareVersion": device.hardware_version,
                "stackVersion": device.stack_version,
                "zclVersion": device.zcl_version,
                "endpoints": device.endpoint_ids.iter().map(|id| serde_json::json!({"ID": id})).collect::<Vec<_>>(),
            },
            "options": {},
        });
        let script = format!(
            r#"(function() {{
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const msg = JSON.parse({msg});
                const meta = JSON.parse({meta});
const __norm = (v) => {{
  if (v === null || v === undefined) return v;
  if (ArrayBuffer.isView(v) || Array.isArray(v)) return Array.from(v, __norm);
  if (typeof v === "object") {{ const o = {{}}; for (const k of Object.keys(v)) o[k] = __norm(v[k]); return o; }}
  return v;
}};
                const incomingDevice = meta.device;
                const device = globalThis.__device || (globalThis.__device = incomingDevice);
                Object.assign(device, incomingDevice);
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                meta.device = device;
                msg.endpoint.getDevice = () => device;
                msg.endpoint.constructor = {{name: "Endpoint"}};
                msg.device = device;
                device.constructor = {{name: "Device"}};
                globalThis.__dispatchDone = false;
                globalThis.__dispatchError = null;
                globalThis.__dispatchOut = {{}};
                const __debug = false;
                let __matched = 0;
                const __errors = [];
                (async function() {{
                    try {{
                        const __published = [];
                        const publish = (p) => {{ __published.push(p); }};
                        const out = {{}};
                        for (const c of (definition.fromZigbee || [])) {{
                            try {{
                                const cl = c.cluster;
                                const tys = Array.isArray(c.type) ? c.type : [c.type];
                                if ((cl === msg.cluster || String(cl) === String({cluster_id})) && tys.includes(msg.type)) {{
                                    const r = await c.convert(definition, msg, publish, {{}}, meta);
                                    if (r !== undefined && r !== null) {{
                                        const o = (typeof r === "object") ? r : {{}};
                                        for (const k of Object.keys(o)) out[k] = o[k];
                                    }}
                                }}
                            }} catch (e) {{ __errors.push(String(e && (e.stack || e.message) || e)); }}
                        }}
                        for (const p of __published) {{
                            if (p && typeof p === "object") {{
                                for (const k of Object.keys(p)) out[k] = __norm(p[k]);
                            }}
                        }}
                        if (__debug) out.__debug = {{matched: __matched, errors: __errors}};
                        globalThis.__dispatchOut = out;
                    }} catch (e) {{
                        globalThis.__dispatchError = String(e);
                    }}
                    globalThis.__dispatchDone = true;
                }})();
                return "started";
            }})()"#,
            file = session.file,
            index = session.index,
            msg = serde_json::to_string(&serde_json::to_string(&msg).unwrap()).unwrap(),
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
            cluster_id = cluster_id,
        );
        let _started = eval_string(&mut session.ctx, &script)?;
        for _ in 0..1000 {
            session.ctx.run_jobs();
            let state = eval_string(
                &mut session.ctx,
                "JSON.stringify({done: !!globalThis.__dispatchDone, error: globalThis.__dispatchError || null})",
            )?;
            let state: serde_json::Value = serde_json::from_str(&state).ok()?;
            if state.get("done").and_then(|value| value.as_bool()) == Some(true) {
                if let Some(error) = state.get("error").and_then(|value| value.as_str()) {
                    warn!("external converter session failed: {error}");
                    return None;
                }
                // Service timers that became due during a synchronous converter.
                let _ = eval_string(
                    &mut session.ctx,
                    "String(globalThis.__runDueTimers(globalThis.__now))",
                );
                break;
            }
            let pumped = eval_string(&mut session.ctx, "String(globalThis.__pumpTimers(60000))")?;
            if pumped == "0" {
                warn!("external converter session waiting on an unserviceable timer");
                return None;
            }
        }
        let out_str = eval_string(&mut session.ctx, "JSON.stringify(globalThis.__dispatchOut)")?;
        let out: serde_json::Value = serde_json::from_str(&out_str).ok()?;
        let map = out.as_object().cloned()?;
        if map.is_empty() {
            return None;
        }
        Some(map)
    }

    /// Execute the upstream `toZigbee` converter for one key and record the ZCL
    /// operations it requests.
    ///
    /// The JS host resolves cluster/command names against the published ZCL
    /// metadata; unresolved names become `Unsupported` so the caller can refuse
    /// them instead of sending a guessed frame.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_session_set(
        &self,
        session: &mut ConverterSession,
        endpoint: u8,
        endpoint_name: Option<&str>,
        key: &str,
        value: &serde_json::Value,
        device: &ExternalDeviceInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<ConvertSetOutcome> {
        let meta = serde_json::json!({
            "state": state,
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
                "type": device.device_type,
                "softwareBuildID": device.software_build_id,
                "endpoints": device.endpoint_ids.iter().map(|id| serde_json::json!({"ID": id})).collect::<Vec<_>>(),
            },
            "options": {},
            "endpoint_name": endpoint_name,
            "message": {},
        });
        let cluster_names = serde_json::json!(session
            .zcl
            .clusters
            .iter()
            .filter_map(|(id, name)| name
                .is_empty()
                .then_some(())
                .map(|_| (id, name))
                .or(Some((id, name))))
            .collect::<std::collections::HashMap<_, _>>());
        let script = format!(
            r#"(function() {{
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const meta = JSON.parse({meta});
                const incomingDevice = meta.device;
                const device = globalThis.__device || (globalThis.__device = incomingDevice);
                Object.assign(device, incomingDevice);
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                meta.device = device;
                const key = JSON.parse({key});
                const value = JSON.parse({value});
const __norm = (v) => {{
  if (v === null || v === undefined) return v;
  if (ArrayBuffer.isView(v) || Array.isArray(v)) return Array.from(v, __norm);
  if (typeof v === "object") {{ const o = {{}}; for (const k of Object.keys(v)) o[k] = __norm(v[k]); return o; }}
  return v;
}};
                const endpointId = {endpoint};
                const endpointName = {endpoint_name};
                const clusterNames = {cluster_names};
                const clusterId = (cluster) => {{
                    if (cluster && cluster.ID !== undefined) return cluster.ID;
                    if (typeof cluster === "number") return cluster;
                    if (typeof cluster === "string") {{
                        if (cluster in clusterNames) return Number(cluster);
                        for (const [id, name] of Object.entries(clusterNames)) if (name === cluster) return Number(id);
                    }}
                    return null;
                }};
                const clusterLabel = (cluster) => {{
                    if (typeof cluster === "string") return cluster;
                    if (cluster && cluster.name) return cluster.name;
                    return String(cluster);
                }};
                const optionsOf = (options) => ({{
                    manufacturerCode: options && options.manufacturerCode !== undefined ? options.manufacturerCode : null,
                    disableDefaultResponse: !!(options && options.disableDefaultResponse),
                }});
                globalThis.__setOut = {{handled: false, state: {{}}, ops: [], error: null}};
                globalThis.__setDone = false;
                const out = globalThis.__setOut;
                const entity = {{
                    ID: endpointId,
                    deviceIeeeAddress: device.ieeeAddr,
                    getDevice: () => device,
                    getEndpoint: () => entity,
                    write: (cluster, attributes, options) => {{
                        out.ops.push({{op: "write", cluster: clusterLabel(cluster), cluster_id: clusterId(cluster), attributes: __norm(attributes) || {{}}, options: optionsOf(options)}});
                        return Promise.resolve();
                    }},
                    command: (cluster, command, payload, options) => {{
                        out.ops.push({{op: "command", cluster: clusterLabel(cluster), cluster_id: clusterId(cluster), command: typeof command === "string" ? command : String(command), payload: __norm(payload) || {{}}, options: optionsOf(options)}});
                        return Promise.resolve();
                    }},
                    read: (cluster, attributes) => {{
                        out.ops.push({{op: "read", cluster: clusterLabel(cluster), cluster_id: clusterId(cluster), attributes: __norm(Array.isArray(attributes) ? attributes : [attributes])}});
                        return Promise.resolve();
                    }},
                    configureReporting: () => Promise.resolve(),
                    bind: () => Promise.resolve(),
                }};
                (async function() {{
                    try {{
                        for (const converter of (definition.toZigbee || [])) {{
                            const keys = Array.isArray(converter.key) ? converter.key : [converter.key];
                            if (keys.includes(key) && typeof converter.convertSet === "function") {{
                                out.handled = true;
                                const result = await converter.convertSet(entity, key, value, meta);
                                if (result && result.state && typeof result.state === "object") {{
                                    for (const k of Object.keys(result.state)) out.state[k] = result.state[k];
                                }}
                                break;
                            }}
                        }}
                    }} catch (error) {{
                        out.error = String((error && error.stack) || error);
                    }}
                    globalThis.__setDone = true;
                }})();
                return "started";
            }})()"#,
            file = session.file,
            index = session.index,
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
            key = serde_json::to_string(&serde_json::to_string(key).unwrap()).unwrap(),
            value = serde_json::to_string(&serde_json::to_string(value).unwrap()).unwrap(),
            endpoint = endpoint,
            endpoint_name = serde_json::to_string(&endpoint_name).unwrap(),
            cluster_names = serde_json::to_string(&cluster_names).unwrap(),
        );
        let _ = eval_string(&mut session.ctx, &script)?;
        for _ in 0..1000 {
            session.ctx.run_jobs();
            let state = eval_string(
                &mut session.ctx,
                "JSON.stringify({done: !!globalThis.__setDone, error: globalThis.__setOut && globalThis.__setOut.error})",
            )?;
            let state: serde_json::Value = serde_json::from_str(&state).ok()?;
            if state.get("done").and_then(|value| value.as_bool()) == Some(true) {
                if let Some(error) = state.get("error").and_then(|value| value.as_str()) {
                    warn!("external converter convertSet failed: {error}");
                    return None;
                }
                break;
            }
            let pumped = eval_string(&mut session.ctx, "String(globalThis.__pumpTimers(60000))")?;
            if pumped == "0" {
                warn!("external converter convertSet waiting on an unserviceable timer");
                return None;
            }
        }
        let raw = eval_string(&mut session.ctx, "JSON.stringify(globalThis.__setOut)")?;
        let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let mut outcome = ConvertSetOutcome {
            handled: parsed
                .get("handled")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            state: parsed
                .get("state")
                .and_then(|value| value.as_object())
                .cloned()
                .unwrap_or_default(),
            ops: Vec::new(),
        };
        for op in parsed
            .get("ops")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let kind = op.get("op").and_then(|value| value.as_str()).unwrap_or("");
            let cluster_id = op
                .get("cluster_id")
                .and_then(|value| value.as_u64())
                .filter(|id| *id <= u16::MAX as u64)
                .map(|id| id as u16)
                .or_else(|| {
                    let name = op.get("cluster").and_then(|value| value.as_str())?;
                    session
                        .zcl
                        .clusters
                        .iter()
                        .find_map(|(id, value)| {
                            (value == name).then(|| id.parse::<u16>().ok()).flatten()
                        })
                        .or_else(|| session.types.cluster_id_by_name(name))
                });
            let Some(cluster) = cluster_id else {
                outcome.ops.push(ConvertSetOp::Unsupported {
                    kind: format!("{kind} on unknown cluster"),
                });
                continue;
            };
            let manufacturer_code = op
                .get("options")
                .and_then(|options| options.get("manufacturerCode"))
                .and_then(|value| value.as_u64())
                .filter(|code| *code <= u16::MAX as u64)
                .map(|code| code as u16);
            let disable_default_response = op
                .get("options")
                .and_then(|options| options.get("disableDefaultResponse"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            match kind {
                "write" => {
                    let attributes: Vec<(String, serde_json::Value)> = op
                        .get("attributes")
                        .and_then(|value| value.as_object())
                        .map(|object| {
                            object
                                .iter()
                                .map(|(name, value)| (name.clone(), value.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    outcome.ops.push(ConvertSetOp::Write {
                        cluster,
                        manufacturer_code,
                        disable_default_response,
                        attributes,
                    });
                }
                "command" => {
                    let command = op
                        .get("command")
                        .and_then(|value| match value {
                            serde_json::Value::Number(number) => number.as_u64(),
                            serde_json::Value::String(name) => session
                                .types
                                .command_id_by_name(cluster, name)
                                .map(u64::from),
                            _ => None,
                        })
                        .filter(|id| *id <= u8::MAX as u64);
                    let Some(command) = command else {
                        outcome.ops.push(ConvertSetOp::Unsupported {
                            kind: "command id could not be resolved".into(),
                        });
                        continue;
                    };
                    outcome.ops.push(ConvertSetOp::Command {
                        cluster,
                        command: command as u8,
                        manufacturer_code,
                        disable_default_response,
                        payload: op
                            .get("payload")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    });
                }
                "read" => {
                    let attributes: Vec<String> = op
                        .get("attributes")
                        .and_then(|value| value.as_array())
                        .map(|array| {
                            array
                                .iter()
                                .filter_map(|value| match value {
                                    serde_json::Value::String(name) => Some(name.clone()),
                                    serde_json::Value::Number(number) => Some(number.to_string()),
                                    _ => None,
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    outcome.ops.push(ConvertSetOp::Read {
                        cluster,
                        attributes,
                    });
                }
                other => outcome.ops.push(ConvertSetOp::Unsupported {
                    kind: format!("operation '{other}'"),
                }),
            }
        }
        Some(outcome)
    }

    /// Run `toZigbee.convertSet` in the persistent per-device session.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_set(
        &self,
        model: &str,
        endpoint: u8,
        endpoint_name: Option<&str>,
        key: &str,
        value: &serde_json::Value,
        device: &ExternalDeviceInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<ConvertSetOutcome> {
        let sender = self.runtime_sender()?;
        let (reply, receiver) = mpsc::channel();
        let job = ConverterJob::ConvertSet {
            model: model.to_string(),
            endpoint,
            endpoint_name: endpoint_name.map(str::to_string),
            key: key.to_string(),
            value: value.clone(),
            device: device.clone(),
            state: state.clone(),
            reply,
        };
        sender.send(job).ok()?;
        match receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(result) => result,
            Err(error) => {
                warn!("external converter convertSet timed out: {error}");
                None
            }
        }
    }

    fn runtime_sender(&self) -> Option<mpsc::Sender<ConverterJob>> {
        let mut guard = self.runtime.lock().ok()?;
        if let Some(sender) = guard.as_ref() {
            return Some(sender.clone());
        }
        let (sender, receiver) = mpsc::channel::<ConverterJob>();
        let dir = self.dir.clone();
        let registry_url = self.registry_url.clone();
        std::thread::Builder::new()
            .name("z2m-converters".into())
            .spawn(move || {
                let converter = ExternalConverter::new(dir, registry_url);
                let mut sessions: HashMap<String, ConverterSession> = HashMap::new();
                while let Ok(job) = receiver.recv() {
                    match job {
                        ConverterJob::ConvertSet {
                            model,
                            endpoint,
                            endpoint_name,
                            key,
                            value,
                            device,
                            state,
                            reply,
                        } => {
                            let session_key = format!("{}|{model}", device.ieee_addr);
                            if !sessions.contains_key(&session_key) {
                                match converter.open_session(&model, &device) {
                                    Some(session) => {
                                        sessions.insert(session_key.clone(), session);
                                    }
                                    None => {
                                        let _ = reply.send(None);
                                        continue;
                                    }
                                }
                            }
                            let result = sessions.get_mut(&session_key).and_then(|session| {
                                converter.dispatch_session_set(
                                    session,
                                    endpoint,
                                    endpoint_name.as_deref(),
                                    &key,
                                    &value,
                                    &device,
                                    &state,
                                )
                            });
                            let _ = reply.send(result);
                        }
                        ConverterJob::Convert {
                            model,
                            cluster_id,
                            msg_type,
                            reports,
                            device,
                            message,
                            state,
                            reply,
                        } => {
                            let key = format!("{}|{model}", device.ieee_addr);
                            if !sessions.contains_key(&key) {
                                match converter.open_session(&model, &device) {
                                    Some(session) => {
                                        sessions.insert(key.clone(), session);
                                    }
                                    None => {
                                        let _ = reply.send(None);
                                        continue;
                                    }
                                }
                            }
                            let result = sessions.get_mut(&key).and_then(|session| {
                                converter.dispatch_session(
                                    session, cluster_id, &msg_type, &reports, &device, &message,
                                    &state,
                                )
                            });
                            let _ = reply.send(result);
                        }
                        ConverterJob::ConvertData {
                            model,
                            cluster_id,
                            msg_type,
                            data,
                            device,
                            message,
                            state,
                            reply,
                        } => {
                            let key = format!("{}|{model}", device.ieee_addr);
                            if !sessions.contains_key(&key) {
                                match converter.open_session(&model, &device) {
                                    Some(session) => {
                                        sessions.insert(key.clone(), session);
                                    }
                                    None => {
                                        let _ = reply.send(None);
                                        continue;
                                    }
                                }
                            }
                            let result = sessions.get_mut(&key).and_then(|session| {
                                converter.dispatch_session_data(
                                    session, cluster_id, &msg_type, data, &device, &message, &state,
                                )
                            });
                            let _ = reply.send(result);
                        }
                    }
                }
            })
            .ok()?;
        *guard = Some(sender.clone());
        Some(sender)
    }

    /// Convert a frame using the persistent per-device Boa session.
    ///
    /// Module state and timers survive between frames for the same IEEE address.
    /// Falls back to the per-call context only when the runtime thread cannot be
    /// started or has stopped.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_from_zigbee_session(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        reports: &[(u16, serde_json::Value)],
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let Some(sender) = self.runtime_sender() else {
            return self.convert_from_zigbee_with_message(
                model, cluster_id, msg_type, reports, device, message, state,
            );
        };
        let (reply, receiver) = mpsc::channel();
        let job = ConverterJob::Convert {
            model: model.to_string(),
            cluster_id,
            msg_type: msg_type.to_string(),
            reports: reports.to_vec(),
            device: device.clone(),
            message: *message,
            state: state.clone(),
            reply,
        };
        if sender.send(job).is_err() {
            return self.convert_from_zigbee_with_message(
                model, cluster_id, msg_type, reports, device, message, state,
            );
        }
        match receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(result) => result,
            Err(error) => {
                warn!("external converter session timed out: {error}");
                None
            }
        }
    }

    /// Session variant for cluster commands whose `msg.data` is structured
    /// (Tuya `commandDataReport` and friends).
    #[allow(clippy::too_many_arguments)]
    pub fn convert_from_zigbee_session_with_data(
        &self,
        model: &str,
        cluster_id: u16,
        msg_type: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        device: &ExternalDeviceInfo,
        message: &IncomingMessageInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let Some(sender) = self.runtime_sender() else {
            return self.convert_from_zigbee_with_data(
                model, cluster_id, msg_type, data, device, message, state,
            );
        };
        let (reply, receiver) = mpsc::channel();
        let job = ConverterJob::ConvertData {
            model: model.to_string(),
            cluster_id,
            msg_type: msg_type.to_string(),
            data: data.clone(),
            device: device.clone(),
            message: *message,
            state: state.clone(),
            reply,
        };
        if sender.send(job).is_err() {
            return self.convert_from_zigbee_with_data(
                model, cluster_id, msg_type, data, device, message, state,
            );
        }
        match receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(result) => result,
            Err(error) => {
                warn!("external converter session timed out: {error}");
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertGetRead {
    pub cluster_id: u16,
    pub attributes: Vec<u16>,
}

impl ExternalConverter {
    /// Execute upstream `toZigbee[].convertGet` and return the reads it issued.
    ///
    /// The JS host exposes only an endpoint stub whose `read` records the
    /// requested cluster/attributes and resolves immediately. Promise jobs are
    /// drained after the call, so async convertGet functions complete. The Rust
    /// caller performs the actual reads through the ZCL transaction service.
    pub fn convert_get(
        &self,
        model: &str,
        keys: &[String],
        device: &ExternalDeviceInfo,
        state: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<ConvertGetRead> {
        let Some(registry) = self.load_registry() else {
            return Vec::new();
        };
        let Some((file, index)) = select_definition(&registry, model, device) else {
            return Vec::new();
        };
        let Some(zcl) = self.load_zcl() else {
            return Vec::new();
        };
        let Some(module_src) = self
            .load_js(&format!("devices/{file}"))
            .or_else(|| self.load_js(&file))
        else {
            return Vec::new();
        };
        let Some(sources) = collect_sources(&self.dir, &module_src, &file, &zcl) else {
            return Vec::new();
        };
        let cluster_ids: HashMap<&str, u16> = zcl
            .clusters
            .iter()
            .filter_map(|(id, name)| id.parse::<u16>().ok().map(|id| (name.as_str(), id)))
            .collect();
        let bootstrap = format!(
            r#"
            const __sources = {sources};
            if (__sources["__builtin__/runtime-prelude"]) new Function(__sources["__builtin__/runtime-prelude"])();
            const __modules = {{}};
            function __resolve(from, spec) {{
                if (spec === "zigbee-herdsman" || spec.startsWith("zigbee-herdsman/")) return "__builtin__/" + spec;
            if (spec.startsWith("converters/") || spec.startsWith("lib/")) {{
                let rel = spec;
                if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                return rel;
            }}
                if (spec.startsWith("converters/") || spec.startsWith("lib/")) {{
                    let rel = spec;
                    if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                    return rel;
                }}
                if (spec.startsWith("zigbee-herdsman-converters/")) {{
                    let rel = spec.slice("zigbee-herdsman-converters/".length);
                    if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                    return rel;
                }}
                if (!spec.startsWith(".")) return "__external__/" + spec;
                if (spec.startsWith(".")) {{
                    const parts = from.split("/").slice(0, -1).concat(spec.split("/"));
                    const out = [];
                    for (const p of parts) {{
                        if (p === "" || p === ".") continue;
                        if (p === "..") {{ out.pop(); continue; }}
                        out.push(p);
                    }}
                    let joined = out.join("/");
                    if (!/\.[a-z0-9]+$/i.test(joined)) joined += ".js";
                    return joined;
                }}
                return spec;
            }}
            function __require(from) {{
                return function(spec) {{
                    const key = __resolve(from, spec);
                    if (!__sources[key] && key.startsWith("__external__/")) {{
                        // Unknown npm packages are exposed as empty modules so the
                        // package still loads; using them fails at runtime.
                        __sources[key] = "module.exports = {{}};";
                    }}
                    if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                    if (!__modules[key]) {{
                        const module = {{exports: {{}}}};
                        __modules[key] = module;
                        try {{
                            const fn_ = new Function("require", "module", "exports", __sources[key]);
                            fn_(__require(key), module, module.exports);
                        }} catch (error) {{
                            throw new Error("while loading '" + key + "': " + String(error));
                        }}
                    }}
                    return __modules[key].exports;
                }};
            }}
            function __prepareDefinition(definition) {{
                if (!definition || !Array.isArray(definition.extend)) return definition;
                let meta = definition.meta ? Object.assign({{}}, definition.meta) : undefined;
                let endpoint = definition.endpoint;
                const prepared = Object.assign({{}}, definition, {{
                    fromZigbee: [...(definition.fromZigbee || [])],
                    toZigbee: [...(definition.toZigbee || [])],
                    exposes: [...(definition.exposes || [])],
                    options: [...(definition.options || [])],
                }});
                for (const extend of definition.extend) {{
                    if (!extend || !extend.isModernExtend) continue;
                    if (extend.fromZigbee) prepared.fromZigbee.push(...extend.fromZigbee);
                    if (extend.toZigbee) prepared.toZigbee.push(...extend.toZigbee);
                    if (extend.exposes) prepared.exposes.push(...extend.exposes);
                    if (extend.options) prepared.options.push(...extend.options);
                    // Upstream: Object.assign({{}}, ext.meta, meta) -> definition wins.
                    if (extend.meta) meta = Object.assign({{}}, extend.meta, meta);
                    if (extend.endpoint && !endpoint) endpoint = extend.endpoint;
                }}
                if (meta) prepared.meta = meta;
                if (endpoint) prepared.endpoint = endpoint;
                // Upstream prepends generic toZigbee converters before device-specific ones.
                try {{
                    const tz = __require("devices/__entry__.js")("converters/toZigbee.js");
                    const generic = [tz.scene_store, tz.scene_recall, tz.scene_add, tz.scene_remove,
                        tz.scene_remove_all, tz.scene_rename, tz.read, tz.write, tz.command,
                        tz.factory_reset, tz.zcl_command].filter(Boolean);
                    if (generic.length) prepared.toZigbee = generic.concat(prepared.toZigbee);
                }} catch (error) {{}}
                return prepared;
            }}
            "#,
            sources = serde_json::to_string(&sources).unwrap_or_default(),
        );
        let meta = serde_json::json!({
            "state": state,
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
                "type": device.device_type,
                "softwareBuildID": device.software_build_id,
                "applicationVersion": device.application_version,
                "dateCode": device.date_code,
                "hardwareVersion": device.hardware_version,
                "stackVersion": device.stack_version,
                "zclVersion": device.zcl_version,
                "endpoints": device.endpoint_ids.iter().map(|id| serde_json::json!({"ID": id})).collect::<Vec<_>>(),
            },
            "options": {},
        });
        let cluster_names = serde_json::to_string(&cluster_ids).unwrap_or_else(|_| "{}".into());
        let script = format!(
            r#"(function() {{
                const __clusterNames = {cluster_names};
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const meta = JSON.parse({meta});
                const keys = JSON.parse({keys});
                const reads = [];
                const errors = [];
                const __device = meta.device;
                __device.getEndpoint = (id) => (__device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                const entity = {{
                    ID: 1,
                    deviceIeeeAddress: __device.ieeeAddr,
                    getDevice: () => __device,
                    read: (cluster, attributes) => {{
                        const clusterId = (cluster && cluster.ID !== undefined) ? cluster.ID
                            : (typeof cluster === "number" ? cluster : __clusterNames[String(cluster)]);
                        reads.push({{cluster: clusterId, attributes: Array.isArray(attributes) ? attributes : [attributes]}});
                        return Promise.resolve();
                    }},
                    write: () => Promise.resolve(),
                    command: () => Promise.resolve(),
                    configureReporting: () => Promise.resolve(),
                    bind: () => Promise.resolve(),
                    getEndpoint: () => entity,
                }};
                for (const key of keys) {{
                    let handled = false;
                    for (const converter of (definition.toZigbee || [])) {{
                        const ckeys = Array.isArray(converter.key) ? converter.key : [converter.key];
                        if (ckeys.includes(key) && typeof converter.convertGet === "function") {{
                            handled = true;
                            Promise.resolve(converter.convertGet(entity, key, meta)).then(
                                () => {{}},
                                (error) => {{ errors.push(String(error)); }}
                            );
                            break;
                        }}
                    }}
                    if (!handled) errors.push("no convertGet for " + key);
                }}
                globalThis.__convertGet = {{reads, errors}};
            }})()"#,
            file = file,
            index = index,
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
            keys = serde_json::to_string(&serde_json::to_string(&keys).unwrap()).unwrap(),
            cluster_names = cluster_names,
        );
        let mut ctx = Context::default();
        if ctx.eval(Source::from_bytes(bootstrap.as_bytes())).is_err() {
            return Vec::new();
        }
        if let Err(error) = ctx.eval(Source::from_bytes(script.as_bytes())) {
            warn!("external converter: convertGet eval failed: {error}");
            return Vec::new();
        }
        ctx.run_jobs();
        let Some(raw) = eval_string(&mut ctx, "JSON.stringify(globalThis.__convertGet)") else {
            return Vec::new();
        };
        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };
        for error in parsed
            .get("errors")
            .and_then(|errors| errors.as_array())
            .into_iter()
            .flatten()
            .filter_map(|error| error.as_str())
        {
            debug!("external converter: convertGet: {error}");
        }
        let mut result = Vec::new();
        for read in parsed
            .get("reads")
            .and_then(|reads| reads.as_array())
            .into_iter()
            .flatten()
        {
            let Some(cluster_id) = read.get("cluster").and_then(|cluster| match cluster {
                serde_json::Value::Number(number) => number.as_u64().map(|id| id as u16),
                serde_json::Value::String(name) => cluster_name_to_id(&zcl, name),
                _ => None,
            }) else {
                continue;
            };
            let cluster_name = zcl.clusters.get(&cluster_id.to_string()).cloned();
            let mut attributes = Vec::new();
            for attribute in read
                .get("attributes")
                .and_then(|attributes| attributes.as_array())
                .into_iter()
                .flatten()
            {
                let id = match attribute {
                    serde_json::Value::Number(number) => number.as_u64().map(|id| id as u16),
                    serde_json::Value::String(name) => cluster_name
                        .as_ref()
                        .and_then(|cluster| zcl.attributes.get(cluster))
                        .and_then(|attributes| {
                            attributes.iter().find_map(|(id, value)| {
                                (value == name).then(|| id.parse::<u16>().ok()).flatten()
                            })
                        }),
                    _ => None,
                };
                if let Some(id) = id {
                    attributes.push(id);
                }
            }
            if !attributes.is_empty() {
                result.push(ConvertGetRead {
                    cluster_id,
                    attributes,
                });
            }
        }
        result
    }
}

fn cluster_name_to_id(zcl: &ZclMaps, name: &str) -> Option<u16> {
    zcl.clusters
        .iter()
        .find_map(|(id, value)| (value == name).then(|| id.parse::<u16>().ok()).flatten())
}

fn cjs_bootstrap(sources: &HashMap<String, String>, _zcl: &ZclMaps) -> String {
    format!(
        r#"
        const __sources = {sources};
        if (__sources["__builtin__/runtime-prelude"]) new Function(__sources["__builtin__/runtime-prelude"])();
        const __modules = {{}};
        function __resolve(from, spec) {{
            if (spec === "zigbee-herdsman" || spec.startsWith("zigbee-herdsman/")) return "__builtin__/" + spec;
            if (spec.startsWith("converters/") || spec.startsWith("lib/")) {{
                let rel = spec;
                if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                return rel;
            }}
            if (spec.startsWith("zigbee-herdsman-converters/")) {{
                let rel = spec.slice("zigbee-herdsman-converters/".length);
                if (!/\.[a-z0-9]+$/i.test(rel)) rel += ".js";
                return rel;
            }}
            if (!spec.startsWith(".")) return "__external__/" + spec;
            if (spec.startsWith(".")) {{
                const parts = from.split("/").slice(0, -1).concat(spec.split("/"));
                const out = [];
                for (const p of parts) {{
                    if (p === "" || p === ".") continue;
                    if (p === "..") {{ out.pop(); continue; }}
                    out.push(p);
                }}
                let joined = out.join("/");
                if (!/\.[a-z0-9]+$/i.test(joined)) joined += ".js";
                return joined;
            }}
            return spec;
        }}
        function __require(from) {{
            return function(spec) {{
                const key = __resolve(from, spec);
                if (!__sources[key] && key.startsWith("__external__/")) {{
                    __sources[key] = "module.exports = {{}};";
                }}
                if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                if (!__modules[key]) {{
                    const module = {{exports: {{}}}};
                    __modules[key] = module;
                    try {{
                        const fn_ = new Function("require", "module", "exports", __sources[key]);
                        fn_(__require(key), module, module.exports);
                    }} catch (error) {{
                        throw new Error("while loading '" + key + "': " + String(error));
                    }}
                }}
                return __modules[key].exports;
            }};
        }}
        function __prepareDefinition(definition) {{
            if (!definition || !Array.isArray(definition.extend)) return definition;
            let meta = definition.meta ? Object.assign({{}}, definition.meta) : undefined;
            let endpoint = definition.endpoint;
            const prepared = Object.assign({{}}, definition, {{
                fromZigbee: [...(definition.fromZigbee || [])],
                toZigbee: [...(definition.toZigbee || [])],
                exposes: typeof definition.exposes === "function" ? definition.exposes : [...(definition.exposes || [])],
                options: [...(definition.options || [])],
            }});
            for (const extend of definition.extend) {{
                if (!extend || !extend.isModernExtend) continue;
                if (extend.fromZigbee) prepared.fromZigbee.push(...extend.fromZigbee);
                if (extend.toZigbee) prepared.toZigbee.push(...extend.toZigbee);
                if (Array.isArray(prepared.exposes) && extend.exposes) prepared.exposes.push(...extend.exposes);
                if (extend.options) prepared.options.push(...extend.options);
                // Upstream: Object.assign({{}}, ext.meta, meta) -> definition wins.
                if (extend.meta) meta = Object.assign({{}}, extend.meta, meta);
                if (extend.endpoint && !endpoint) endpoint = extend.endpoint;
            }}
            if (meta) prepared.meta = meta;
            if (endpoint) prepared.endpoint = endpoint;
            // Upstream prepends generic toZigbee converters (scene_*, read, write,
            // command, factory_reset, zcl_command) before device-specific ones.
            try {{
                const tz = __require("devices/__entry__.js")("converters/toZigbee.js");
                const generic = [tz.scene_store, tz.scene_recall, tz.scene_add, tz.scene_remove,
                    tz.scene_remove_all, tz.scene_rename, tz.read, tz.write, tz.command,
                    tz.factory_reset, tz.zcl_command].filter(Boolean);
                if (generic.length) prepared.toZigbee = generic.concat(prepared.toZigbee);
            }} catch (error) {{}}
            return prepared;
        }}
        "#,
        sources = serde_json::to_string(sources).unwrap_or_default(),
    )
}

impl ExternalConverter {
    /// Evaluate the prepared definition's exposes as JSON.
    ///
    /// Upstream exposes can be an array or a function of the device. Function
    /// values inside expose objects are dropped during serialization.
    pub fn exposes(
        &self,
        model: &str,
        device: &ExternalDeviceInfo,
    ) -> Option<Vec<serde_json::Value>> {
        let registry = self.load_registry()?;
        let (file, index) = select_definition(&registry, model, device)?;
        let zcl = self.load_zcl()?;
        let module_src = self
            .load_js(&format!("devices/{file}"))
            .or_else(|| self.load_js(&file))?;
        let sources = collect_sources(&self.dir, &module_src, &file, &zcl)?;
        let meta = serde_json::json!({
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
                "type": device.device_type,
                "softwareBuildID": device.software_build_id,
                "endpoints": device.endpoint_ids.iter().map(|id| serde_json::json!({"ID": id})).collect::<Vec<_>>(),
            },
        });
        let script = format!(
            r#"(function() {{
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const meta = JSON.parse({meta});
                const device = meta.device;
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                let exposes = definition.exposes;
                if (typeof exposes === "function") exposes = exposes(device);
                if (!Array.isArray(exposes)) exposes = [];
                return JSON.stringify(exposes, (key, value) => typeof value === "function" ? undefined : value);
            }})()"#,
            file = file,
            index = index,
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
        );
        let mut ctx = Context::default();
        let bootstrap = cjs_bootstrap(&sources, &zcl);
        if ctx.eval(Source::from_bytes(bootstrap.as_bytes())).is_err() {
            return None;
        }
        let raw = eval_string(&mut ctx, &script)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
        parsed.as_array().cloned()
    }
}

/// Collect every CJS source reachable from the entry device file.
/// Keys are dist-relative paths (`devices/lytko.js`, `converters/foo.js`, ...).
///
/// Non-relative `require()`s (herdsman subpaths, `node:*`, npm packages) are not
/// traversed here: the JS bootstrap maps them to synthetic stubs. A missing
/// *relative* file is treated as a load failure because the package is
/// incomplete.
fn collect_sources(
    dir: &Path,
    entry_src: &str,
    entry_file: &str,
    zcl: &ZclMaps,
) -> Option<HashMap<String, String>> {
    let mut sources = HashMap::new();
    let mut stack = vec![format!("devices/{entry_file}")];
    sources.insert(format!("devices/{entry_file}"), entry_src.to_string());
    // Upstream prepends the generic toZigbee converters; load that module even
    // when the device file itself does not import it.
    let generic_to_zigbee = dir.join("converters").join("toZigbee.js");
    if generic_to_zigbee.is_file() {
        match std::fs::read_to_string(&generic_to_zigbee) {
            Ok(source) => {
                sources.insert("converters/toZigbee.js".to_string(), source);
                stack.push("converters/toZigbee.js".to_string());
            }
            Err(error) => {
                warn!("external converter: cannot read converters/toZigbee.js: {error}");
            }
        }
    }
    while let Some(path) = stack.pop() {
        let src = sources.get(&path)?.clone();
        for spec in required_specs(&src) {
            // Upstream device/lib modules sometimes require their own package
            // by name (`zigbee-herdsman-converters/lib/...`).
            let resolved = if let Some(rest) = spec.strip_prefix("zigbee-herdsman-converters/") {
                let mut joined = rest.to_string();
                if !joined.ends_with(".js") {
                    joined.push_str(".js");
                }
                joined
            } else if spec.starts_with('.') {
                match resolve_relative(&path, &spec) {
                    Some(resolved) => resolved,
                    None => {
                        warn!("external converter: cannot resolve '{spec}' from '{path}'");
                        return None;
                    }
                }
            } else {
                // Mapped to a synthetic stub in the bootstrap.
                continue;
            };
            if sources.contains_key(&resolved) {
                continue;
            }
            let local = dir.join(&resolved);
            let content = match std::fs::read_to_string(&local) {
                Ok(content) => content,
                Err(error) => {
                    warn!(
                        "external converter: missing module '{}' required by '{}': {error}",
                        resolved, path
                    );
                    return None;
                }
            };
            sources.insert(resolved.clone(), content);
            stack.push(resolved);
        }
    }
    inject_synthetic_modules(&mut sources, zcl);
    Some(sources)
}

/// Deterministic JS stubs for non-relative imports of the compiled package.
fn inject_synthetic_modules(sources: &mut HashMap<String, String>, zcl: &ZclMaps) {
    sources.insert("__builtin__/zigbee-herdsman".into(), stub_source(zcl));
    sources.insert(
        "__builtin__/zigbee-herdsman/dist/zspec/zcl/definition/enums".into(),
        enums_stub_source(zcl),
    );
    sources.insert(
        "__builtin__/zigbee-herdsman/dist/zspec/zcl".into(),
        zcl_subpath_stub_source(zcl),
    );
    sources.insert(
        "__builtin__/zigbee-herdsman/dist/zspec".into(),
        zspec_stub_source(),
    );
    sources.insert(
        "__builtin__/runtime-prelude".into(),
        RUNTIME_PRELUDE.to_string(),
    );
    sources.insert("__external__/node:assert".into(), ASSERT_STUB.to_string());
    sources.insert("__external__/node:buffer".into(), BUFFER_STUB.to_string());
    sources.insert("__external__/node:util".into(), NODE_UTIL_STUB.to_string());
    sources.insert("__external__/node:fs".into(), NODE_FS_STUB.to_string());
}

fn required_specs(src: &str) -> Vec<String> {
    // Static CJS requires only (upstream dist files use static strings).
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("require(") {
        rest = &rest[i + "require(".len()..];
        let first = rest.chars().next();
        if first == Some('"') || first == Some('\'') {
            let quote = first.unwrap();
            if let Some(end) = rest[1..].find(quote) {
                let spec = rest[1..1 + end].to_string();
                if !spec.contains('+') && !spec.contains("`") && !spec.contains("${") {
                    out.push(spec);
                }
            }
        }
    }
    out
}

fn resolve_relative(from: &str, spec: &str) -> Option<String> {
    if !spec.starts_with('.') {
        // Bare specifier other than zigbee-herdsman: resolved against dist root.
        // Upstream only uses relative requires plus zigbee-herdsman.
        return None;
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for p in spec.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    let mut joined = parts.join("/");
    if !joined.ends_with(".js") {
        joined.push_str(".js");
    }
    Some(joined)
}

/// Source of the `zigbee-herdsman` builtin stub, fed with published real
/// values. Unknown members throw loudly (never silently wrong).
fn stub_source(zcl: &ZclMaps) -> String {
    let mfr = serde_json::to_string(&zcl.manufacturer_codes).unwrap_or_else(|_| "{}".into());
    let dts = serde_json::to_string(&zcl.data_types).unwrap_or_else(|_| "{}".into());
    let clusters: HashMap<&str, u16> = zcl
        .clusters
        .iter()
        .filter_map(|(id, name)| id.parse::<u16>().ok().map(|n| (name.as_str(), n)))
        .collect();
    let cl = serde_json::to_string(&clusters).unwrap_or_else(|_| "{}".into());
    format!(
        r#"const __mfr = {mfr};
        const __dt = {dts};
        const __cl = {cl};
        const __throw = (what) => {{ throw new Error("zigbee-herdsman stub: unsupported " + what + " (see CONVERTER-external.md)"); }};
        const __status = {{SUCCESS: 0x00, FAILURE: 0x01, NOT_AUTHORIZED: 0x7E, MALFORMED_COMMAND: 0x80,
            UNSUPPORTED_ATTRIBUTE: 0x86, INVALID_VALUE: 0x87, READ_ONLY: 0x88, NOT_FOUND: 0x8B,
            INVALID_DATA_TYPE: 0x8D, WRITE_ONLY: 0x8F, TIMEOUT: 0x94, NO_IMAGE_AVAILABLE: 0x98}};
        const Zcl = {{
            ManufacturerCode: __mfr,
            DataType: __dt,
            Clusters: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                if (p in __cl) return {{ID: __cl[p]}};
                return __throw("Zcl.Clusters." + String(p));
            }} }}),
            Status: typeof __status !== "undefined" ? __status : new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.Status." + String(p));
            }} }}),
            FrameType: {{GLOBAL: 0, SPECIFIC: 1}},
            Direction: {{CLIENT_TO_SERVER: 0, SERVER_TO_CLIENT: 1}},
            StructuredIndicatorType: {{Whole: 0x00, WriteAdd: 0x10, WriteRemove: 0x20}},
            Utils: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.Utils." + String(p));
            }} }}),
            BuffaloZclDataType: {{
                USE_DATA_TYPE: 1000, LIST_UINT8: 1001, LIST_UINT16: 1002, LIST_UINT24: 1003,
                LIST_UINT32: 1004, LIST_ZONEINFO: 1005, EXTENSION_FIELD_SETS: 1006,
                LIST_THERMO_TRANSITIONS: 1007, BUFFER: 1008, GPD_FRAME: 1009,
                STRUCTURED_SELECTOR: 1010, LIST_TUYA_DATAPOINT_VALUES: 1011,
                LIST_MIBOXER_ZONES: 1012, BIG_ENDIAN_UINT24: 1013, MI_STRUCT: 1014
            }},
        }};
        const ZSpec = {{
            HA_PROFILE_ID: 260,
            HA_ENDPOINT: 1,
            BroadcastAddress: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("ZSpec.BroadcastAddress." + String(p));
            }} }}),
        }};
        module.exports = {{Zcl, ZSpec}};"#,
        mfr = mfr,
        dts = dts,
        cl = cl,
    )
}

/// `zigbee-herdsman/dist/zspec/zcl/definition/enums` replacement.
fn enums_stub_source(zcl: &ZclMaps) -> String {
    let dts = serde_json::to_string(&zcl.data_types).unwrap_or_else(|_| "{}".into());
    format!(
        r#"const __dt = {dts};
        const BuffaloZclDataType = {{
            USE_DATA_TYPE: 1000, LIST_UINT8: 1001, LIST_UINT16: 1002, LIST_UINT24: 1003,
            LIST_UINT32: 1004, LIST_ZONEINFO: 1005, EXTENSION_FIELD_SETS: 1006,
            LIST_THERMO_TRANSITIONS: 1007, BUFFER: 1008, GPD_FRAME: 1009,
            STRUCTURED_SELECTOR: 1010, LIST_TUYA_DATAPOINT_VALUES: 1011,
            LIST_MIBOXER_ZONES: 1012, BIG_ENDIAN_UINT24: 1013, MI_STRUCT: 1014
        }};
        const Direction = {{CLIENT_TO_SERVER: 0, SERVER_TO_CLIENT: 1}};
        const FrameType = {{GLOBAL: 0, SPECIFIC: 1}};
        const StructuredIndicatorType = {{Whole: 0x00, WriteAdd: 0x10, WriteRemove: 0x20}};
        const __status = {{SUCCESS: 0x00, FAILURE: 0x01, NOT_AUTHORIZED: 0x7E, MALFORMED_COMMAND: 0x80,
            UNSUPPORTED_ATTRIBUTE: 0x86, INVALID_VALUE: 0x87, READ_ONLY: 0x88, NOT_FOUND: 0x8B,
            INVALID_DATA_TYPE: 0x8D, WRITE_ONLY: 0x8F, TIMEOUT: 0x94, NO_IMAGE_AVAILABLE: 0x98}};
        const Status = new Proxy(__status, {{ get: (t, p) => {{
            if (p === "__esModule" || p === "then") return undefined;
            if (p in t) return t[p];
            throw new Error("zigbee-herdsman stub: unsupported Zcl.Status." + String(p));
        }} }});
        module.exports = {{DataType: __dt, BuffaloZclDataType, Direction, FrameType, Status, StructuredIndicatorType}};"#,
        dts = dts,
    )
}

/// `zigbee-herdsman/dist/zspec/zcl` replacement.
///
/// Real converter modules import enums and cluster metadata from this module
/// path (`zcl_1.DataType`, `zcl_1.Clusters`). Frame construction is not ported
/// and fails explicitly when used.
fn zcl_subpath_stub_source(zcl: &ZclMaps) -> String {
    let mfr = serde_json::to_string(&zcl.manufacturer_codes).unwrap_or_else(|_| "{}".into());
    let dts = serde_json::to_string(&zcl.data_types).unwrap_or_else(|_| "{}".into());
    let clusters: HashMap<&str, u16> = zcl
        .clusters
        .iter()
        .filter_map(|(id, name)| id.parse::<u16>().ok().map(|id| (name.as_str(), id)))
        .collect();
    let cl = serde_json::to_string(&clusters).unwrap_or_else(|_| "{}".into());
    format!(
        r#"const __mfr = {mfr};
        const __dt = {dts};
        const __cl = {cl};
        const __throw = (what) => {{ throw new Error("zigbee-herdsman stub: unsupported " + what); }};
        const __proxy = (label) => new Proxy({{}}, {{ get: (t, p) => {{
            if (p === "__esModule" || p === "then") return undefined;
            return __throw(label + "." + String(p));
        }} }});
        const Clusters = new Proxy({{}}, {{ get: (t, p) => {{
            if (p === "__esModule" || p === "then") return undefined;
            if (p in __cl) return {{ID: __cl[p], name: p}};
            return __throw("Zcl.Clusters." + String(p));
        }} }});
        const Status = new Proxy({{SUCCESS: 0x00, FAILURE: 0x01, NOT_AUTHORIZED: 0x7E,
            MALFORMED_COMMAND: 0x80, UNSUPPORTED_ATTRIBUTE: 0x86, INVALID_VALUE: 0x87,
            READ_ONLY: 0x88, NOT_FOUND: 0x8B, INVALID_DATA_TYPE: 0x8D, WRITE_ONLY: 0x8F,
            TIMEOUT: 0x94, NO_IMAGE_AVAILABLE: 0x98}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                if (p in t) return t[p];
                return __throw("Zcl.Status." + String(p));
            }} }});
        module.exports = {{
            Clusters, ManufacturerCode: __mfr, DataType: __dt, Status,
            DataTypeClass: {{ANALOG: "ANALOG", DISCRETE: "DISCRETE"}},
            BuffaloZclDataType: {{
                USE_DATA_TYPE: 1000, LIST_UINT8: 1001, LIST_UINT16: 1002, LIST_UINT24: 1003,
                LIST_UINT32: 1004, LIST_ZONEINFO: 1005, EXTENSION_FIELD_SETS: 1006,
                LIST_THERMO_TRANSITIONS: 1007, BUFFER: 1008, GPD_FRAME: 1009,
                STRUCTURED_SELECTOR: 1010, LIST_TUYA_DATAPOINT_VALUES: 1011,
                LIST_MIBOXER_ZONES: 1012, BIG_ENDIAN_UINT24: 1013, MI_STRUCT: 1014
            }},
            FrameType: {{GLOBAL: 0, SPECIFIC: 1}},
            Direction: {{CLIENT_TO_SERVER: 0, SERVER_TO_CLIENT: 1}},
            StructuredIndicatorType: {{Whole: 0x00, WriteAdd: 0x10, WriteRemove: 0x20}},
            Frame: __proxy("Zcl.Frame"), Header: __proxy("Zcl.Header"),
            Utils: __proxy("Zcl.Utils"), Foundation: __proxy("Zcl.Foundation"),
        }};"#,
        mfr = mfr,
        dts = dts,
        cl = cl,
    )
}

/// `zigbee-herdsman/dist/zspec` replacement.
fn zspec_stub_source() -> String {
    r#"const BuffaloZclDataType = {
        USE_DATA_TYPE: 1000, LIST_UINT8: 1001, LIST_UINT16: 1002, LIST_UINT24: 1003,
        LIST_UINT32: 1004, LIST_ZONEINFO: 1005, EXTENSION_FIELD_SETS: 1006,
        LIST_THERMO_TRANSITIONS: 1007, BUFFER: 1008, GPD_FRAME: 1009,
        STRUCTURED_SELECTOR: 1010, LIST_TUYA_DATAPOINT_VALUES: 1011,
        LIST_MIBOXER_ZONES: 1012, BIG_ENDIAN_UINT24: 1013, MI_STRUCT: 1014
    };
    module.exports = {BuffaloZclDataType, HA_PROFILE_ID: 260, HA_ENDPOINT: 1};"#
        .to_string()
}

/// `node:assert` replacement (compiled converters use `assert_1.default`).
const ASSERT_STUB: &str = r#"function __assert(value, message) {
    if (!value) {
        const error = new Error(message || "Assertion failed");
        error.name = "AssertionError";
        throw error;
    }
}
__assert.ok = __assert;
__assert.strict = __assert;
__assert.equal = (a, b, m) => __assert(a == b, m || `${a} != ${b}`);
__assert.strictEqual = (a, b, m) => __assert(a === b, m || `${a} !== ${b}`);
__assert.notStrictEqual = (a, b, m) => __assert(a !== b, m || `${a} === ${b}`);
__assert.deepStrictEqual = (a, b, m) => __assert(JSON.stringify(a) === JSON.stringify(b), m || "deepStrictEqual failed");
__assert.deepEqual = __assert.deepStrictEqual;
__assert.throws = (fn, _e, m) => { let threw = false; try { fn(); } catch { threw = true; } __assert(threw, m || "expected throw"); };
__assert.fail = (m) => __assert(false, m || "assert.fail");
module.exports = __assert;
module.exports.default = __assert;
module.exports.strict = __assert;"#;

/// Runtime globals real converter modules assume from Node.
///
/// `process` and `Buffer` are used without an explicit `require` (e.g. Tuya
/// helpers), so they must exist before any module is evaluated.
const RUNTIME_PRELUDE: &str = r#"
if (typeof globalThis.process === "undefined") {
    globalThis.process = {
        env: {}, platform: "linux", version: "v22", versions: {node: "22"},
        argv: [], pid: 1, nextTick: (fn) => { try { fn(); } catch (e) {} },
    };
}
if (typeof globalThis.__BufferShim === "undefined") {
    class __BufferShim extends Uint8Array {
        static from(value, encoding) {
            if (typeof value === "string") {
                if (encoding === "hex") {
                    const out = [];
                    for (let i = 0; i + 1 < value.length; i += 2) out.push(parseInt(value.substr(i, 2), 16));
                    return new __BufferShim(out);
                }
                return __BufferShim.from(Array.from(new TextEncoder().encode(value)));
            }
            if (Array.isArray(value)) return new __BufferShim(value);
            if (value instanceof Uint8Array) return new __BufferShim(Array.from(value));
            if (value && typeof value.length === "number") return new __BufferShim(Array.from(value));
            return new __BufferShim(0);
        }
        static alloc(size, fill = 0) { const out = new __BufferShim(size); out.fill(fill); return out; }
        static concat(list) {
            const total = list.reduce((sum, item) => sum + item.length, 0);
            const out = new __BufferShim(total);
            let offset = 0;
            for (const item of list) { out.set(item, offset); offset += item.length; }
            return out;
        }
        static isBuffer(value) { return value instanceof Uint8Array; }
        static byteLength(value) { return value.length; }
        toString(encoding) {
            if (encoding === "hex") return Array.from(this, (b) => b.toString(16).padStart(2, "0")).join("");
            if (encoding === "base64") {
                let binary = "";
                for (const b of this) binary += String.fromCharCode(b);
                return btoa(binary);
            }
            return new TextDecoder().decode(this);
        }
        writeUInt8(value, offset) { this[offset] = value & 0xff; return offset + 1; }
        writeUInt16BE(value, offset) { this[offset] = (value >> 8) & 0xff; this[offset + 1] = value & 0xff; return offset + 2; }
        writeUInt16LE(value, offset) { this[offset] = value & 0xff; this[offset + 1] = (value >> 8) & 0xff; return offset + 2; }
        writeUInt32BE(value, offset) { for (let i = 0; i < 4; i++) this[offset + i] = (value >>> (8 * (3 - i))) & 0xff; return offset + 4; }
        writeUInt32LE(value, offset) { for (let i = 0; i < 4; i++) this[offset + i] = (value >>> (8 * i)) & 0xff; return offset + 4; }
        writeInt16BE(value, offset) { return this.writeUInt16BE(value & 0xffff, offset); }
        writeInt16LE(value, offset) { return this.writeUInt16LE(value & 0xffff, offset); }
        readUInt8(offset) { return this[offset]; }
        readUInt16BE(offset) { return (this[offset] << 8) | this[offset + 1]; }
        readUInt16LE(offset) { return this[offset] | (this[offset + 1] << 8); }
        readUInt32BE(offset) { return ((this[offset] << 24) | (this[offset + 1] << 16) | (this[offset + 2] << 8) | this[offset + 3]) >>> 0; }
        readUInt32LE(offset) { return (this[offset] | (this[offset + 1] << 8) | (this[offset + 2] << 16) | (this[offset + 3] << 24)) >>> 0; }
        readInt16BE(offset) { const v = this.readUInt16BE(offset); return v & 0x8000 ? v - 0x10000 : v; }
        readInt16LE(offset) { const v = this.readUInt16LE(offset); return v & 0x8000 ? v - 0x10000 : v; }
        write(text, offset = 0, length, encoding) {
            const bytes = encoding === "hex"
                ? __BufferShim.from(text, "hex")
                : __BufferShim.from(Array.from(new TextEncoder().encode(String(text))));
            const size = Math.min(length === undefined ? bytes.length : length, bytes.length);
            for (let i = 0; i < size; i++) this[offset + i] = bytes[i];
            return size;
        }
        copy(target, targetStart = 0, sourceStart = 0, sourceEnd = this.length) {
            for (let i = sourceStart; i < sourceEnd; i++) target[targetStart + (i - sourceStart)] = this[i];
            return sourceEnd - sourceStart;
        }
        equals(other) {
            if (!other || other.length !== this.length) return false;
            for (let i = 0; i < this.length; i++) if (this[i] !== other[i]) return false;
            return true;
        }
        compare(other) {
            for (let i = 0; i < Math.min(this.length, other.length); i++) {
                if (this[i] !== other[i]) return this[i] - other[i];
            }
            return this.length - other.length;
        }
    }
    globalThis.__BufferShim = __BufferShim;
}
if (typeof globalThis.Buffer === "undefined") globalThis.Buffer = globalThis.__BufferShim;
"#;

/// `node:buffer` module: the shim installed by the runtime prelude.
const BUFFER_STUB: &str =
    r#"module.exports = {Buffer: globalThis.__BufferShim, default: globalThis.__BufferShim};"#;

const NODE_UTIL_STUB: &str = r#"module.exports = {
    inspect: (value) => { try { return JSON.stringify(value); } catch { return String(value); } },
    format: (...args) => args.map((value) => String(value)).join(" "),
    inherits: (ctor, superCtor) => { Object.setPrototypeOf(ctor.prototype, superCtor.prototype); },
    promisify: (fn) => fn,
};"#;

/// `node:fs` is imported by some vendor libraries; file I/O is not exposed to
/// converters, so reads fail explicitly and existence checks are false.
const NODE_FS_STUB: &str = r#"module.exports = {
    existsSync: () => false,
    readFileSync: () => { throw new Error("node:fs is not available to converters"); },
    writeFileSync: () => { throw new Error("node:fs is not available to converters"); },
    mkdirSync: () => {},
    readdirSync: () => [],
};"#;

fn eval_string(ctx: &mut Context, script: &str) -> Option<String> {
    let v = ctx.eval(Source::from_bytes(script.as_bytes())).ok()?;
    v.as_string().map(|s| s.to_std_string_escaped())
}

const MAX_FETCH_BYTES: usize = 8 * 1024 * 1024;

async fn fetch_url(url: &str) -> Option<String> {
    for prog in ["curl", "wget", "uclient-fetch"] {
        let out = match prog {
            "curl" => tokio::process::Command::new("curl")
                .arg("-fsSL")
                .arg(url)
                .output()
                .await
                .ok(),
            "wget" => tokio::process::Command::new("wget")
                .arg("-qO-")
                .arg(url)
                .output()
                .await
                .ok(),
            _ => tokio::process::Command::new("uclient-fetch")
                .arg("-qO-")
                .arg(url)
                .output()
                .await
                .ok(),
        };
        if let Some(o) = out {
            if o.status.success() {
                if o.stdout.len() > MAX_FETCH_BYTES {
                    warn!(
                        "refusing oversized download from {url}: {} bytes",
                        o.stdout.len()
                    );
                    return None;
                }
                if let Ok(text) = String::from_utf8(o.stdout) {
                    return Some(text);
                }
            }
        }
    }
    warn!("fetch failed for {}", url);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Minimal upstream-shaped fixture: CJS device file using a relative
    /// helper (exercises the module loader) plus zigbee-herdsman constants.
    const DEVICE_JS: &str = r#"
const {Zcl} = require("zigbee-herdsman");
const {double} = require("./helpers/util");
const {check} = require("./helpers/assert_helper");
const {uint16, listUint8} = require("./helpers/enums_helper");
const MANU = Zcl.ManufacturerCode.TEXAS_INSTRUMENTS;
const definitions = [
    {
        model: "FixtureBulb",
        vendor: "Fixture",
        description: "test fixture",
        exposes: [
            {type: "numeric", name: "temperature", property: "temperature", access: 1, unit: "°C"},
        ],
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport", "readResponse"],
                convert: (model, msg, publish, options, meta) => {
                    if (msg.data.measuredValue === undefined) return;
                    if (MANU !== 7) throw new Error("bad manufacturer code, got " + MANU);
                    return {
                        temperature: double(msg.data.measuredValue) / 100.0,
                        stub_uint16: uint16,
                        stub_list_uint8: listUint8,
                        checked: check(msg.data.measuredValue),
                    };
                },
            },
        ],
        toZigbee: [
            {
                key: ["temperature"],
                convertGet: async (entity, key, meta) => {
                    await entity.read("msTemperatureMeasurement", ["measuredValue"]);
                },
            },
            {
                key: ["device_info"],
                convertGet: async (entity, key, meta) => {
                    const attribute = entity.getDevice().modelID === "FixtureBulb" ? "measuredValue" : "unknown";
                    await entity.read("msTemperatureMeasurement", [attribute]);
                },
            },
            {
                key: ["window_detection"],
                convertSet: async (entity, key, value, meta) => {
                    await entity.write("genOnOff", {onOff: value === "ON" ? 1 : 0});
                    return {state: {[key]: value}};
                },
            },
            {
                key: ["on_timed"],
                convertSet: async (entity, key, value, meta) => {
                    await entity.command("genOnOff", "onWithTimedOff", {ctrlbits: 0, ontime: 10, offwaittime: 5});
                    return {state: {[key]: value}};
                },
            },
            {
                key: ["vendor_flag"],
                convertSet: async (entity, key, value, meta) => {
                    await entity.write("manuSpecificUnknown", {flag: 1});
                    return {state: {[key]: value}};
                },
            },
        ],
    },
];
module.exports = {definitions};
"#;

    const HELPER_JS: &str = r#"
function double(x) { return x * 2; }
module.exports = {double};
"#;

    const ASSERT_HELPER_JS: &str = r#"
const assert = require("node:assert");
function check(value) {
    assert(value !== undefined, "value required");
    assert.strictEqual(typeof value, "number");
    return value;
}
module.exports = {check};
"#;

    const ENUMS_HELPER_JS: &str = r#"
const enums = require("zigbee-herdsman/dist/zspec/zcl/definition/enums");
module.exports = {uint16: enums.DataType.UINT16, listUint8: enums.BuffaloZclDataType.LIST_UINT8};
"#;

    struct FixtureDir(PathBuf);
    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_dir() -> (FixtureDir, ExternalConverter) {
        static CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("z2m-extconv-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let dev_dir = dir.join("devices");
        std::fs::create_dir_all(dev_dir.join("helpers")).unwrap();
        let mut f = std::fs::File::create(dev_dir.join("fakedev.js")).unwrap();
        f.write_all(DEVICE_JS.as_bytes()).unwrap();
        let mut f = std::fs::File::create(dev_dir.join("helpers").join("util.js")).unwrap();
        f.write_all(HELPER_JS.as_bytes()).unwrap();
        let mut f =
            std::fs::File::create(dev_dir.join("helpers").join("assert_helper.js")).unwrap();
        f.write_all(ASSERT_HELPER_JS.as_bytes()).unwrap();
        let mut f = std::fs::File::create(dev_dir.join("helpers").join("enums_helper.js")).unwrap();
        f.write_all(ENUMS_HELPER_JS.as_bytes()).unwrap();
        let registry = serde_json::json!({
            "models": {
                "fixturebulb": {"file": "fakedev.js", "index": 0, "vendor": "Fixture", "model": "FixtureBulb"},
            },
            "lookup": {
                "fixturebulb": [["fakedev.js", 0]],
                "null": [["fakedev.js", 0]],
            },
            "fingerprints": [],
        });
        std::fs::write(
            dir.join("zcl-commands.json"),
            serde_json::json!({
                "clusters": {
                    "6": {
                        "name": "genOnOff",
                        "attributes": {"0": {"name": "onOff", "type": 16}},
                        "commands": {
                            "66": {"name": "onWithTimedOff", "parameters": [
                                {"name": "ctrlbits", "type": 32},
                                {"name": "ontime", "type": 33},
                                {"name": "offwaittime", "type": 33}
                            ]}
                        }
                    },
                    "1026": {
                        "name": "msTemperatureMeasurement",
                        "attributes": {"0": {"name": "measuredValue", "type": 41}},
                        "commands": {}
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join("registry.json"),
            serde_json::to_string(&registry).unwrap(),
        )
        .unwrap();
        let zcl = serde_json::json!({
            "manufacturerCodes": {"TEXAS_INSTRUMENTS": 7},
            "dataTypes": {"UINT16": 33},
            "clusters": {"1026": "msTemperatureMeasurement", "6": "genOnOff"},
            "attributes": {"msTemperatureMeasurement": {"0": "measuredValue"}, "genOnOff": {"0": "onOff"}},
        });
        std::fs::write(dir.join("zcl.json"), serde_json::to_string(&zcl).unwrap()).unwrap();
        let conv = ExternalConverter::new(&dir, "https://example.invalid/x");
        (FixtureDir(dir), conv)
    }

    fn device() -> ExternalDeviceInfo {
        ExternalDeviceInfo {
            model_id: Some("FixtureBulb".into()),
            manufacturer_id: Some(7),
            manufacturer_name: Some("Fixture".into()),
            ieee_addr: "0x0011223344556677".into(),
            power_source: Some("Mains (single phase)".into()),
            device_type: Some("Router".into()),
            endpoint_ids: vec![1],
            ..Default::default()
        }
    }

    #[test]
    fn dispatch_calls_matching_converter() {
        let (_tmp, conv) = fixture_dir();
        let out = conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2250))],
                &device(),
                &Default::default(),
            )
            .expect("converter should match");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(45)));
    }

    #[test]
    fn real_package_modules_and_enums_subpath_are_stubbed() {
        let (_tmp, conv) = fixture_dir();
        let out = conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("conversion with node:assert and enums stub");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));
        assert_eq!(out.get("stub_uint16"), Some(&serde_json::json!(33)));
        assert_eq!(out.get("stub_list_uint8"), Some(&serde_json::json!(1001)));
        assert_eq!(out.get("checked"), Some(&serde_json::json!(2000)));
    }

    #[test]
    fn json_serialized_regex_fingerprint_does_not_break_registry_loading() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("otherdev.js"),
            OTHER_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"fixturebulb": [["fakedev.js", 0], ["otherdev.js", 0]]},
                "fingerprints": [
                    {"file": "fakedev.js", "index": 0, "fingerprint": {"ieeeAddr": {}, "priority": 100}},
                ],
            }),
        );
        // A RegExp fingerprint cannot be evaluated in Rust; it must not match and
        // must not abort selection (first candidate is used).
        let out = conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("selection must fall back to the first candidate");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        let (_tmp, conv) = fixture_dir();
        let out = conv
            .convert_from_zigbee(
                "fixturebulb",
                1026,
                "readResponse",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("converter should match");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));
    }

    #[test]
    fn unknown_cluster_returns_none() {
        let (_tmp, conv) = fixture_dir();
        let out = conv.convert_from_zigbee(
            "FixtureBulb",
            6,
            "attributeReport",
            &[(0, serde_json::json!(1))],
            &device(),
            &Default::default(),
        );
        assert!(out.is_none());
    }

    #[test]
    fn unknown_model_returns_none() {
        let (_tmp, conv) = fixture_dir();
        assert!(conv
            .convert_from_zigbee(
                "Nope",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(1))],
                &device(),
                &Default::default()
            )
            .is_none());
    }

    #[test]
    fn typescript_source_is_rejected_not_executed() {
        let (_tmp, conv) = fixture_dir();
        let ts = "import type {DefinitionWithExtend} from '../lib/types';\nexport const x = 1;\n";
        std::fs::write(conv.dir.join("devices").join("badts.js"), ts).unwrap();
        // Direct eval of TS must fail loudly instead of producing a value.
        let mut ctx = Context::default();
        assert!(ctx.eval(Source::from_bytes(ts.as_bytes())).is_err());
    }

    const OTHER_DEVICE_JS: &str = r#"
const definitions = [
    {
        model: "OtherBulb",
        vendor: "Fixture",
        description: "second fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport", "readResponse"],
                convert: () => ({temperature: 999}),
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    fn write_registry(conv: &ExternalConverter, value: serde_json::Value) {
        std::fs::write(
            conv.dir.join("registry.json"),
            serde_json::to_string(&value).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn fingerprint_priority_selects_the_matching_definition() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("otherdev.js"),
            OTHER_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"fixturebulb": [["fakedev.js", 0], ["otherdev.js", 0]]},
                "fingerprints": [
                    {"file": "fakedev.js", "index": 0, "fingerprint": {"manufacturerName": "Fixture", "priority": 0}},
                    {"file": "otherdev.js", "index": 0, "fingerprint": {"manufacturerName": "Fixture", "modelID": "FixtureBulb", "priority": 10}},
                ],
            }),
        );
        let out = conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("priority candidate should match");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(999)));
    }

    #[test]
    fn unmatched_fingerprint_falls_back_to_first_candidate() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("otherdev.js"),
            OTHER_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"fixturebulb": [["fakedev.js", 0], ["otherdev.js", 0]]},
                "fingerprints": [
                    {"file": "otherdev.js", "index": 0, "fingerprint": {"manufacturerName": "Another Vendor", "priority": 100}},
                ],
            }),
        );
        let out = conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("first candidate should match");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));
    }

    #[test]
    fn empty_model_uses_null_lookup_bucket_and_nul_suffix_is_trimmed() {
        let (_tmp, conv) = fixture_dir();
        let out = conv
            .convert_from_zigbee(
                "",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("null bucket should match");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));

        let out = conv
            .convert_from_zigbee(
                "FixtureBulb\0garbage",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("normalized model should match lookup");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(40)));
    }

    #[test]
    fn product_model_map_is_not_used_when_lookup_index_exists() {
        let (_tmp, conv) = fixture_dir();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"some-other-model": [["fakedev.js", 0]]},
                "models": {"fixturebulb": {"file": "fakedev.js", "index": 0}},
            }),
        );
        assert!(conv
            .convert_from_zigbee(
                "FixtureBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .is_none());
    }

    #[tokio::test]
    async fn ensure_device_files_resolves_by_lookup_and_reports_misses() {
        let (_tmp, conv) = fixture_dir();
        assert_eq!(
            conv.ensure_device_files("FixtureBulb", &device())
                .await
                .as_deref(),
            Some("devices/fakedev.js")
        );
        assert!(conv.ensure_device_files("Nope", &device()).await.is_none());
    }

    #[test]
    fn collect_sources_follows_relative_requires() {
        let (_tmp, conv) = fixture_dir();
        let src = std::fs::read_to_string(conv.dir.join("devices").join("fakedev.js")).unwrap();
        let zcl = conv.load_zcl().expect("zcl");
        let map = collect_sources(conv.dir.as_path(), &src, "fakedev.js", &zcl).expect("collect");
        assert!(map.contains_key("devices/fakedev.js"));
        assert!(map.contains_key("devices/helpers/util.js"));
    }

    #[test]
    fn cache_path_rejects_traversal_and_absolute_paths() {
        let (_tmp, conv) = fixture_dir();
        assert!(conv.cache_path("../evil.js").is_none());
        assert!(conv.cache_path("devices/../../evil.js").is_none());
        assert!(conv.cache_path("/etc/passwd").is_none());
        assert!(conv.cache_path(r"devices\evil.js").is_none());
        assert!(conv.cache_path("").is_none());
        let valid = conv.cache_path("devices/fakedev.js").expect("valid path");
        assert!(valid.starts_with(conv.dir.as_path()));
        assert!(conv.load_js("../outside").is_none());
    }

    #[tokio::test]
    async fn fetch_js_rejects_traversal_before_network() {
        let (_tmp, conv) = fixture_dir();
        assert!(conv.fetch_js("../../etc/passwd").await.is_none());
    }

    #[test]
    fn async_convert_get_records_resolved_reads() {
        let (_tmp, conv) = fixture_dir();
        let reads = conv.convert_get(
            "FixtureBulb",
            &["temperature".to_string()],
            &device(),
            &Default::default(),
        );
        assert_eq!(
            reads,
            vec![ConvertGetRead {
                cluster_id: 1026,
                attributes: vec![0],
            }]
        );
    }

    #[test]
    fn convert_get_entity_exposes_device_identity() {
        let (_tmp, conv) = fixture_dir();
        let reads = conv.convert_get(
            "FixtureBulb",
            &["device_info".to_string()],
            &device(),
            &Default::default(),
        );
        assert_eq!(
            reads,
            vec![ConvertGetRead {
                cluster_id: 1026,
                attributes: vec![0],
            }]
        );
    }

    #[test]
    fn missing_convert_get_returns_no_reads() {
        let (_tmp, conv) = fixture_dir();
        assert!(conv
            .convert_get(
                "FixtureBulb",
                &["unknown_key".to_string()],
                &device(),
                &Default::default(),
            )
            .is_empty());
    }

    const EXTEND_DEVICE_JS: &str = r#"
const definitions = [
    {
        model: "ExtendBulb",
        vendor: "Fixture",
        description: "extend-only fixture",
        extend: [
            {
                isModernExtend: true,
                fromZigbee: [
                    {
                        cluster: "msTemperatureMeasurement",
                        type: ["attributeReport"],
                        convert: (model, msg) => ({temperature: msg.data.measuredValue / 100}),
                    },
                ],
                exposes: [],
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    const META_DEVICE_JS: &str = r#"
const definitions = [
    {
        model: "MetaBulb",
        vendor: "Fixture",
        description: "message metadata fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport"],
                convert: (model, msg, publish, options, meta) => ({
                    endpoint_seen: msg.endpoint.ID,
                    lqi_seen: msg.linkquality,
                    device_model: msg.endpoint.getDevice().modelID,
                    ieee_seen: msg.endpoint.deviceIeeeAddress,
                    endpoint_count: meta.device.endpoints.length,
                }),
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn extend_only_definition_is_prepared_before_dispatch() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("extenddev.js"),
            EXTEND_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"extendbulb": [["extenddev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let out = conv
            .convert_from_zigbee(
                "ExtendBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &Default::default(),
            )
            .expect("modern extend should be prepared");
        assert_eq!(out.get("temperature"), Some(&serde_json::json!(20)));
    }

    #[test]
    fn message_endpoint_and_linkquality_reach_the_converter() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(conv.dir.join("devices").join("metadev.js"), META_DEVICE_JS).unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"metabulb": [["metadev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let out = conv
            .convert_from_zigbee_with_message(
                "MetaBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(2000))],
                &device(),
                &IncomingMessageInfo {
                    endpoint: 3,
                    linkquality: 77,
                    group_id: 0,
                },
                &Default::default(),
            )
            .expect("metadata converter should match");
        assert_eq!(out.get("endpoint_seen"), Some(&serde_json::json!(3)));
        assert_eq!(out.get("lqi_seen"), Some(&serde_json::json!(77)));
        assert_eq!(
            out.get("device_model"),
            Some(&serde_json::json!("FixtureBulb"))
        );
        assert_eq!(
            out.get("ieee_seen"),
            Some(&serde_json::json!("0x0011223344556677"))
        );
        assert_eq!(out.get("endpoint_count"), Some(&serde_json::json!(1)));
    }

    const EXPOSE_DEVICE_JS: &str = r#"
const definitions = [
    {
        model: "ExposeBulb",
        vendor: "Fixture",
        description: "function exposes fixture",
        exposes: (device) => [
            {type: "numeric", name: device.modelID === "FixtureBulb" ? "temperature" : "other", property: "temperature", access: 1},
        ],
        fromZigbee: [],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn exposes_array_is_returned_as_json() {
        let (_tmp, conv) = fixture_dir();
        let exposes = conv.exposes("FixtureBulb", &device()).expect("exposes");
        assert_eq!(exposes.len(), 1);
        assert_eq!(exposes[0]["name"], "temperature");
        assert_eq!(exposes[0]["unit"], "°C");
    }

    #[test]
    fn exposes_function_receives_the_device() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("exposedev.js"),
            EXPOSE_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"exposebulb": [["exposedev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let exposes = conv.exposes("ExposeBulb", &device()).expect("exposes");
        assert_eq!(exposes[0]["name"], "temperature");
    }

    const TIMER_DEVICE_JS: &str = r#"
let frames = 0;
let timerFired = 0;
const definitions = [
    {
        model: "TimerBulb",
        vendor: "Fixture",
        description: "session state fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport"],
                convert: () => {
                    frames += 1;
                    if (frames === 1) setTimeout(() => { timerFired += 1; }, 0);
                    return {frames, timerFired};
                },
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn session_keeps_module_state_and_runs_due_timers() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("timerdev.js"),
            TIMER_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"timerbulb": [["timerdev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let mut timer_device = device();
        timer_device.ieee_addr = "0x00000000000000aa".into();
        let first = conv
            .convert_from_zigbee_session(
                "TimerBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(100))],
                &timer_device,
                &IncomingMessageInfo::default(),
                &Default::default(),
            )
            .expect("first frame");
        assert_eq!(first.get("frames"), Some(&serde_json::json!(1)));
        assert_eq!(first.get("timerFired"), Some(&serde_json::json!(0)));

        let second = conv
            .convert_from_zigbee_session(
                "TimerBulb",
                1026,
                "attributeReport",
                &[(0, serde_json::json!(110))],
                &timer_device,
                &IncomingMessageInfo::default(),
                &Default::default(),
            )
            .expect("second frame");
        assert_eq!(second.get("frames"), Some(&serde_json::json!(2)));
        assert_eq!(
            second.get("timerFired"),
            Some(&serde_json::json!(1)),
            "setTimeout callback must run and mutate module state"
        );
    }

    const STORE_DEVICE_JS: &str = r#"
const deviceStore = new Map();
const definitions = [
    {
        model: "StoreBulb",
        vendor: "Fixture",
        description: "shared store fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport"],
                convert: (model, msg, publish, options, meta) => {
                    const count = (deviceStore.get(meta.device) || 0) + 1;
                    deviceStore.set(meta.device, count);
                    return {store_count: count};
                },
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn session_keeps_device_keyed_module_store() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("storedev.js"),
            STORE_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"storebulb": [["storedev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let mut store_device = device();
        store_device.ieee_addr = "0x00000000000000bb".into();
        for expected in [1, 2, 3] {
            let out = conv
                .convert_from_zigbee_session(
                    "StoreBulb",
                    1026,
                    "attributeReport",
                    &[(0, serde_json::json!(100))],
                    &store_device,
                    &IncomingMessageInfo::default(),
                    &Default::default(),
                )
                .expect("store frame");
            assert_eq!(out.get("store_count"), Some(&serde_json::json!(expected)));
        }
    }

    const ASYNC_DEVICE_JS: &str = r#"
let frames = 0;
const definitions = [
    {
        model: "AsyncBulb",
        vendor: "Fixture",
        description: "async fz fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport"],
                convert: async () => {
                    await Promise.resolve();
                    frames += 1;
                    return {async_frames: frames};
                },
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    const FUTURE_TIMER_DEVICE_JS: &str = r#"
let fired = 0;
const definitions = [
    {
        model: "FutureTimerBulb",
        vendor: "Fixture",
        description: "future timer fixture",
        fromZigbee: [
            {
                cluster: "msTemperatureMeasurement",
                type: ["attributeReport"],
                convert: async () => {
                    await new Promise((resolve) => setTimeout(resolve, 1000));
                    fired += 1;
                    return {fired};
                },
            },
        ],
        toZigbee: [],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn async_from_zigbee_promises_are_awaited_across_frames() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("asyncdev.js"),
            ASYNC_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"asyncbulb": [["asyncdev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let mut async_device = device();
        async_device.ieee_addr = "0x00000000000000cc".into();
        for expected in [1, 2] {
            let out = conv
                .convert_from_zigbee_session(
                    "AsyncBulb",
                    1026,
                    "attributeReport",
                    &[(0, serde_json::json!(100))],
                    &async_device,
                    &IncomingMessageInfo::default(),
                    &Default::default(),
                )
                .expect("async frame");
            assert_eq!(out.get("async_frames"), Some(&serde_json::json!(expected)));
        }
    }

    #[test]
    fn future_timer_is_pumped_and_resolves_async_converter() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("futuretimedev.js"),
            FUTURE_TIMER_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"futuretimerbulb": [["futuretimedev.js", 0]]},
                "fingerprints": [],
            }),
        );
        let mut timer_device = device();
        timer_device.ieee_addr = "0x00000000000000dd".into();
        for expected in [1, 2] {
            let out = conv
                .convert_from_zigbee_session(
                    "FutureTimerBulb",
                    1026,
                    "attributeReport",
                    &[(0, serde_json::json!(100))],
                    &timer_device,
                    &IncomingMessageInfo::default(),
                    &Default::default(),
                )
                .expect("future timer frame");
            assert_eq!(out.get("fired"), Some(&serde_json::json!(expected)));
        }
    }

    #[test]
    fn convert_set_records_write_and_returns_state() {
        let (_tmp, conv) = fixture_dir();
        let outcome = conv
            .convert_set(
                "FixtureBulb",
                1,
                None,
                "window_detection",
                &serde_json::json!("ON"),
                &device(),
                &Default::default(),
            )
            .expect("convertSet outcome");
        assert!(outcome.handled);
        assert_eq!(
            outcome.state.get("window_detection"),
            Some(&serde_json::json!("ON"))
        );
        assert_eq!(
            outcome.ops,
            vec![ConvertSetOp::Write {
                cluster: 6,
                manufacturer_code: None,
                disable_default_response: false,
                attributes: vec![("onOff".to_string(), serde_json::json!(1))],
            }]
        );
    }

    #[test]
    fn convert_set_records_cluster_command_with_payload() {
        let (_tmp, conv) = fixture_dir();
        let outcome = conv
            .convert_set(
                "FixtureBulb",
                1,
                None,
                "on_timed",
                &serde_json::json!(true),
                &device(),
                &Default::default(),
            )
            .expect("convertSet outcome");
        assert!(outcome.handled);
        assert_eq!(
            outcome.ops,
            vec![ConvertSetOp::Command {
                cluster: 6,
                command: 66,
                manufacturer_code: None,
                disable_default_response: false,
                payload: serde_json::json!({"ctrlbits": 0, "ontime": 10, "offwaittime": 5}),
            }]
        );
    }

    #[test]
    fn convert_set_marks_unknown_cluster_as_unsupported() {
        let (_tmp, conv) = fixture_dir();
        let outcome = conv
            .convert_set(
                "FixtureBulb",
                1,
                None,
                "vendor_flag",
                &serde_json::json!(true),
                &device(),
                &Default::default(),
            )
            .expect("convertSet outcome");
        assert!(outcome.handled);
        assert_eq!(outcome.ops.len(), 1);
        assert!(matches!(
            outcome.ops[0],
            ConvertSetOp::Unsupported { ref kind } if kind.contains("unknown cluster")
        ));
    }

    #[test]
    fn convert_set_without_matching_converter_is_not_handled() {
        let (_tmp, conv) = fixture_dir();
        let outcome = conv
            .convert_set(
                "FixtureBulb",
                1,
                None,
                "not_a_key",
                &serde_json::json!(1),
                &device(),
                &Default::default(),
            )
            .expect("convertSet outcome");
        assert!(!outcome.handled);
        assert!(outcome.ops.is_empty());
    }

    const EXTEND_META_DEVICE_JS: &str = r#"
const definitions = [
    {
        model: "ExtendMetaBulb",
        vendor: "Fixture",
        description: "extend meta/endpoint fixture",
        extend: [
            {
                isModernExtend: true,
                meta: {multiEndpoint: true},
                endpoint: (device) => ({l1: 1, l2: 2}),
                fromZigbee: [
                    {
                        cluster: "genOnOff",
                        type: ["attributeReport"],
                        convert: (model, msg) => ({
                            device_model: msg.device.modelID,
                            device_ctor: msg.device.constructor.name,
                            endpoint_ctor: msg.endpoint.constructor.name,
                            endpoint_id: msg.endpoint.ID,
                            multi_endpoint: !!(model.meta && model.meta.multiEndpoint),
                            endpoint_name: model.endpoint ? (msg.endpoint.ID === 2 ? "l2" : "l1") : null,
                        }),
                    },
                ],
            },
        ],
    },
];
module.exports = {definitions};
"#;

    #[test]
    fn extend_meta_endpoint_and_message_device_are_available() {
        let (_tmp, conv) = fixture_dir();
        std::fs::write(
            conv.dir.join("devices").join("extendmeta.js"),
            EXTEND_META_DEVICE_JS,
        )
        .unwrap();
        write_registry(
            &conv,
            serde_json::json!({
                "lookup": {"extendmetabulb": [["extendmeta.js", 0]]},
                "fingerprints": [],
            }),
        );
        let out = conv
            .convert_from_zigbee_with_message(
                "ExtendMetaBulb",
                6,
                "attributeReport",
                &[(0, serde_json::json!(1))],
                &device(),
                &IncomingMessageInfo {
                    endpoint: 2,
                    linkquality: 10,
                    group_id: 0,
                },
                &Default::default(),
            )
            .expect("extend meta conversion");
        assert_eq!(
            out.get("device_model"),
            Some(&serde_json::json!("FixtureBulb"))
        );
        assert_eq!(out.get("device_ctor"), Some(&serde_json::json!("Device")));
        assert_eq!(
            out.get("endpoint_ctor"),
            Some(&serde_json::json!("Endpoint"))
        );
        assert_eq!(out.get("endpoint_id"), Some(&serde_json::json!(2)));
        assert_eq!(out.get("multi_endpoint"), Some(&serde_json::json!(true)));
        assert_eq!(out.get("endpoint_name"), Some(&serde_json::json!("l2")));
    }
}
