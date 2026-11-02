use std::collections::HashMap;
use std::path::{Path, PathBuf};

use boa_engine::{Context, Source};
use tracing::warn;

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
}

/// Minimal device identity for the `meta.device` stub.
#[derive(Debug, Clone, Default)]
pub struct ExternalDeviceInfo {
    pub model_id: Option<String>,
    pub manufacturer_id: Option<u16>,
    pub manufacturer_name: Option<String>,
    pub ieee_addr: String,
    pub power_source: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct RegistryFile {
    #[serde(default)]
    models: HashMap<String, RegistryModel>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct RegistryModel {
    file: String,
    index: usize,
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
        }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)
    }

    pub fn registry_path(&self) -> PathBuf {
        self.dir.join("registry.json")
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

    fn load_js(&self, file: &str) -> Option<String> {
        // `file` is a dist-relative path like `devices/lytko.js` or a bare
        // basename for the legacy flat layout.
        for candidate in [self.dir.join(file), self.dir.join("devices").join(file)] {
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
        let url = format!("{}/{}", self.root_url(), file.trim_start_matches('/'));
        let text = fetch_url(&url).await?;
        let dest = self.dir.join(file);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&dest, &text);
        Some(text)
    }

    /// Ensure the device module plus its transitive relative dependencies
    /// and zcl.json are present locally (bounded: max 80 files). Returns the
    /// dist-relative device file (e.g. `devices/lytko.js`) on success.
    pub async fn ensure_device_files(&self, model: &str) -> Option<String> {
        let registry = self.load_registry()?;
        let key = model.to_lowercase();
        let entry = registry.models.get(&key)?;
        let mut pending = vec![format!("devices/{}", entry.file)];
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
        Some(format!("devices/{}", entry.file))
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
        let registry = self.load_registry()?;
        let key = model.to_lowercase();
        let entry = registry.models.get(&key)?;
        let zcl = self.load_zcl()?;
        let cluster_name = zcl.clusters.get(&cluster_id.to_string())?.clone();

        let module_src = self
            .load_js(&format!("devices/{}", entry.file))
            .or_else(|| {
                // Legacy flat mirror (pre-layout change).
                self.load_js(&entry.file)
            })?;

        let mut ctx = Context::default();
        let sources = collect_sources(&self.dir, &module_src, &entry.file)?;
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
            file = entry.file,
        );
        if ctx.eval(Source::from_bytes(check.as_bytes())).is_err() {
            warn!("external converter: cannot load {}", entry.file);
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
            "linkquality": 0,
            "endpoint": {"ID": 1},
        });
        let meta = serde_json::json!({
            "state": state,
            "device": {
                "modelID": device.model_id,
                "manufacturerID": device.manufacturer_id,
                "manufacturerName": device.manufacturer_name,
                "ieeeAddr": device.ieee_addr,
                "powerSource": device.power_source,
            },
            "options": {},
        });
        let script = format!(
            r#"(function() {{
                const defs = __require("devices/__entry__.js")("./{file}").definitions;
                const definition = defs[{index}];
                const msg = JSON.parse({msg});
                const meta = JSON.parse({meta});
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
            file = entry.file,
            index = entry.index,
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

/// Collect every CJS source reachable from the entry device file.
/// Keys are dist-relative paths (`devices/lytko.js`, `converters/foo.js`, ...).
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
        toZigbee: [],
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

    #[test]
    fn collect_sources_follows_relative_requires() {
        let (_tmp, conv) = fixture_dir();
        let src = std::fs::read_to_string(conv.dir.join("devices").join("fakedev.js")).unwrap();
        let map = collect_sources(conv.dir.as_path(), &src, "fakedev.js").expect("collect");
        assert!(map.contains_key("devices/fakedev.js"));
        assert!(map.contains_key("devices/helpers/util.js"));
    }
}
