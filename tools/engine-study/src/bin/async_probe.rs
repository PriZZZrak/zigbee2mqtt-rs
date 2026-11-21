// Non-blocking async host prototype for the selected engine (quickjs-oxide).
//
// The host returns *pending* Promises; Rust owns the queue and settles them
// later. While one device's configure waits, independent jobs are advanced by
// the same scheduler. Also exercises host errors, timeout/cancellation and
// recovery. This is an experiment, not a runtime.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use quickjs_oxide::engine::api::core::{
    JsString, PromiseCapability, Runtime, RuntimeError, Scope, Value,
};
use quickjs_oxide_host::SystemHost;
use serde_json::{Value as Json, json};

static START: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

type Pending = Rc<RefCell<VecDeque<PromiseCapability>>>;

fn arg(name: &str, fallback: &str) -> String {
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == name {
            return iter.next().expect("missing value");
        }
    }
    fallback.to_owned()
}

fn read_json(path: &Path) -> Json {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("parse json")
}

fn eval_string(context: &quickjs_oxide::engine::api::core::Context, source: &str) -> String {
    match context.eval(source) {
        Ok(Value::String(text)) => text.to_utf8_lossy(),
        _ => String::new(),
    }
}

fn last_result(context: &quickjs_oxide::engine::api::core::Context) -> String {
    eval_string(context, "globalThis.__lastResult")
}

fn drive(
    runtime: &Runtime,
    context: &mut quickjs_oxide::engine::api::core::Context,
    pending: &Pending,
    settle: &mut impl FnMut(
        &mut quickjs_oxide::engine::api::core::Context,
        PromiseCapability,
    ) -> Result<(), RuntimeError>,
) -> Result<(f64, f64, usize), RuntimeError> {
    let mut settle_ms = 0.0;
    let mut jobs_ms = 0.0;
    let mut turns = 0;
    loop {
        if pending.borrow().is_empty() {
            let started = Instant::now();
            runtime.run_jobs()?;
            jobs_ms += started.elapsed().as_secs_f64() * 1000.0;
            if pending.borrow().is_empty() {
                return Ok((settle_ms, jobs_ms, turns));
            }
        }
        let capabilities: Vec<_> = pending.borrow_mut().drain(..).collect();
        let started = Instant::now();
        for capability in capabilities {
            settle(context, capability)?;
        }
        settle_ms += started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        runtime.run_jobs()?;
        jobs_ms += started.elapsed().as_secs_f64() * 1000.0;
        turns += 1;
    }
}

