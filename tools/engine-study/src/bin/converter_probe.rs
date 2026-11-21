// Real-upstream converter probe for Boa and Kataan.
//
// Not a production runtime: it loads the pinned upstream package through a
// minimal CommonJS loader, prepares definitions and executes selected golden
// scenarios. It records wall time, converter-only time, CPU and RSS.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use serde_json::{Value, json};

static START: OnceLock<Instant> = OnceLock::new();
static ROOT: OnceLock<PathBuf> = OnceLock::new();

fn now_ms() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

#[cfg(feature = "boa")]
fn host_read_file(
    _this: &boa_engine::JsValue,
    args: &[boa_engine::JsValue],
    context: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsValue> {
    use boa_engine::{JsError, JsNativeError, JsValue};
    let path = args
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_escaped();
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(JsValue::from(boa_engine::JsString::from(text))),
        Err(error) => Err(JsError::from(
            JsNativeError::error().with_message(format!("read {path}: {error}")),
        )),
    }
}

#[cfg(feature = "boa")]
fn host_file_exists(
    _this: &boa_engine::JsValue,
    args: &[boa_engine::JsValue],
    context: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsValue> {
    let path = args
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_escaped();
    Ok(boa_engine::JsValue::from(Path::new(&path).is_file()))
}

#[cfg(feature = "boa")]
fn host_op(
    _this: &boa_engine::JsValue,
    _args: &[boa_engine::JsValue],
    context: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsValue> {
    use boa_engine::job::GenericJob;
    use boa_engine::object::builtins::JsPromise;
    let (promise, resolvers) = JsPromise::new_pending(context);
    let realm = context.realm().clone();
    context.enqueue_job(
        GenericJob::new(
            move |context| {
                resolvers.resolve.call(
                    &boa_engine::JsValue::undefined(),
                    &[boa_engine::JsValue::from(true)],
                    context,
                )
            },
            realm,
        )
        .into(),
    );
    Ok(promise.into())
}

#[cfg(feature = "boa")]
fn host_log(
    _this: &boa_engine::JsValue,
    args: &[boa_engine::JsValue],
    context: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsValue> {
    let text = args
        .first()
        .cloned()
        .unwrap_or_default()
        .to_string(context)?
        .to_std_string_escaped();
    eprintln!("[host] {text}");
    Ok(boa_engine::JsValue::undefined())
}

#[cfg(feature = "boa")]
fn host_now(
    _this: &boa_engine::JsValue,
    _args: &[boa_engine::JsValue],
    _context: &mut boa_engine::Context,
) -> boa_engine::JsResult<boa_engine::JsValue> {
    Ok(boa_engine::JsValue::from(now_ms()))
}

#[cfg(feature = "kataan")]
fn host_read_file(
    cx: &mut kataan::Ctx,
    _this: kataan::NanBox,
    args: &[kataan::NanBox],
) -> Result<kataan::NanBox, kataan::NanBox> {
    let path = cx.to_string(args.first().copied().unwrap_or(cx.undefined()))?;
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(cx.string(&text)),
        Err(error) => Err(cx.error(&format!("read {path}: {error}"))),
    }
}

#[cfg(feature = "kataan")]
fn host_file_exists(
    cx: &mut kataan::Ctx,
    _this: kataan::NanBox,
    args: &[kataan::NanBox],
) -> Result<kataan::NanBox, kataan::NanBox> {
    let path = cx.to_string(args.first().copied().unwrap_or(cx.undefined()))?;
    Ok(cx.boolean(Path::new(&path).is_file()))
}

#[cfg(feature = "kataan")]
fn host_op(
    cx: &mut kataan::Ctx,
    _this: kataan::NanBox,
    _args: &[kataan::NanBox],
) -> Result<kataan::NanBox, kataan::NanBox> {
    let value = cx.boolean(true);
    Ok(cx.resolved_promise(value))
}

#[cfg(feature = "kataan")]
fn host_log(
    cx: &mut kataan::Ctx,
    _this: kataan::NanBox,
    args: &[kataan::NanBox],
) -> Result<kataan::NanBox, kataan::NanBox> {
    let text = cx.to_string(args.first().copied().unwrap_or(cx.undefined()))?;
    eprintln!("[host] {text}");
    Ok(cx.undefined())
}

#[cfg(feature = "kataan")]
fn host_now(
    cx: &mut kataan::Ctx,
    _this: kataan::NanBox,
    _args: &[kataan::NanBox],
) -> Result<kataan::NanBox, kataan::NanBox> {
    Ok(cx.number(now_ms()))
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((fraction * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

fn stats(values: &[f64]) -> Value {
    if values.is_empty() {
        return json!({ "count": 0 });
    }
    let total: f64 = values.iter().sum();
    json!({
        "count": values.len(),
        "meanMs": total / values.len() as f64,
        "p50Ms": percentile(values, 0.5),
        "p95Ms": percentile(values, 0.95),
        "p99Ms": percentile(values, 0.99),
        "minMs": values.iter().copied().fold(f64::INFINITY, f64::min),
        "maxMs": values.iter().copied().fold(0.0, f64::max),
    })
}

fn proc_cpu_ms() -> f64 {
    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return 0.0;
    };
    let Some(close) = stat.rfind(')') else {
        return 0.0;
    };
    let fields: Vec<&str> = stat[close + 2..].split_whitespace().collect();
    // Fields after comm: state=0, ... utime=11, stime=12 (1-based 14/15)
    let utime: f64 = fields.get(11).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let stime: f64 = fields.get(12).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let ticks = 100.0;
    (utime + stime) * 1000.0 / ticks
}

fn proc_rss_kb(field: &str) -> u64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
        }
    }
    0
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

struct Args {
    package: PathBuf,
    external: PathBuf,
    herdsman: PathBuf,
    scenarios: PathBuf,
    zcl: PathBuf,
    bootstrap: PathBuf,
    filter: String,
    iterations: usize,
    profile_host: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        package: PathBuf::from(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"),
        external: PathBuf::from("tools/bench/fixtures/external/L101Ze_z2m.js"),
        herdsman: PathBuf::from(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman"),
        scenarios: PathBuf::from("tools/engine-study/fixtures/scenarios.json"),
        zcl: PathBuf::from("tools/engine-study/fixtures/zcl.json"),
        bootstrap: PathBuf::from("tools/engine-study/fixtures/bootstrap.js"),
        filter: String::new(),
        iterations: 20,
        profile_host: false,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        let mut value = || iter.next().expect("missing value");
        match flag.as_str() {
            "--package" => args.package = PathBuf::from(value()),
            "--external" => args.external = PathBuf::from(value()),
            "--herdsman" => args.herdsman = PathBuf::from(value()),
            "--scenarios" => args.scenarios = PathBuf::from(value()),
            "--zcl" => args.zcl = PathBuf::from(value()),
            "--bootstrap" => args.bootstrap = PathBuf::from(value()),
            "--filter" => args.filter = value(),
            "--iterations" => args.iterations = value().parse().expect("iterations"),
            "--profile-host" => args.profile_host = true,
            other => panic!("unknown argument {other}"),
        }
    }
    args
}

fn main() {
    START.get_or_init(Instant::now);
    let args = parse_args();
    let root = args.package.canonicalize().expect("package root");
    ROOT.set(root.clone()).ok();
    let scenarios = read_json(&args.scenarios);
    let scenarios = scenarios.as_array().expect("scenario array").clone();

    #[cfg(feature = "boa")]
    {
        run_boa(&args, &root, &scenarios);
    }
    #[cfg(feature = "kataan")]
    {
        run_kataan(&args, &root, &scenarios);
    }
    #[cfg(feature = "qjs")]
    {
        run_qjs(&args, &root, &scenarios);
    }
    #[cfg(not(any(feature = "boa", feature = "kataan", feature = "qjs")))]
    {
        let _ = (&args, &scenarios);
        eprintln!("select --features boa or --features kataan");
        std::process::exit(1);
    }
}

#[cfg(feature = "qjs")]
fn run_qjs(args: &Args, root: &Path, scenarios: &[Value]) {
    use quickjs_oxide::engine::api::core::{
        JsString, NativeErrorKind, Runtime, RuntimeError, Scope, Value as QValue,
    };
    use quickjs_oxide_host::SystemHost;

    let runtime = Runtime::new_with_host_services(SystemHost::default());
    let context = runtime.new_context().expect("context");
    let global = context.global_object().expect("global");

    context
        .define_function(&global, "__readFile", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let path = scope.to_js_string(&argument)?.to_utf8_lossy();
            match std::fs::read_to_string(&path) {
                Ok(text) => Ok(QValue::String(
                    JsString::try_from_utf8(&text)
                        .map_err(|_| RuntimeError::Exception(QValue::Undefined))?,
                )),
                Err(error) => {
                    eprintln!("[host] read {path}: {error}");
                    Err(scope
                        .throw_error(NativeErrorKind::Internal, &format!("read {path}: {error}")))
                }
            }
        })
        .expect("readFile");
    context
        .define_function(&global, "__fileExists", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let path = scope.to_js_string(&argument)?.to_utf8_lossy();
            Ok(QValue::Bool(Path::new(&path).is_file()))
        })
        .expect("fileExists");
    context
        .define_function(&global, "__now", 0, |_scope: &mut Scope<'_>| {
            Ok(QValue::Float(now_ms()))
        })
        .expect("now");
    context
        .define_function(&global, "__log", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let message = scope.to_js_string(&argument)?.to_utf8_lossy();
            eprintln!("[host] {message}");
            Ok(QValue::Undefined)
        })
        .expect("log");
    context
        .define_function(&global, "__hostOp", 1, |scope: &mut Scope<'_>| {
            let capability = scope.new_promise()?;
            let promise = capability.promise.clone();
            capability.resolve(scope, QValue::Bool(true))?;
            Ok(QValue::Object(promise))
        })
        .expect("hostOp");

    let preamble = format!(
        "globalThis.__root = {};\nglobalThis.__external = {};\nglobalThis.__zclPath = {};\nglobalThis.__herdsmanRoot = {};\nglobalThis.__profileHost = {};",
        serde_json::to_string(&root.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.external.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.zcl.to_string_lossy()).unwrap(),
        serde_json::to_string(
            &args
                .herdsman
                .canonicalize()
                .unwrap_or_else(|_| args.herdsman.clone())
                .to_string_lossy()
        )
        .unwrap(),
        args.profile_host,
    );
    context.eval(&preamble).expect("globals");

    let bootstrap = format!(
        "try {{ {} }} catch (e) {{ __log(String(e && (e.stack || e.message) || e)); throw e; }}",
        std::fs::read_to_string(&args.bootstrap).expect("bootstrap")
    );
    let started = Instant::now();
    context.eval(&bootstrap).expect("bootstrap");
    runtime.run_jobs().expect("bootstrap jobs");
    context.eval("__init()").expect("init");
    let cold_root_ms = started.elapsed().as_secs_f64() * 1000.0;

    let first = scenarios
        .iter()
        .find(|s| args.filter.is_empty() || s["id"].as_str().unwrap_or("").contains(&args.filter));
    let mut prepare_ms = 0.0;
    if let Some(scenario) = first {
        let spec = serde_json::to_string(&scenario["definition"]).unwrap();
        let started = Instant::now();
        context
            .eval(&format!("__prepareOnly({spec})"))
            .expect("prepare");
        prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    let mut cases = Vec::new();
    for scenario in scenarios {
        let id = scenario["id"].as_str().unwrap_or("?");
        if !args.filter.is_empty() && !id.contains(&args.filter) {
            continue;
        }
        let spec = serde_json::to_string(&serde_json::to_string(&scenario).unwrap()).unwrap();
        let source = format!(
            "globalThis.__lastResult=undefined;globalThis.__lastError=undefined;__bench({spec}, {})",
            args.iterations
        );
        let mut error = Value::Null;
        if let Err(run_error) = context.eval(&source) {
            let text = runtime
                .print_value(&match run_error {
                    quickjs_oxide::engine::api::core::RuntimeError::Exception(value) => value,
                    other => {
                        QValue::String(JsString::try_from_utf8(&format!("{other:?}")).unwrap())
                    }
                })
                .unwrap_or_default();
            error = json!(text);
        }
        if let Err(run_error) = runtime.run_jobs() {
            error = json!(format!("{run_error:?}"));
        }
        let text = context
            .eval("globalThis.__lastResult")
            .ok()
            .and_then(|value| match value {
                QValue::String(text) => Some(text.to_utf8_lossy()),
                _ => None,
            })
            .unwrap_or_default();
        let last_error = context
            .eval("globalThis.__lastError")
            .ok()
            .and_then(|value| match value {
                QValue::String(text) => Some(text.to_utf8_lossy()),
                _ => None,
            })
            .unwrap_or_default();
        if !last_error.is_empty() && last_error != "undefined" {
            error = json!(last_error);
        }
        let summary = summarize_results(id, &text, error);
        eprintln!(
            "qjs {id} ... {:.3} ms",
            summary["wallMs"]["p50Ms"].as_f64().unwrap_or(0.0)
        );
        cases.push(summary);
    }

    let output = json!({
        "engine": "quickjs-oxide",
        "version": "0.5.0",
        "coldRootMs": cold_root_ms,
        "prepareMs": prepare_ms,
        "coldTotalMs": cold_root_ms + prepare_ms,
        "cpuMs": proc_cpu_ms(),
        "rssBeforeKb": proc_rss_kb("VmRSS:"),
        "maxRssKb": proc_rss_kb("VmHWM:"),
        "cases": cases,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn summarize_results(id: &str, text: &str, error: Value) -> Value {
    if let Ok(Value::Array(results)) = serde_json::from_str::<Value>(text) {
        let wall: Vec<f64> = results
            .iter()
            .filter_map(|r| r["wallMs"].as_f64())
            .collect();
        let converter: Vec<f64> = results
            .iter()
            .filter_map(|r| r["converterMs"].as_f64())
            .collect();
        let calls = results
            .first()
            .and_then(|r| r["calls"].as_array())
            .map(|v| v.len())
            .unwrap_or(0);
        let sample = results.first().map(|r| {
            json!({
                "result": r["result"],
                "published": r["published"],
                "converterKey": r["converterKey"],
                "hostMs": r["hostMs"],
                "hostCalls": r["hostCalls"],
                "perf": r["perf"],
                "calls": r["calls"],
            })
        });
        json!({
            "id": id,
            "iterations": wall.len(),
            "wallMs": stats(&wall),
            "converterMs": stats(&converter),
            "calls": calls,
            "sample": sample,
            "error": error,
        })
    } else {
        json!({ "id": id, "iterations": 0, "error": if error.is_null() { json!(format!("unparsed result: {text}")) } else { error } })
    }
}

#[cfg(feature = "boa")]
fn run_boa(args: &Args, root: &Path, scenarios: &[Value]) {
    use boa_engine::{Context, JsString, NativeFunction, Source};

    let mut context = Context::default();
    context
        .runtime_limits_mut()
        .set_loop_iteration_limit(50_000_000);
    context.runtime_limits_mut().set_recursion_limit(1_000);
    for (name, length, function) in [
        ("__readFile", 1, NativeFunction::from_fn_ptr(host_read_file)),
        (
            "__fileExists",
            1,
            NativeFunction::from_fn_ptr(host_file_exists),
        ),
        ("__now", 0, NativeFunction::from_fn_ptr(host_now)),
        ("__log", 1, NativeFunction::from_fn_ptr(host_log)),
        ("__hostOp", 1, NativeFunction::from_fn_ptr(host_op)),
    ] {
        context
            .register_global_callable(JsString::from(name), length, function)
            .expect("register host function");
    }

    let preamble = format!(
        "globalThis.__root = {};\nglobalThis.__external = {};\nglobalThis.__zclPath = {};\nglobalThis.__herdsmanRoot = {};\nglobalThis.__profileHost = {};",
        serde_json::to_string(&root.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.external.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.zcl.to_string_lossy()).unwrap(),
        serde_json::to_string(
            &args
                .herdsman
                .canonicalize()
                .unwrap_or_else(|_| args.herdsman.clone())
                .to_string_lossy()
        )
        .unwrap(),
        args.profile_host,
    );
    context
        .eval(Source::from_bytes(&preamble))
        .expect("set globals");

    let bootstrap = format!(
        "try {{ {} }} catch (e) {{ __log(String(e && (e.stack || e.message) || e)); throw e; }}",
        std::fs::read_to_string(&args.bootstrap).expect("bootstrap")
    );
    let started = Instant::now();
    context
        .eval(Source::from_bytes(&bootstrap))
        .unwrap_or_else(|error| panic!("bootstrap eval: {error}"));
    context.eval(Source::from_bytes("__init()")).expect("init");
    let cold_root_ms = started.elapsed().as_secs_f64() * 1000.0;

    // Prepare the first selected definition to separate package load from
    // definition preparation.
    let first = scenarios
        .iter()
        .find(|s| args.filter.is_empty() || s["id"].as_str().unwrap_or("").contains(&args.filter));
    let mut prepare_ms = 0.0;
    if let Some(scenario) = first {
        let spec = serde_json::to_string(&scenario["definition"]).unwrap();
        let source = format!("__prepareOnly({spec})");
        let started = Instant::now();
        context
            .eval(Source::from_bytes(&source))
            .unwrap_or_else(|error| panic!("prepare: {error}"));
        prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    let mut cases = Vec::new();
    for scenario in scenarios {
        let id = scenario["id"].as_str().unwrap_or("?");
        if !args.filter.is_empty() && !id.contains(&args.filter) {
            continue;
        }
        let spec = serde_json::to_string(&serde_json::to_string(&scenario).unwrap()).unwrap();
        let source = format!(
            "globalThis.__lastResult=undefined;globalThis.__lastError=undefined;__bench({spec}, {})",
            args.iterations
        );
        let script = boa_engine::Script::parse(Source::from_bytes(&source), None, &mut context)
            .expect("parse bench script");
        let result_script = boa_engine::Script::parse(
            Source::from_bytes("globalThis.__lastResult"),
            None,
            &mut context,
        )
        .expect("parse result script");
        let error_script = boa_engine::Script::parse(
            Source::from_bytes("globalThis.__lastError"),
            None,
            &mut context,
        )
        .expect("parse error script");
        let mut error = Value::Null;
        if let Err(run_error) = script.evaluate(&mut context) {
            error = json!(format!("{run_error}"));
        } else if let Err(run_error) = context.run_jobs() {
            error = json!(format!("{run_error}"));
        }
        let text = result_script
            .evaluate(&mut context)
            .and_then(|value| value.to_string(&mut context))
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let last_error = error_script
            .evaluate(&mut context)
            .and_then(|value| value.to_string(&mut context))
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        if last_error != "undefined" && !last_error.is_empty() {
            error = json!(last_error);
        }
        let summary = summarize_results(id, &text, error);
        eprintln!(
            "boa {id} ... {:.3} ms",
            summary["wallMs"]["p50Ms"].as_f64().unwrap_or(0.0)
        );
        cases.push(summary);
    }

    let output = json!({
        "engine": "boa",
        "version": "0.22.0",
        "coldRootMs": cold_root_ms,
        "prepareMs": prepare_ms,
        "coldTotalMs": cold_root_ms + prepare_ms,
        "cpuMs": proc_cpu_ms(),
        "rssBeforeKb": proc_rss_kb("VmRSS:"),
        "maxRssKb": proc_rss_kb("VmHWM:"),
        "cases": cases,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(feature = "kataan")]
fn run_kataan(args: &Args, root: &Path, scenarios: &[Value]) {
    use kataan::Interp;
    use kataan::parser::Parser;

    let mut interp = Interp::new();
    interp.register_global_fn("__readFile", 1, host_read_file);
    interp.register_global_fn("__fileExists", 1, host_file_exists);
    interp.register_global_fn("__now", 0, host_now);
    interp.register_global_fn("__log", 1, host_log);
    interp.register_global_fn("__hostOp", 1, host_op);

    let preamble = format!(
        "globalThis.__root = {};\nglobalThis.__external = {};\nglobalThis.__zclPath = {};\nglobalThis.__herdsmanRoot = {};\nglobalThis.__profileHost = {};",
        serde_json::to_string(&root.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.external.to_string_lossy()).unwrap(),
        serde_json::to_string(&args.zcl.to_string_lossy()).unwrap(),
        serde_json::to_string(
            &args
                .herdsman
                .canonicalize()
                .unwrap_or_else(|_| args.herdsman.clone())
                .to_string_lossy()
        )
        .unwrap(),
        args.profile_host,
    );
    let program = Parser::parse_program(&preamble).expect("preamble parse");
    interp.run(&program).expect("set globals");

    let bootstrap = format!(
        "try {{ {} }} catch (e) {{ __log(String(e && (e.stack || e.message) || e)); throw e; }}",
        std::fs::read_to_string(&args.bootstrap).expect("bootstrap")
    );
    let started = Instant::now();
    let program = Parser::parse_program(&bootstrap).expect("bootstrap parse");
    if let Err(error) = interp.run(&program) {
        let text = match error {
            kataan::nbexec::ExecError::Throw(value) => interp.realm().to_display_string(value),
            other => format!("{other:?}"),
        };
        panic!("bootstrap run: {text}");
    }
    let init = Parser::parse_program("__init()").expect("init parse");
    interp.run(&init).expect("init");
    let cold_root_ms = started.elapsed().as_secs_f64() * 1000.0;

    let first = scenarios
        .iter()
        .find(|s| args.filter.is_empty() || s["id"].as_str().unwrap_or("").contains(&args.filter));
    let mut prepare_ms = 0.0;
    if let Some(scenario) = first {
        let spec = serde_json::to_string(&scenario["definition"]).unwrap();
        let program =
            Parser::parse_program(&format!("__prepareOnly({spec})")).expect("prepare parse");
        let started = Instant::now();
        interp
            .run(&program)
            .unwrap_or_else(|error| panic!("prepare: {error:?}"));
        prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    let mut cases = Vec::new();
    for scenario in scenarios {
        let id = scenario["id"].as_str().unwrap_or("?");
        if !args.filter.is_empty() && !id.contains(&args.filter) {
            continue;
        }
        let spec = serde_json::to_string(&serde_json::to_string(&scenario).unwrap()).unwrap();
        let source = format!(
            "globalThis.__lastResult=undefined;globalThis.__lastError=undefined;__bench({spec}, {})",
            args.iterations
        );
        let program = Parser::parse_program(&source).expect("bench parse");
        let result_program =
            Parser::parse_program("globalThis.__lastResult").expect("result parse");
        let error_program = Parser::parse_program("globalThis.__lastError").expect("error parse");
        let mut error = Value::Null;
        if let Err(run_error) = interp.run(&program) {
            error = json!(format!("{run_error:?}"));
        }
        let text = interp
            .run(&result_program)
            .map(|value| interp.realm().to_display_string(value))
            .unwrap_or_default();
        let last_error = interp
            .run(&error_program)
            .map(|value| interp.realm().to_display_string(value))
            .unwrap_or_default();
        if last_error != "undefined" && !last_error.is_empty() {
            error = json!(last_error);
        }
        let summary = summarize_results(id, &text, error);
        eprintln!(
            "kataan {id} ... {:.3} ms",
            summary["wallMs"]["p50Ms"].as_f64().unwrap_or(0.0)
        );
        cases.push(summary);
    }

    let output = json!({
        "engine": "kataan",
        "version": "0.0.10",
        "coldRootMs": cold_root_ms,
        "prepareMs": prepare_ms,
        "coldTotalMs": cold_root_ms + prepare_ms,
        "cpuMs": proc_cpu_ms(),
        "rssBeforeKb": proc_rss_kb("VmRSS:"),
        "maxRssKb": proc_rss_kb("VmHWM:"),
        "cases": cases,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
