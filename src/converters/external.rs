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

enum ConverterJob {
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
}

struct ConverterSession {
    ctx: Context,
    file: String,
    index: usize,
    zcl: ZclMaps,
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
    #[serde(default, rename = "ieeeAddr")]
    ieee_addr: Option<String>,
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
        let registry = self.load_registry()?;
        let (file, index) = select_definition(&registry, model, device)?;
        let zcl = self.load_zcl()?;
        let cluster_name = zcl.clusters.get(&cluster_id.to_string())?.clone();

        let module_src = self.load_js(&format!("devices/{file}")).or_else(|| {
            // Legacy flat mirror (pre-layout change).
            self.load_js(&file)
        })?;

        let mut ctx = Context::default();
        let sources = collect_sources(&self.dir, &module_src, &file)?;
        let bootstrap = format!(
            r#"
            const __sources = {sources};
            const __stub = {stub};
            const __modules = {{}};
            function __resolve(from, spec) {{
                if (spec === "zigbee-herdsman") return "__builtin__/zigbee-herdsman";
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
                    if (key === "__builtin__/zigbee-herdsman") {{
                        if (!__modules[key]) {{
                            const module = {{exports: {{}}}};
                            const fn_ = new Function("require", "module", "exports", __stub);
                            fn_(__require(key), module, module.exports);
                            __modules[key] = module;
                        }}
                        return __modules[key].exports;
                    }}
                    if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                    if (!__modules[key]) {{
                        const module = {{exports: {{}}}};
                        __modules[key] = module;
                        const fn_ = new Function("require", "module", "exports", __sources[key]);
                        fn_(__require(key), module, module.exports);
                    }}
                    return __modules[key].exports;
                }};
            }}
            function __prepareDefinition(definition) {{
                if (!definition || !Array.isArray(definition.extend)) return definition;
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
                    if (extend.exposes) prepared.exposes.push(...extend.exposes);
                    if (extend.options) prepared.options.push(...extend.options);
                }}
                return prepared;
            }}
            "#,
            sources = serde_json::to_string(&sources).unwrap_or_default(),
            stub = serde_json::to_string(&stub_source(&zcl)).unwrap_or_default(),
        );
        if ctx.eval(Source::from_bytes(bootstrap.as_bytes())).is_err() {
            warn!("external converter: bootstrap eval failed");
            return None;
        }

        // Load the device module (its `definitions` export) lazily inside the
        // dispatch script below; here we only verify the module is loadable
        // by evaluating its source once (syntax + top-level side effects).
        let check = format!(
            r#"__require("devices/__entry__.js")("./{file}")"#,
            file = file,
        );
        if ctx.eval(Source::from_bytes(check.as_bytes())).is_err() {
            warn!("external converter: cannot load {file}");
            return None;
        }

        // Build msg + meta as JSON, then run dispatch fully inside JS so that
        // converter functions receive real objects (not strings).
        let data_obj: serde_json::Map<String, serde_json::Value> = reports
            .iter()
            .map(|(id, v)| {
                let name = zcl
                    .attributes
                    .get(&cluster_name)
                    .and_then(|m| m.get(&id.to_string()))
                    .cloned()
                    .unwrap_or_else(|| format!("attr_{id}"));
                (name, v.clone())
            })
            .collect();
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
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = __prepareDefinition(defs[{index}]);
                const msg = JSON.parse({msg});
                const meta = JSON.parse({meta});
                const incomingDevice = meta.device;
                const device = globalThis.__device || (globalThis.__device = incomingDevice);
                Object.assign(device, incomingDevice);
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                meta.device = device;
                msg.endpoint.getDevice = () => device;
                const __published = [];
                const publish = (p) => {{ __published.push(p); }};
                const out = {{}};
                for (const c of (definition.fromZigbee || [])) {{
                    try {{
                        const cl = c.cluster;
                        const tys = Array.isArray(c.type) ? c.type : [c.type];
                        if ((cl === msg.cluster || String(cl) === String({cluster_id})) && tys.includes(msg.type)) {{
                            const r = c.convert(definition, msg, publish, {{}}, meta);
                            if (r !== undefined && r !== null) {{
                                const o = (typeof r === "object") ? r : {{}};
                                for (const k of Object.keys(o)) out[k] = o[k];
                            }}
                        }}
                    }} catch (e) {{
                        // One bad converter must not kill the frame; Rust logs below via returned __errors.
                    }}
                }}
                for (const p of __published) {{
                    if (p && typeof p === "object") {{
                        for (const k of Object.keys(p)) out[k] = p[k];
                    }}
                }}
                return JSON.stringify(out);
            }})()"#,
            file = file,
            index = index,
            msg = serde_json::to_string(&serde_json::to_string(&msg).unwrap()).unwrap(),
            meta = serde_json::to_string(&serde_json::to_string(&meta).unwrap()).unwrap(),
            cluster_id = cluster_id,
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
        let sources = collect_sources(&self.dir, &module_src, &file)?;
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
        })
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
                const incomingDevice = meta.device;
                const device = globalThis.__device || (globalThis.__device = incomingDevice);
                Object.assign(device, incomingDevice);
                device.getEndpoint = (id) => (device.endpoints || []).find((endpoint) => endpoint.ID === id) || {{ID: id}};
                meta.device = device;
                msg.endpoint.getDevice = () => device;
                globalThis.__dispatchDone = false;
                globalThis.__dispatchError = null;
                globalThis.__dispatchOut = {{}};
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
                            }} catch (e) {{}}
                        }}
                        for (const p of __published) {{
                            if (p && typeof p === "object") {{
                                for (const k of Object.keys(p)) out[k] = p[k];
                            }}
                        }}
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
        let Some(sources) = collect_sources(&self.dir, &module_src, &file) else {
            return Vec::new();
        };
        let cluster_ids: HashMap<&str, u16> = zcl
            .clusters
            .iter()
            .filter_map(|(id, name)| id.parse::<u16>().ok().map(|id| (name.as_str(), id)))
            .collect();
        let stub = stub_source(&zcl);
        let bootstrap = format!(
            r#"
            const __sources = {sources};
            const __stub = {stub};
            const __modules = {{}};
            function __resolve(from, spec) {{
                if (spec === "zigbee-herdsman") return "__builtin__/zigbee-herdsman";
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
                    if (key === "__builtin__/zigbee-herdsman") {{
                        if (!__modules[key]) {{
                            const module = {{exports: {{}}}};
                            const fn_ = new Function("require", "module", "exports", __stub);
                            fn_(__require(key), module, module.exports);
                            __modules[key] = module;
                        }}
                        return __modules[key].exports;
                    }}
                    if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                    if (!__modules[key]) {{
                        const module = {{exports: {{}}}};
                        __modules[key] = module;
                        const fn_ = new Function("require", "module", "exports", __sources[key]);
                        fn_(__require(key), module, module.exports);
                    }}
                    return __modules[key].exports;
                }};
            }}
            function __prepareDefinition(definition) {{
                if (!definition || !Array.isArray(definition.extend)) return definition;
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
                }}
                return prepared;
            }}
            "#,
            sources = serde_json::to_string(&sources).unwrap_or_default(),
            stub = serde_json::to_string(&stub).unwrap_or_default(),
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