fn main() {
    START.get_or_init(Instant::now);
    let package = PathBuf::from(arg(
        "--package",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters",
    ));
    let herdsman = PathBuf::from(arg(
        "--herdsman",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman",
    ));
    let external = PathBuf::from(arg(
        "--external",
        "tools/bench/fixtures/external/L101Ze_z2m.js",
    ));
    let scenarios = read_json(Path::new(&arg(
        "--scenarios",
        "tools/engine-study/fixtures/scenarios.json",
    )));
    let zcl = arg("--zcl", "tools/engine-study/fixtures/zcl.json");
    let bootstrap = arg("--bootstrap", "tools/engine-study/fixtures/bootstrap.js");
    let jobs: usize = arg("--jobs", "8").parse().expect("jobs");
    let root = package.canonicalize().expect("package root");

    let configure = scenarios
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "configure-danfoss")
        .expect("configure-danfoss");
    let job = scenarios
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "set-ikea-brightness")
        .expect("set-ikea-brightness");

    let runtime = Runtime::new_with_host_services(SystemHost::default());
    let mut context = runtime.new_context().expect("context");
    let global = context.global_object().expect("global");
    let pending: Pending = Rc::new(RefCell::new(VecDeque::new()));
    let host_pending = Rc::clone(&pending);

    context
        .define_function(&global, "__readFile", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let path = scope.to_js_string(&argument)?.to_utf8_lossy();
            match std::fs::read_to_string(&path) {
                Ok(text) => Ok(Value::String(JsString::try_from_utf8(&text).unwrap())),
                Err(error) => Err(scope.throw_error(
                    quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                    &format!("read {path}: {error}"),
                )),
            }
        })
        .expect("readFile");
    context
        .define_function(&global, "__fileExists", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let path = scope.to_js_string(&argument)?.to_utf8_lossy();
            Ok(Value::Bool(Path::new(&path).is_file()))
        })
        .expect("fileExists");
    context
        .define_function(&global, "__now", 0, |_scope: &mut Scope<'_>| {
            Ok(Value::Float(now_ms()))
        })
        .expect("now");
    context
        .define_function(&global, "__log", 1, |scope: &mut Scope<'_>| {
            let argument = scope.arg(0);
            let message = scope.to_js_string(&argument)?.to_utf8_lossy();
            eprintln!("[host] {message}");
            Ok(Value::Undefined)
        })
        .expect("log");
    context
        .define_function(&global, "__hostOp", 1, move |scope: &mut Scope<'_>| {
            let capability = scope.new_promise()?;
            let promise = capability.promise.clone();
            host_pending.borrow_mut().push_back(capability);
            Ok(Value::Object(promise))
        })
        .expect("hostOp");

    let preamble = format!(
        "globalThis.__root = {};\nglobalThis.__external = {};\nglobalThis.__zclPath = {};\nglobalThis.__herdsmanRoot = {};\nglobalThis.__profileHost = false;",
        serde_json::to_string(&root.to_string_lossy()).unwrap(),
        serde_json::to_string(&external.to_string_lossy()).unwrap(),
        serde_json::to_string(&zcl).unwrap(),
        serde_json::to_string(
            &herdsman
                .canonicalize()
                .unwrap_or_else(|_| herdsman.clone())
                .to_string_lossy()
        )
        .unwrap(),
    );
    context.eval(&preamble).expect("globals");
    context
        .eval(&std::fs::read_to_string(&bootstrap).expect("bootstrap"))
        .expect("bootstrap");
    runtime.run_jobs().expect("bootstrap jobs");
    context.eval("__init()").expect("init");

    let mut results: Vec<Json> = Vec::new();
    let configure_json = serde_json::to_string(&serde_json::to_string(configure).unwrap()).unwrap();
    let job_json = serde_json::to_string(&serde_json::to_string(job).unwrap()).unwrap();

    // 1. Normal scheduling: configure + independent jobs that also use host
    // I/O. Reports/sets must complete while configure is still waiting.
    let initial_started = Instant::now();
    context
        .eval(&format!(
            "globalThis.__lastResult='';__asyncStart({configure_json},{job_json},{jobs})"
        ))
        .expect("async start");
    let initial_eval_ms = initial_started.elapsed().as_secs_f64() * 1000.0;
    runtime.run_jobs().expect("start jobs");
    let initial_jobs_ms = initial_started.elapsed().as_secs_f64() * 1000.0 - initial_eval_ms;
    let started = Instant::now();
    let mut settled = 0usize;
    let normal = drive(
        &runtime,
        &mut context,
        &pending,
        &mut |context, capability| {
            settled += 1;
            capability.resolve(context, Value::Bool(true))
        },
    );
    let (settle_ms, jobs_ms, turns) = normal.as_ref().copied().unwrap_or((0.0, 0.0, 0));
    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    let read_started = Instant::now();
    runtime.run_jobs().ok();
    let state_text = last_result(&context);
    let read_ms = read_started.elapsed().as_secs_f64() * 1000.0;
    let state: Json = serde_json::from_str(&state_text).unwrap_or(Json::Null);
    let configure_done = state["configureDone"].as_f64().unwrap_or(0.0);
    let first_job = state["firstJobDone"].as_f64().unwrap_or(0.0);
    results.push(json!({
        "mode": "normal",
        "engine": "quickjs-oxide",
        "jobs": jobs,
        "hostOpsSettled": settled,
        "settleMs": settle_ms,
        "runJobsMs": jobs_ms,
        "turns": turns,
        "readResultMs": read_ms,
        "initialEvalMs": initial_eval_ms,
        "initialJobsMs": initial_jobs_ms,
        "totalMs": total_ms,
        "configureDoneMs": configure_done,
        "firstJobDoneMs": first_job,
        "jobsBeforeConfigureDone": state["jobsDone"],
        "interleaved": first_job != 0.0 && first_job < configure_done,
        "configureError": state["configureError"],
        "configureResult": state["configureResult"],
        "configureConverterMs": state["configureResult"]["converterMs"],
        "driveError": normal.as_ref().err().map(|e| format!("{e:?}")),
    }));

    // 2. Host error: reject one host operation and verify the rejection is
    // observable, then recover by evaluating a normal script.
    let _ = context.eval("globalThis.__lastResult=''");
    context.eval("__hostErrorProbe()").expect("error probe");
    runtime.run_jobs().expect("error probe jobs");
    let mut rejected = false;
    while let Some(capability) = pending.borrow_mut().pop_front() {
        if !rejected {
            rejected = true;
            capability
                .reject(
                    &mut context,
                    Value::String(JsString::try_from_utf8("host EIO").unwrap()),
                )
                .expect("reject host op");
        } else {
            capability
                .resolve(&mut context, Value::Bool(true))
                .expect("resolve host op");
        }
    }
    runtime.run_jobs().expect("error probe resume");
    let error_text = last_result(&context);
    let recovered = context.eval("1 + 1").is_ok();
    results.push(json!({
        "mode": "host_error",
        "hostResult": error_text,
        "evalAfterError": recovered,
    }));

    // 3. Host timeout: reject a pending host operation after a deadline and
    // verify the timeout is observable; then verify engine interrupt
    // (cancellation) and recovery separately.
    let _ = context.eval("globalThis.__lastResult=''");
    context.eval("__hostErrorProbe()").expect("timeout probe");
    runtime.run_jobs().expect("timeout probe jobs");
    std::thread::sleep(Duration::from_millis(100));
    if let Some(capability) = pending.borrow_mut().pop_front() {
        capability
            .reject(
                &mut context,
                Value::String(JsString::try_from_utf8("host timeout after 100 ms").unwrap()),
            )
            .expect("reject timeout");
    }
    runtime.run_jobs().expect("timeout resume");
    let timeout_text = last_result(&context);

    let handle = runtime.interrupt_handle();
    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        handle.interrupt();
    });
    let started = Instant::now();
    let interrupted = context.eval("while (true) {}");
    watchdog.join().ok();
    let interrupted_ms = started.elapsed().as_secs_f64() * 1000.0;
    runtime.interrupt_handle().clear();
    let after = context.eval("1 + 1");
    results.push(json!({
        "mode": "timeout_cancel",
        "hostTimeoutResult": timeout_text,
        "interruptResult": interrupted.as_ref().err().map(|e| format!("{e:?}")),
        "interruptMs": interrupted_ms,
        "afterInterrupt": after.is_ok(),
    }));

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "engine": "quickjs-oxide",
            "version": "0.5.0",
            "runs": results,
        }))
        .unwrap()
    );
}
