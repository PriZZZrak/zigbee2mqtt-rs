//! Minimal converter runtime prototype: one quickjs-oxide worker, an
//! asynchronous Rust host boundary and a bounded per-device scheduler.
//!
//! This crate is deliberately not wired to Zigbee, MQTT or the bridge. It
//! exists to validate scheduling, correlation, lifecycle and metrics against
//! the real upstream converter package and the Node golden corpus.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use quickjs_oxide::engine::api::core::{
    HeapLimits, InterruptHandle, JsString, PromiseCapability, Runtime, RuntimeError,
    RuntimeOptions, Scope, Value,
};
use quickjs_oxide_host::SystemHost;
use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub type JobId = u64;
pub type CorrelationId = u64;

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub package_root: PathBuf,
    pub herdsman_root: PathBuf,
    pub external_path: PathBuf,
    /// Maximum queued jobs per device (one running + this many queued).
    pub max_per_device_queue: usize,
    /// Maximum queued jobs across all devices.
    pub max_total_queue: usize,
    /// Maximum jobs executing JS at once (independent devices).
    pub max_concurrent_jobs: usize,
    pub job_timeout: Duration,
    pub host_timeout: Duration,
    pub heap_limit_bytes: usize,
}

impl RuntimeConfig {
    pub fn new(package_root: impl Into<PathBuf>, herdsman_root: impl Into<PathBuf>) -> Self {
        Self {
            package_root: package_root.into(),
            herdsman_root: herdsman_root.into(),
            external_path: PathBuf::new(),
            max_per_device_queue: 16,
            max_total_queue: 128,
            max_concurrent_jobs: 1,
            job_timeout: Duration::from_secs(10),
            host_timeout: Duration::from_secs(5),
            heap_limit_bytes: 128 << 20,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JobRequest {
    pub device: String,
    pub scenario: Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default)]
pub struct JobMetrics {
    pub queue_wait_ms: f64,
    pub eval_ms: f64,
    pub js_exec_ms: f64,
    pub promise_sched_ms: f64,
    pub host_wait_ms: f64,
    pub total_ms: f64,
    pub host_calls: u64,
    pub gc_collections: u64,
    pub gc_max_pause_ms: f64,
    pub heap_live_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct JobOutcome {
    pub id: JobId,
    pub device: String,
    pub state: JobState,
    pub result: Option<Json>,
    pub error: Option<String>,
    pub metrics: JobMetrics,
}

#[derive(Clone, Debug)]
pub struct HostRequest {
    pub correlation_id: CorrelationId,
    pub job_id: JobId,
    pub device: String,
    pub operation: Json,
}

#[derive(Clone, Debug)]
pub enum HostResult {
    Ok(Json),
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitError {
    Backpressure,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitError {
    Timeout,
    Closed,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeMetrics {
    pub startup_ms: f64,
    pub submitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub late_responses: u64,
    pub host_timeouts: u64,
    pub queue_wait_ms: Vec<f64>,
    pub js_exec_ms: Vec<f64>,
    pub promise_sched_ms: Vec<f64>,
    pub host_wait_ms: Vec<f64>,
    pub total_ms: Vec<f64>,
    pub gc_collections: u64,
    pub gc_max_pause_ms: f64,
    pub heap_live_bytes: u64,
}

pub struct ConverterRuntime {
    tx: mpsc::Sender<Command>,
    host_rx: Mutex<mpsc::Receiver<HostRequest>>,
    outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
    counters: Arc<Mutex<RuntimeMetrics>>,
    states: Arc<Mutex<HashMap<JobId, JobState>>>,
    next_id: AtomicU64,
    interrupt: InterruptHandle,
    join: Option<JoinHandle<()>>,
    shutdown_sent: bool,
}

impl ConverterRuntime {
    pub fn new(config: RuntimeConfig) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let (host_tx, host_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let outcomes = Arc::new((
            Mutex::new(HashMap::<JobId, JobOutcome>::new()),
            Condvar::new(),
        ));
        let counters = Arc::new(Mutex::new(RuntimeMetrics::default()));
        let states = Arc::new(Mutex::new(HashMap::<JobId, JobState>::new()));
        let worker_states = Arc::clone(&states);
        let worker_outcomes = Arc::clone(&outcomes);
        let worker_counters = Arc::clone(&counters);
        let bootstrap = include_str!("../assets/bootstrap.js").to_owned();
        let zcl = include_str!("../assets/zcl.json").to_owned();

        let join = thread::Builder::new()
            .name("converter-js-worker".into())
            .spawn(move || {
                let worker = Worker::new(
                    config,
                    bootstrap,
                    zcl,
                    rx,
                    host_tx,
                    worker_outcomes,
                    worker_counters,
                    worker_states,
                );
                match worker {
                    Ok(mut worker) => {
                        let interrupt = worker.interrupt_handle();
                        ready_tx.send(Ok((worker.startup_ms, interrupt))).ok();
                        worker.run();
                    }
                    Err(error) => {
                        ready_tx.send(Err(error)).ok();
                    }
                }
            })
            .map_err(|error| format!("spawn converter worker: {error}"))?;

        let (startup_ms, interrupt) = ready_rx
            .recv()
            .map_err(|_| "converter worker exited during startup".to_owned())??;
        counters.lock().unwrap().startup_ms = startup_ms;

        Ok(Self {
            tx,
            host_rx: Mutex::new(host_rx),
            outcomes,
            counters,
            states,
            next_id: AtomicU64::new(1),
            interrupt,
            join: Some(join),
            shutdown_sent: false,
        })
    }

    pub fn host_receiver(&self) -> std::sync::MutexGuard<'_, mpsc::Receiver<HostRequest>> {
        self.host_rx.lock().unwrap()
    }

    pub fn respond(&self, correlation_id: CorrelationId, result: HostResult) -> bool {
        self.tx
            .send(Command::HostResponse {
                correlation_id,
                result,
            })
            .is_ok()
    }

    pub fn submit(&self, request: JobRequest) -> Result<JobId, SubmitError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Command::Submit {
                id,
                request,
                reply: reply_tx,
            })
            .map_err(|_| SubmitError::Closed)?;
        match reply_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(id),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(SubmitError::Closed),
        }
    }

    pub fn cancel(&self, id: JobId) -> bool {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        if self
            .tx
            .send(Command::Cancel {
                id,
                reply: reply_tx,
            })
            .is_err()
        {
            return false;
        }
        reply_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or(false)
    }

    pub fn wait(&self, id: JobId, timeout: Duration) -> Result<JobOutcome, WaitError> {
        let (lock, cvar) = &*self.outcomes;
        let deadline = Instant::now() + timeout;
        let mut map = lock.lock().unwrap();
        loop {
            if let Some(outcome) = map.get(&id) {
                return Ok(outcome.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(WaitError::Timeout);
            }
            let (next, _) = cvar
                .wait_timeout(map, deadline.saturating_duration_since(now))
                .unwrap();
            map = next;
        }
    }

    pub fn state(&self, id: JobId) -> Option<JobState> {
        self.states.lock().unwrap().get(&id).copied()
    }

    pub fn metrics(&self) -> RuntimeMetrics {
        self.counters.lock().unwrap().clone()
    }

    pub fn shutdown(&mut self) {
        self.send_shutdown();
        if let Some(join) = self.join.take() {
            join.join().ok();
        }
    }

    fn send_shutdown(&mut self) {
        if self.shutdown_sent {
            return;
        }
        self.shutdown_sent = true;
        self.interrupt.interrupt();
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        if self.tx.send(Command::Shutdown { reply: reply_tx }).is_ok() {
            reply_rx.recv_timeout(Duration::from_secs(5)).ok();
        }
    }
}

impl Drop for ConverterRuntime {
    fn drop(&mut self) {
        self.send_shutdown();
        if let Some(join) = self.join.take() {
            join.join().ok();
        }
    }
}

// ---------------------------------------------------------------------------
// Worker internals
// ---------------------------------------------------------------------------

enum Command {
    Submit {
        id: JobId,
        request: JobRequest,
        reply: mpsc::SyncSender<Result<(), SubmitError>>,
    },
    HostResponse {
        correlation_id: CorrelationId,
        result: HostResult,
    },
    Cancel {
        id: JobId,
        reply: mpsc::SyncSender<bool>,
    },
    Shutdown {
        reply: mpsc::SyncSender<()>,
    },
}

struct PendingHost {
    job_id: JobId,
    capability: PromiseCapability,
    sent_at: Instant,
}

struct PendingHosts {
    next_correlation: u64,
    map: HashMap<CorrelationId, PendingHost>,
}

struct Completion {
    job_id: JobId,
    result: Result<Json, String>,
}

struct JobRecord {
    device: String,
    scenario: Json,
    state: JobState,
    submitted_at: Instant,
    started_at: Option<Instant>,
    metrics: JobMetrics,
    pending_hosts: usize,
}

struct Worker {
    runtime: Runtime,
    context: quickjs_oxide::engine::api::core::Context,
    config: RuntimeConfig,
    rx: mpsc::Receiver<Command>,
    outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
    counters: Arc<Mutex<RuntimeMetrics>>,
    states: Arc<Mutex<HashMap<JobId, JobState>>>,
    jobs: HashMap<JobId, JobRecord>,
    device_queues: HashMap<String, VecDeque<JobId>>,
    ready_devices: VecDeque<String>,
    ready_set: HashSet<String>,
    active_devices: HashMap<String, JobId>,
    total_queued: usize,
    pending: Rc<RefCell<PendingHosts>>,
    completions: Rc<RefCell<Vec<Completion>>>,
    startup_ms: f64,
}

impl Worker {
    fn new(
        config: RuntimeConfig,
        bootstrap: String,
        zcl: String,
        rx: mpsc::Receiver<Command>,
        host_tx: mpsc::Sender<HostRequest>,
        outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
        counters: Arc<Mutex<RuntimeMetrics>>,
        states: Arc<Mutex<HashMap<JobId, JobState>>>,
    ) -> Result<Self, String> {
        let options = RuntimeOptions::from(HeapLimits {
            max_bytes: config.heap_limit_bytes,
        });
        let runtime = Runtime::with_host(SystemHost::default(), options);
        let context = runtime
            .new_context()
            .map_err(|error| format!("context: {error:?}"))?;
        let global = context
            .global_object()
            .map_err(|error| format!("global: {error:?}"))?;

        let pending = Rc::new(RefCell::new(PendingHosts {
            next_correlation: 1,
            map: HashMap::new(),
        }));
        let completions = Rc::new(RefCell::new(Vec::<Completion>::new()));
        let host_pending = Rc::clone(&pending);
        let host_completions = Rc::clone(&completions);
        let host_completions_failed = Rc::clone(&completions);
        let host_tx_op = host_tx.clone();

        context
            .define_function(&global, "__readFile", 1, |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let path = scope.to_js_string(&arg)?.to_utf8_lossy();
                match std::fs::read_to_string(&path) {
                    Ok(text) => Ok(Value::String(
                        JsString::try_from_utf8(&text)
                            .map_err(|_| RuntimeError::Exception(Value::Undefined))?,
                    )),
                    Err(error) => Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        &format!("read {path}: {error}"),
                    )),
                }
            })
            .map_err(|error| format!("__readFile: {error:?}"))?;
        context
            .define_function(&global, "__fileExists", 1, |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let path = scope.to_js_string(&arg)?.to_utf8_lossy();
                Ok(Value::Bool(Path::new(&path).is_file()))
            })
            .map_err(|error| format!("__fileExists: {error:?}"))?;
        context
            .define_function(&global, "__now", 0, |_scope: &mut Scope<'_>| {
                Ok(Value::Float(now_ms()))
            })
            .map_err(|error| format!("__now: {error:?}"))?;
        context
            .define_function(&global, "__log", 1, |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let message = scope.to_js_string(&arg)?.to_utf8_lossy();
                eprintln!("[runtime] {message}");
                Ok(Value::Undefined)
            })
            .map_err(|error| format!("__log: {error:?}"))?;
        context
            .define_function(&global, "__hostOp", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let operation = scope.to_js_string(&arg)?.to_utf8_lossy();
                let parsed: Json = serde_json::from_str(&operation).unwrap_or(Json::Null);
                let job_id = parsed["jobId"].as_u64().unwrap_or(0);
                let device = parsed["device"].as_str().unwrap_or("").to_owned();
                let capability = scope.new_promise()?;
                let promise = capability.promise.clone();
                let correlation_id = {
                    let Ok(mut pending) = host_pending.try_borrow_mut() else {
                        eprintln!("[runtime] pending borrow failed");
                        return Err(scope.throw_error(
                            quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                            "pending map already borrowed",
                        ));
                    };
                    let id = pending.next_correlation;
                    pending.next_correlation = id.saturating_add(1);
                    pending.map.insert(
                        id,
                        PendingHost {
                            job_id,
                            capability,
                            sent_at: Instant::now(),
                        },
                    );
                    id
                };
                let request = HostRequest {
                    correlation_id,
                    job_id,
                    device,
                    operation: parsed,
                };
                if host_tx_op.send(request).is_err() {
                    host_pending.borrow_mut().map.remove(&correlation_id);
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        "host request channel closed",
                    ));
                }
                Ok(Value::Object(promise))
            })
            .map_err(|error| format!("__hostOp: {error:?}"))?;
        context
            .define_function(&global, "__runtimeDone", 2, move |scope: &mut Scope<'_>| {
                let id_arg = scope.arg(0);
                let id_text = scope.to_js_string(&id_arg)?.to_utf8_lossy();
                let result_arg = scope.arg(1);
                let result_text = scope.to_js_string(&result_arg)?.to_utf8_lossy();
                let id: JobId = id_text.parse().unwrap_or(0);
                let result = serde_json::from_str::<Json>(&result_text).map_err(|error| {
                    scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        &format!("invalid job result: {error}"),
                    )
                })?;
                host_completions.borrow_mut().push(Completion {
                    job_id: id,
                    result: Ok(result),
                });
                Ok(Value::Undefined)
            })
            .map_err(|error| format!("__runtimeDone: {error:?}"))?;
        context
            .define_function(
                &global,
                "__runtimeFailed",
                2,
                move |scope: &mut Scope<'_>| {
                    let id_arg = scope.arg(0);
                    let id_text = scope.to_js_string(&id_arg)?.to_utf8_lossy();
                    let error_arg = scope.arg(1);
                    let error_text = scope.to_js_string(&error_arg)?.to_utf8_lossy();
                    let id: JobId = id_text.parse().unwrap_or(0);
                    host_completions_failed.borrow_mut().push(Completion {
                        job_id: id,
                        result: Err(error_text),
                    });
                    Ok(Value::Undefined)
                },
            )
            .map_err(|error| format!("__runtimeFailed: {error:?}"))?;

        let preamble = format!(
            "globalThis.__root = {}; globalThis.__external = {}; globalThis.__zclPath = 'bench:zcl'; globalThis.__herdsmanRoot = {}; globalThis.__zclJson = {}; globalThis.__profileHost = false;",
            serde_json::to_string(&config.package_root.to_string_lossy()).unwrap(),
            serde_json::to_string(&config.external_path.to_string_lossy()).unwrap(),
            serde_json::to_string(
                &config
                    .herdsman_root
                    .canonicalize()
                    .unwrap_or_else(|_| config.herdsman_root.clone())
                    .to_string_lossy()
            )
            .unwrap(),
            serde_json::to_string(&zcl).unwrap(),
        );
        context
            .eval(&preamble)
            .map_err(|error| format!("preamble: {error:?}"))?;

        let started = Instant::now();
        context
            .eval(&bootstrap)
            .map_err(|error| format!("bootstrap: {error:?}"))?;
        runtime
            .run_jobs()
            .map_err(|error| format!("bootstrap jobs: {error:?}"))?;
        context
            .eval("__init()")
            .map_err(|error| format!("init: {error:?}"))?;
        let startup_ms = started.elapsed().as_secs_f64() * 1000.0;

        Ok(Self {
            runtime,
            context,
            config,
            rx,
            outcomes,
            counters,
            states,
            jobs: HashMap::new(),
            device_queues: HashMap::new(),
            ready_devices: VecDeque::new(),
            ready_set: HashSet::new(),
            active_devices: HashMap::new(),
            total_queued: 0,
            pending,
            completions,
            startup_ms,
        })
    }

    fn interrupt_handle(&self) -> InterruptHandle {
        self.runtime.interrupt_handle()
    }

    fn run(&mut self) {
        loop {
            self.start_ready_jobs();
            let timeout = Duration::from_millis(20);
            match self.rx.recv_timeout(timeout) {
                Ok(Command::Submit { id, request, reply }) => {
                    let result = self.enqueue(id, request);
                    reply.send(result).ok();
                }
                Ok(Command::HostResponse {
                    correlation_id,
                    result,
                }) => {
                    self.on_host_response(correlation_id, result);
                }
                Ok(Command::Cancel { id, reply }) => {
                    let cancelled = self.cancel(id);
                    reply.send(cancelled).ok();
                }
                Ok(Command::Shutdown { reply }) => {
                    self.cancel_all();
                    reply.send(()).ok();
                    return;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.check_timeouts();
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
            self.drain_completions();
        }
    }

    fn enqueue(&mut self, id: JobId, request: JobRequest) -> Result<(), SubmitError> {
        let device_queue = self
            .device_queues
            .entry(request.device.clone())
            .or_default();
        if device_queue.len() >= self.config.max_per_device_queue
            || self.total_queued >= self.config.max_total_queue
        {
            return Err(SubmitError::Backpressure);
        }
        let record = JobRecord {
            device: request.device.clone(),
            scenario: request.scenario,
            state: JobState::Pending,
            submitted_at: Instant::now(),
            started_at: None,
            metrics: JobMetrics::default(),
            pending_hosts: 0,
        };
        self.jobs.insert(id, record);
        self.states.lock().unwrap().insert(id, JobState::Pending);
        device_queue.push_back(id);
        self.total_queued += 1;
        self.counters.lock().unwrap().submitted += 1;
        if self.ready_set.insert(request.device.clone()) {
            self.ready_devices.push_back(request.device.clone());
        }
        self.start_ready_jobs();
        Ok(())
    }

    fn start_ready_jobs(&mut self) {
        while self.active_devices.len() < self.config.max_concurrent_jobs {
            let Some(device) = self.ready_devices.pop_front() else {
                break;
            };
            self.ready_set.remove(&device);
            if self.active_devices.contains_key(&device) {
                continue;
            }
            let Some(queue) = self.device_queues.get_mut(&device) else {
                continue;
            };
            let Some(job_id) = queue.pop_front() else {
                continue;
            };
            if queue.is_empty() {
                self.device_queues.remove(&device);
            }
            self.total_queued = self.total_queued.saturating_sub(1);
            self.active_devices.insert(device.clone(), job_id);
            self.start_job(job_id);
        }
    }

    fn start_job(&mut self, job_id: JobId) {
        let Some(job) = self.jobs.get_mut(&job_id) else {
            return;
        };
        job.state = JobState::Running;
        self.states
            .lock()
            .unwrap()
            .insert(job_id, JobState::Running);
        job.started_at = Some(Instant::now());
        job.metrics.queue_wait_ms = job
            .started_at
            .unwrap()
            .duration_since(job.submitted_at)
            .as_secs_f64()
            * 1000.0;
        let spec = job.scenario.to_string();
        let source = format!(
            "__startRuntimeJob({}, {})",
            job_id,
            serde_json::to_string(&spec).unwrap()
        );
        let started = Instant::now();
        if let Err(error) = self.context.eval(&source) {
            self.complete_job(job_id, Err(format!("start job: {error:?}")));
            return;
        }
        job.metrics.eval_ms = started.elapsed().as_secs_f64() * 1000.0;
        if let Err(error) = self.runtime.run_jobs() {
            self.complete_job(job_id, Err(format!("job pump: {error:?}")));
            return;
        }
        {
            let pending = self.pending.borrow();
            if let Some(job) = self.jobs.get_mut(&job_id) {
                job.pending_hosts = pending
                    .map
                    .values()
                    .filter(|pending| pending.job_id == job_id)
                    .count();
                if job.pending_hosts > 0 {
                    job.state = JobState::Waiting;
                }
            }
        }
    }

    fn on_host_response(&mut self, correlation_id: CorrelationId, result: HostResult) {
        let pending = self.pending.borrow_mut().map.remove(&correlation_id);
        let Some(pending) = pending else {
            self.counters.lock().unwrap().late_responses += 1;
            return;
        };
        let Some(job) = self.jobs.get_mut(&pending.job_id) else {
            self.counters.lock().unwrap().late_responses += 1;
            return;
        };
        if job.state == JobState::Cancelled {
            self.counters.lock().unwrap().late_responses += 1;
            return;
        }
        let wait_ms = pending.sent_at.elapsed().as_secs_f64() * 1000.0;
        job.metrics.host_wait_ms += wait_ms;
        job.metrics.host_calls += 1;
        job.pending_hosts = job.pending_hosts.saturating_sub(1);
        if job.pending_hosts == 0 {
            job.state = JobState::Running;
            self.states
                .lock()
                .unwrap()
                .insert(pending.job_id, JobState::Running);
        }
        let started = Instant::now();
        let settle = match result {
            HostResult::Ok(value) => pending
                .capability
                .resolve(&mut self.context, JsonValue::from(value)),
            HostResult::Err(error) => pending.capability.reject(
                &mut self.context,
                Value::String(JsString::try_from_utf8(&error).unwrap()),
            ),
        };
        if let Err(error) = settle {
            self.complete_job(
                pending.job_id,
                Err(format!("settle host promise: {error:?}")),
            );
            return;
        }
        if let Err(error) = self.runtime.run_jobs() {
            self.complete_job(pending.job_id, Err(format!("resume job: {error:?}")));
            return;
        }
        if let Some(job) = self.jobs.get_mut(&pending.job_id) {
            job.metrics.promise_sched_ms += started.elapsed().as_secs_f64() * 1000.0;
        }
        self.start_ready_jobs();
    }

    fn drain_completions(&mut self) {
        let completions: Vec<_> = self.completions.borrow_mut().drain(..).collect();
        for completion in completions {
            self.complete_job(completion.job_id, completion.result);
        }
        self.start_ready_jobs();
    }

    fn complete_job(&mut self, job_id: JobId, result: Result<Json, String>) {
        let Some(mut job) = self.jobs.remove(&job_id) else {
            return;
        };
        let completed_at = Instant::now();
        job.state = match &result {
            Ok(_) => JobState::Completed,
            Err(_) => JobState::Failed,
        };
        if let Ok(value) = &result {
            job.metrics.js_exec_ms = value["converterMs"].as_f64().unwrap_or(0.0);
            job.metrics.host_calls = value["hostCalls"]
                .as_u64()
                .unwrap_or(job.metrics.host_calls);
        }
        job.metrics.total_ms = completed_at.duration_since(job.submitted_at).as_secs_f64() * 1000.0;
        if let Ok(gc) = self.runtime.gc_stats() {
            job.metrics.gc_collections = gc.collections;
            job.metrics.gc_max_pause_ms = gc.max_pause.as_secs_f64() * 1000.0;
            job.metrics.heap_live_bytes = gc.live_bytes as u64;
        }
        self.active_devices.remove(&job.device);
        self.pending
            .borrow_mut()
            .map
            .retain(|_, pending| pending.job_id != job_id);

        self.states.lock().unwrap().insert(job_id, job.state);
        let outcome = JobOutcome {
            id: job_id,
            device: job.device.clone(),
            state: job.state,
            result: result.as_ref().ok().cloned(),
            error: result.as_ref().err().cloned(),
            metrics: job.metrics.clone(),
        };
        {
            let mut metrics = self.counters.lock().unwrap();
            match job.state {
                JobState::Completed => metrics.completed += 1,
                JobState::Failed => metrics.failed += 1,
                JobState::Cancelled => metrics.cancelled += 1,
                _ => {}
            }
            metrics.queue_wait_ms.push(job.metrics.queue_wait_ms);
            metrics.js_exec_ms.push(job.metrics.js_exec_ms);
            metrics.promise_sched_ms.push(job.metrics.promise_sched_ms);
            metrics.host_wait_ms.push(job.metrics.host_wait_ms);
            metrics.total_ms.push(job.metrics.total_ms);
            metrics.gc_collections = metrics.gc_collections.max(job.metrics.gc_collections);
            metrics.gc_max_pause_ms = metrics.gc_max_pause_ms.max(job.metrics.gc_max_pause_ms);
            metrics.heap_live_bytes = metrics.heap_live_bytes.max(job.metrics.heap_live_bytes);
        }
        {
            let (lock, cvar) = &*self.outcomes;
            lock.lock().unwrap().insert(job_id, outcome);
            cvar.notify_all();
        }
        if self.device_queues.contains_key(&job.device) && self.ready_set.insert(job.device.clone())
        {
            self.ready_devices.push_back(job.device.clone());
        }
    }

    fn cancel(&mut self, job_id: JobId) -> bool {
        if let Some(queue) = self
            .device_queues
            .values_mut()
            .find(|queue| queue.iter().any(|id| *id == job_id))
        {
            queue.retain(|id| *id != job_id);
            self.total_queued = self.total_queued.saturating_sub(1);
            self.cancel_job(job_id, "cancelled before start".to_owned());
            return true;
        }
        if let Some(device) = self
            .active_devices
            .iter()
            .find_map(|(device, id)| (*id == job_id).then(|| device.clone()))
        {
            self.active_devices.remove(&device);
            self.cancel_job(job_id, "cancelled while running".to_owned());
            return true;
        }
        false
    }

    fn cancel_job(&mut self, job_id: JobId, reason: String) {
        let Some(mut job) = self.jobs.remove(&job_id) else {
            return;
        };
        job.state = JobState::Cancelled;
        self.states
            .lock()
            .unwrap()
            .insert(job_id, JobState::Cancelled);
        self.pending
            .borrow_mut()
            .map
            .retain(|_, pending| pending.job_id != job_id);
        self.states.lock().unwrap().insert(job_id, job.state);
        let outcome = JobOutcome {
            id: job_id,
            device: job.device.clone(),
            state: JobState::Cancelled,
            result: None,
            error: Some(reason),
            metrics: job.metrics.clone(),
        };
        self.counters.lock().unwrap().cancelled += 1;
        let (lock, cvar) = &*self.outcomes;
        lock.lock().unwrap().insert(job_id, outcome);
        cvar.notify_all();
    }

    fn cancel_all(&mut self) {
        let ids: Vec<_> = self.jobs.keys().copied().collect();
        for id in ids {
            self.cancel_job(id, "runtime shutdown".to_owned());
        }
    }

    fn check_timeouts(&mut self) {
        let now = Instant::now();
        let expired: Vec<JobId> = self
            .jobs
            .iter()
            .filter(|(_, job)| {
                matches!(job.state, JobState::Running | JobState::Waiting)
                    && job.started_at.is_some_and(|started| {
                        now.duration_since(started) >= self.config.job_timeout
                    })
            })
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.cancel_job(id, "job timeout".to_owned());
        }
        let host_timeouts: Vec<CorrelationId> = self
            .pending
            .borrow()
            .map
            .iter()
            .filter(|(_, pending)| now.duration_since(pending.sent_at) >= self.config.host_timeout)
            .map(|(id, _)| *id)
            .collect();
        for correlation_id in host_timeouts {
            self.counters.lock().unwrap().host_timeouts += 1;
            let pending = { self.pending.borrow_mut().map.remove(&correlation_id) };
            if let Some(pending) = pending {
                let _ = pending.capability.reject(
                    &mut self.context,
                    Value::String(JsString::try_from_utf8("host operation timeout").unwrap()),
                );
                let _ = self.runtime.run_jobs();
            }
        }
    }
}

// Convert serde_json to JS values for host responses.
struct JsonValue;

impl JsonValue {
    fn from(value: Json) -> Value {
        match value {
            Json::Null => Value::Null,
            Json::Bool(value) => Value::Bool(value),
            Json::Number(value) => value
                .as_i64()
                .map(|value| Value::Int(value as i32))
                .or_else(|| value.as_f64().map(Value::Float))
                .unwrap_or(Value::Undefined),
            Json::String(value) => Value::String(JsString::try_from_utf8(&value).unwrap()),
            Json::Array(values) => {
                // Arrays are not needed by the current host responses; keep a
                // diagnostic value instead of silently accepting a shape the
                // bridge cannot deliver.
                let _ = values;
                Value::Undefined
            }
            Json::Object(_) => Value::Undefined,
        }
    }
}

fn now_ms() -> f64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

// Keep the worker's host response type independent from serde in the public
// API while using the same JSON representation.
#[allow(dead_code)]
fn _assert_host_result_send() {
    fn assert_send<T: Send>() {}
    assert_send::<HostRequest>();
}