fn cjs_bootstrap(sources: &HashMap<String, String>, zcl: &ZclMaps) -> String {
    let stub = stub_source(zcl);
    format!(
        r#"
        const __sources = {sources};
        const __stub = {stub};
        const __modules = {{}};
        function __resolve(from, spec) {{
            if (spec === "zigbee-herdsman") return "__builtin__/zigbee-herdsman";
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
                if (key === "__builtin__/zigbee-herdsman") {{
                    if (!__modules[key]) {{
                        const module = {{exports: {{}}}};
                        const fn_ = new Function("require", "module", "exports", __stub);
                        fn_(__require(key), module, module.exports);
                        __modules[key] = module;
                    }}
                    return __modules[key].exports;
                }}
                if (!__sources[key]) throw new Error("Cannot find module '" + spec + "' from '" + from + "'");
                if (!__modules[key]) {{
                    const module = {{exports: {{}}}};
                    __modules[key] = module;
                    const fn_ = new Function("require", "module", "exports", __sources[key]);
                    fn_(__require(key), module, module.exports);
                }}
                return __modules[key].exports;
            }};
        }}
        function __prepareDefinition(definition) {{
            if (!definition || !Array.isArray(definition.extend)) return definition;
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
            }}
            return prepared;
        }}
        "#,
        sources = serde_json::to_string(sources).unwrap_or_default(),
        stub = serde_json::to_string(&stub).unwrap_or_default(),
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
        let sources = collect_sources(&self.dir, &module_src, &file)?;
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
/// Collect every CJS source reachable from the entry device file.
fn collect_sources(
    dir: &Path,
    entry_src: &str,
    entry_file: &str,
) -> Option<HashMap<String, String>> {
    let mut sources = HashMap::new();
    let mut stack = vec![format!("devices/{entry_file}")];
    sources.insert(format!("devices/{entry_file}"), entry_src.to_string());
    while let Some(path) = stack.pop() {
        let src = sources.get(&path)?.clone();
        for spec in required_specs(&src) {
            if spec == "zigbee-herdsman" {
                continue;
            }
            let resolved = resolve_relative(&path, &spec)?;
            if sources.contains_key(&resolved) {
                continue;
            }
            // dist layout mirror first, then legacy flat file.
            let local = dir.join(&resolved);
            let content = std::fs::read_to_string(&local).ok()?;
            sources.insert(resolved.clone(), content);
            stack.push(resolved);
        }
    }
    Some(sources)
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
        const Zcl = {{
            ManufacturerCode: __mfr,
            DataType: __dt,
            Clusters: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                if (p in __cl) return {{ID: __cl[p]}};
                return __throw("Zcl.Clusters." + String(p));
            }} }}),
            Status: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.Status." + String(p));
            }} }}),
            FrameType: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.FrameType." + String(p));
            }} }}),
            Utils: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.Utils." + String(p));
            }} }}),
            BuffaloZclDataType: new Proxy({{}}, {{ get: (t, p) => {{
                if (p === "__esModule" || p === "then") return undefined;
                return __throw("Zcl.BuffaloZclDataType." + String(p));
            }} }}),
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
                    return {temperature: double(msg.data.measuredValue) / 100.0};
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
        ],
    },
];
module.exports = {definitions};
"#;

    const HELPER_JS: &str = r#"
function double(x) { return x * 2; }
module.exports = {double};
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
            dir.join("registry.json"),
            serde_json::to_string(&registry).unwrap(),
        )
        .unwrap();
        let zcl = serde_json::json!({
            "manufacturerCodes": {"TEXAS_INSTRUMENTS": 7},
            "dataTypes": {},
            "clusters": {"1026": "msTemperatureMeasurement"},
            "attributes": {"msTemperatureMeasurement": {"0": "measuredValue"}},
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
        let map = collect_sources(conv.dir.as_path(), &src, "fakedev.js").expect("collect");
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
}
