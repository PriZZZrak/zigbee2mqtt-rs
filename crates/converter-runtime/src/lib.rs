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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use quickjs_oxide::engine::api::core::{
    CallableRef, HeapLimits, InterruptHandle, Js, JsBigInt, JsString, PromiseCapability, Runtime,
    RuntimeError, RuntimeOptions, Scope, Value,
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
    /// Maximum admitted jobs across all devices (queued + running).
    pub max_total_queue: usize,
    /// Maximum jobs executing JS at once (independent devices).
    pub max_concurrent_jobs: usize,
    pub job_timeout: Duration,
    pub host_timeout: Duration,
    pub heap_limit_bytes: usize,
    /// Maximum completed outcomes kept for late `wait` calls.
    pub max_retained_outcomes: usize,
    /// Maximum metric samples kept per vector.
    pub max_metric_samples: usize,
    /// Maximum pending host operations across the worker.
    pub max_pending_host_ops: usize,
    /// Maximum queued host responses.
    pub max_pending_responses: usize,
    pub max_pending_events: usize,
    /// Maximum serialized host request payload.
    pub max_host_payload_bytes: usize,
    pub max_rss_bytes: u64,
    pub rss_poll_interval: Duration,
    pub worker_heartbeat_timeout: Duration,
    pub shutdown_timeout: Duration,
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
            max_retained_outcomes: 256,
            max_metric_samples: 4096,
            max_pending_host_ops: 256,
            max_pending_responses: 256,
            max_pending_events: 1024,
            max_host_payload_bytes: 256 * 1024,
            max_rss_bytes: 128 << 20,
            rss_poll_interval: Duration::from_millis(500),
            worker_heartbeat_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(5),
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

/// Jobs are reported through the runtime event channel. `Started` is sent
/// when the worker begins a job; `Finished` carries the terminal outcome.
/// The bridge consumes these asynchronously and must not call `wait()` on its
/// event loop.
#[derive(Clone, Debug)]
pub enum JobEvent {
    Started { id: JobId, device: String },
    Finished(JobOutcome),
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
pub enum WorkerHealth {
    Starting,
    Running,
    RssExceeded,
    Unresponsive,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelDeviceStatus {
    /// Registered in the shared device-cancel set; the worker processes it on
    /// its next turn regardless of command-queue pressure.
    Accepted,
    /// The worker command channel is closed.
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownStatus {
    Stopped,
    TimedOut,
    AlreadyStopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespondStatus {
    Delivered,
    Backpressure,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelStatus {
    Accepted,
    AlreadyTerminal,
    Unknown,
    Overloaded,
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
    pub host_request_backpressure: u64,
    pub response_backpressure: u64,
    pub payload_rejected: u64,
    pub queue_wait_ms: Vec<f64>,
    pub js_exec_ms: Vec<f64>,
    pub promise_sched_ms: Vec<f64>,
    pub host_wait_ms: Vec<f64>,
    pub total_ms: Vec<f64>,
    pub gc_collections: u64,
    pub gc_max_pause_ms: f64,
    pub heap_live_bytes: u64,
    pub heap_live_peak_bytes: u64,
}

struct AdmissionState {
    total: usize,
    per_device: HashMap<String, usize>,
}

struct Admission {
    max_total: usize,
    max_per_device: usize,
    state: Mutex<AdmissionState>,
}

impl Admission {
    fn new(max_total: usize, max_per_device: usize) -> Self {
        Self {
            max_total,
            max_per_device,
            state: Mutex::new(AdmissionState {
                total: 0,
                per_device: HashMap::new(),
            }),
        }
    }

    fn try_admit(&self, device: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.total >= self.max_total {
            return false;
        }
        let count = state.per_device.get(device).copied().unwrap_or(0);
        if count >= self.max_per_device {
            return false;
        }
        state.total += 1;
        *state.per_device.entry(device.to_owned()).or_insert(0) += 1;
        true
    }

    fn release(&self, device: &str) {
        let mut state = self.state.lock().unwrap();
        state.total = state.total.saturating_sub(1);
        if let Some(count) = state.per_device.get_mut(device) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                state.per_device.remove(device);
            }
        }
    }
}

pub struct ConverterRuntime {
    tx: mpsc::SyncSender<Command>,
    response_tx: mpsc::SyncSender<ResponseEvent>,
    admission: Arc<Admission>,
    host_rx: Mutex<Option<mpsc::Receiver<HostRequest>>>,
    event_rx: Mutex<Option<mpsc::Receiver<JobEvent>>>,
    outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
    counters: Arc<Mutex<RuntimeMetrics>>,
    states: Arc<Mutex<HashMap<JobId, JobState>>>,
    cancel_requests: Arc<Mutex<HashSet<JobId>>>,
    device_cancel_requests: Arc<Mutex<HashSet<String>>>,
    admitted_ids: Arc<Mutex<HashSet<JobId>>>,
    health: Arc<Mutex<WorkerHealth>>,
    last_rss_bytes: Arc<AtomicU64>,
    supervisor_stop: Arc<AtomicBool>,
    supervisor: Option<JoinHandle<()>>,
    config: RuntimeConfig,
    next_id: AtomicU64,
    interrupt: InterruptHandle,
    join: Option<JoinHandle<()>>,
    shutdown_sent: bool,
}

impl ConverterRuntime {
    pub fn new(config: RuntimeConfig) -> Result<Self, String> {
        let (tx, rx) = mpsc::sync_channel(config.max_total_queue.max(1) + 16);
        let (response_tx, response_rx) = mpsc::sync_channel(config.max_pending_responses.max(1));
        let (host_tx, host_rx) = mpsc::sync_channel(config.max_pending_host_ops.max(1));
        let (event_tx, event_rx) = mpsc::sync_channel(config.max_pending_events.max(1));
        let (ready_tx, ready_rx) = mpsc::channel();
        let outcomes = Arc::new((
            Mutex::new(HashMap::<JobId, JobOutcome>::new()),
            Condvar::new(),
        ));
        let counters = Arc::new(Mutex::new(RuntimeMetrics::default()));
        let states = Arc::new(Mutex::new(HashMap::<JobId, JobState>::new()));
        let cancel_requests = Arc::new(Mutex::new(HashSet::<JobId>::new()));
        let worker_cancel_requests = Arc::clone(&cancel_requests);
        let device_cancel_requests = Arc::new(Mutex::new(HashSet::<String>::new()));
        let worker_device_cancel_requests = Arc::clone(&device_cancel_requests);
        let admitted_ids = Arc::new(Mutex::new(HashSet::<JobId>::new()));
        let worker_admitted_ids = Arc::clone(&admitted_ids);
        let health = Arc::new(Mutex::new(WorkerHealth::Starting));
        let worker_health = Arc::clone(&health);
        let last_rss_bytes = Arc::new(AtomicU64::new(0));
        let supervisor_stop = Arc::new(AtomicBool::new(false));
        let heartbeat = Arc::new(Mutex::new(Instant::now()));
        let worker_heartbeat = Arc::clone(&heartbeat);
        let admission = Arc::new(Admission::new(
            config.max_total_queue.max(1),
            config.max_per_device_queue.max(1),
        ));
        let worker_admission = Arc::clone(&admission);
        let worker_states = Arc::clone(&states);
        let worker_outcomes = Arc::clone(&outcomes);
        let worker_counters = Arc::clone(&counters);
        let bootstrap = include_str!("../assets/bootstrap.js").to_owned();
        let zcl = include_str!("../assets/zcl.json").to_owned();
        let supervisor_config = config.clone();
        let stored_config = config.clone();

        let join = thread::Builder::new()
            .name("converter-js-worker".into())
            .spawn(move || {
                let worker = Worker::new(
                    config,
                    bootstrap,
                    zcl,
                    rx,
                    response_rx,
                    host_tx,
                    event_tx,
                    worker_admission,
                    worker_outcomes,
                    worker_counters,
                    worker_states,
                    worker_cancel_requests,
                    worker_device_cancel_requests,
                    worker_admitted_ids,
                    worker_heartbeat,
                    worker_health,
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
        let supervisor_health = Arc::clone(&health);
        let supervisor_rss = Arc::clone(&last_rss_bytes);
        let supervisor_stop_thread = Arc::clone(&supervisor_stop);
        let supervisor_interrupt = interrupt.clone();
        let supervisor_heartbeat = Arc::clone(&heartbeat);
        let supervisor = thread::Builder::new()
            .name("converter-supervisor".into())
            .spawn(move || {
                loop {
                    if supervisor_stop_thread.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(supervisor_config.rss_poll_interval);
                    if supervisor_stop_thread.load(Ordering::Relaxed) {
                        return;
                    }
                    let rss = process_rss_bytes();
                    supervisor_rss.store(rss, Ordering::Relaxed);
                    let heartbeat_age = supervisor_heartbeat.lock().unwrap().elapsed();
                    let mut health = supervisor_health.lock().unwrap();
                    if rss > supervisor_config.max_rss_bytes {
                        *health = WorkerHealth::RssExceeded;
                        drop(health);
                        supervisor_interrupt.interrupt();
                    } else if heartbeat_age > supervisor_config.worker_heartbeat_timeout {
                        *health = WorkerHealth::Unresponsive;
                        drop(health);
                        supervisor_interrupt.interrupt();
                    } else if *health != WorkerHealth::Running {
                        *health = WorkerHealth::Running;
                    }
                }
            })
            .map_err(|error| format!("spawn converter supervisor: {error}"))?;

        Ok(Self {
            tx,
            response_tx,
            admission,
            host_rx: Mutex::new(Some(host_rx)),
            event_rx: Mutex::new(Some(event_rx)),
            outcomes,
            counters,
            states,
            cancel_requests,
            device_cancel_requests,
            admitted_ids,
            health,
            last_rss_bytes,
            supervisor_stop,
            supervisor: Some(supervisor),
            config: stored_config,
            next_id: AtomicU64::new(1),
            interrupt,
            join: Some(join),
            shutdown_sent: false,
        })
    }

    /// Take the single host-request receiver. The bridge transport owns it and
    /// must take a fresh one after `restart()`.
    pub fn take_host_receiver(&self) -> Option<mpsc::Receiver<HostRequest>> {
        self.host_rx.lock().unwrap().take()
    }

    /// Take the single job-event receiver. The dispatcher owns it and must
    /// take a fresh one after `restart()`.
    pub fn take_events(&self) -> Option<mpsc::Receiver<JobEvent>> {
        self.event_rx.lock().unwrap().take()
    }

    /// Non-blocking host response delivery. On backpressure the caller keeps
    /// the already-completed physical result and retries with the same
    /// correlation ID; the pending job remains waiting until it arrives or
    /// times out. A closed response channel means the worker is gone.
    pub fn respond(&self, correlation_id: CorrelationId, result: HostResult) -> RespondStatus {
        match self.response_tx.try_send(ResponseEvent {
            correlation_id,
            result,
        }) {
            Ok(()) => {
                let _ = self.tx.try_send(Command::Wake);
                RespondStatus::Delivered
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.counters.lock().unwrap().response_backpressure += 1;
                RespondStatus::Backpressure
            }
            Err(mpsc::TrySendError::Disconnected(_)) => RespondStatus::Closed,
        }
    }

    /// Non-blocking admission. The bridge thread never waits for the worker.
    pub fn submit(&self, request: JobRequest) -> Result<JobId, SubmitError> {
        let device = request.device.clone();
        if !self.admission.try_admit(&device) {
            return Err(SubmitError::Backpressure);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Register the id before the worker can observe the command, so a
        // very fast completion still removes an existing admission entry
        // instead of leaving a stale one behind.
        self.admitted_ids.lock().unwrap().insert(id);
        match self.tx.try_send(Command::Submit { id, request }) {
            Ok(()) => Ok(id),
            Err(mpsc::TrySendError::Full(_)) => {
                self.admitted_ids.lock().unwrap().remove(&id);
                self.admission.release(&device);
                Err(SubmitError::Backpressure)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.admitted_ids.lock().unwrap().remove(&id);
                self.admission.release(&device);
                Err(SubmitError::Closed)
            }
        }
    }

    /// Non-blocking cancel request. Acceptance is separate from the terminal
    /// state: the outcome is delivered through `wait`.
    pub fn cancel_request(&self, id: JobId) -> CancelStatus {
        if self.outcomes.0.lock().unwrap().contains_key(&id) {
            return CancelStatus::AlreadyTerminal;
        }
        let state = self.states.lock().unwrap().get(&id).copied();
        let Some(state) = state else {
            if self.admitted_ids.lock().unwrap().contains(&id) {
                self.cancel_requests.lock().unwrap().insert(id);
                let _ = self.tx.try_send(Command::Cancel { id });
                return CancelStatus::Accepted;
            }
            return CancelStatus::Unknown;
        };
        self.cancel_requests.lock().unwrap().insert(id);
        let _ = self.tx.try_send(Command::Cancel { id });
        if state == JobState::Running {
            // A synchronous JS slice cannot observe the queue; interrupt the
            // shared execution turn. The worker documents the shared policy.
            self.interrupt.interrupt();
        }
        CancelStatus::Accepted
    }

    /// Cancel all queued/running jobs and timers owned by a device. The
    /// request is registered in a shared set polled by the worker, so a full
    /// command queue cannot drop it; the only failure is a closed worker.
    pub fn cancel_device(&self, device: impl Into<String>) -> CancelDeviceStatus {
        if self.shutdown_sent {
            return CancelDeviceStatus::Closed;
        }
        let device = device.into();
        self.device_cancel_requests
            .lock()
            .unwrap()
            .insert(device.clone());
        match self.tx.try_send(Command::Wake) {
            Ok(()) | Err(mpsc::TrySendError::Full(_)) => CancelDeviceStatus::Accepted,
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.device_cancel_requests.lock().unwrap().remove(&device);
                CancelDeviceStatus::Closed
            }
        }
    }

    /// Compatibility convenience for the earlier probe API.
    pub fn cancel(&self, id: JobId) -> bool {
        matches!(
            self.cancel_request(id),
            CancelStatus::Accepted | CancelStatus::AlreadyTerminal
        )
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

    /// Retained terminal outcome for a job, used to close the tiny race where
    /// a job finishes before the dispatcher registers its waiter.
    pub fn outcome(&self, id: JobId) -> Option<JobOutcome> {
        self.outcomes.0.lock().unwrap().get(&id).cloned()
    }

    pub fn state(&self, id: JobId) -> Option<JobState> {
        self.states.lock().unwrap().get(&id).copied()
    }

    pub fn health(&self) -> WorkerHealth {
        self.health.lock().unwrap().clone()
    }

    pub fn rss_bytes(&self) -> u64 {
        self.last_rss_bytes.load(Ordering::Relaxed)
    }

    pub fn metrics(&self) -> RuntimeMetrics {
        self.counters.lock().unwrap().clone()
    }

    /// Recreate the JS worker and supervisor after an unrecoverable worker
    /// failure or a deliberate reload. The caller must reacquire
    /// `host_receiver()` because the transport channels are replaced.
    /// Stop the old worker with a bounded wait, then create a fresh one.
    /// Returns `TimedOut` if the old worker could not be joined in time; the
    /// detached join helper continues in the background and the OS process
    /// supervisor remains the final recovery boundary.
    pub fn restart(&mut self) -> Result<ShutdownStatus, String> {
        let status = self.shutdown_with_timeout(self.config.shutdown_timeout);
        if status == ShutdownStatus::TimedOut {
            return Ok(status);
        }
        let config = self.config.clone();
        let fresh = ConverterRuntime::new(config)?;
        let old = std::mem::replace(self, fresh);
        drop(old);
        Ok(ShutdownStatus::Stopped)
    }

    /// Bounded shutdown. `JoinHandle::join` has no timeout; a helper thread
    /// performs the join and the caller waits only for the configured bound.
    /// A timed-out worker is detached and must be contained by the OS service
    /// supervisor; this method never claims it was stopped.
    pub fn shutdown(&mut self) -> ShutdownStatus {
        self.shutdown_with_timeout(self.config.shutdown_timeout)
    }

    pub fn shutdown_with_timeout(&mut self, timeout: Duration) -> ShutdownStatus {
        if self.shutdown_sent {
            return ShutdownStatus::AlreadyStopped;
        }
        self.shutdown_sent = true;
        self.supervisor_stop.store(true, Ordering::Relaxed);
        self.interrupt.interrupt();
        let deadline = Instant::now() + timeout;

        let (reply_tx, _reply_rx) = mpsc::sync_channel(1);
        let mut sent = false;
        let mut disconnected = false;
        while Instant::now() < deadline {
            match self.tx.try_send(Command::Shutdown {
                reply: reply_tx.clone(),
            }) {
                Ok(()) => {
                    sent = true;
                    break;
                }
                Err(mpsc::TrySendError::Full(_)) => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    disconnected = true;
                    break;
                }
            }
        }

        let Some(join) = self.join.take() else {
            *self.health.lock().unwrap() = WorkerHealth::Stopped;
            return ShutdownStatus::Stopped;
        };
        if disconnected {
            // The worker already dropped its command receiver, but the join is
            // still bounded: a helper thread performs it and this call never
            // blocks past the deadline.
            let (done_tx, done_rx) = mpsc::channel();
            std::thread::spawn(move || {
                join.join().ok();
                let _ = done_tx.send(());
            });
            let remaining = deadline.saturating_duration_since(Instant::now());
            match done_rx.recv_timeout(remaining) {
                Ok(()) => {
                    if let Some(supervisor) = self.supervisor.take() {
                        supervisor.join().ok();
                    }
                    *self.health.lock().unwrap() = WorkerHealth::Stopped;
                    return ShutdownStatus::Stopped;
                }
                Err(_) => {
                    *self.health.lock().unwrap() = WorkerHealth::Unresponsive;
                    return ShutdownStatus::TimedOut;
                }
            }
        }
        if !sent {
            // The queue stayed full for the whole deadline: detach the join
            // and report the worker as unresponsive, never as stopped.
            std::thread::spawn(move || {
                join.join().ok();
            });
            *self.health.lock().unwrap() = WorkerHealth::Unresponsive;
            return ShutdownStatus::TimedOut;
        }

        let (done_tx, done_rx) = mpsc::channel();
        // Detach the actual join so this call has a hard deadline.
        std::thread::spawn(move || {
            join.join().ok();
            let _ = done_tx.send(());
        });
        let remaining = deadline.saturating_duration_since(Instant::now());
        match done_rx.recv_timeout(remaining) {
            Ok(()) => {
                *self.health.lock().unwrap() = WorkerHealth::Stopped;
                if let Some(supervisor) = self.supervisor.take() {
                    supervisor.join().ok();
                }
                ShutdownStatus::Stopped
            }
            Err(_) => {
                *self.health.lock().unwrap() = WorkerHealth::Unresponsive;
                ShutdownStatus::TimedOut
            }
        }
    }
}

impl Drop for ConverterRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown_with_timeout(Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------------
// Worker internals
// ---------------------------------------------------------------------------

enum Command {
    Submit { id: JobId, request: JobRequest },
    Cancel { id: JobId },
    Wake,
    Shutdown { reply: mpsc::SyncSender<()> },
}

struct ResponseEvent {
    correlation_id: CorrelationId,
    result: HostResult,
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
    generation: u64,
}

#[derive(Clone)]
struct AllowedFiles {
    roots: Vec<PathBuf>,
    files: HashSet<PathBuf>,
}

impl AllowedFiles {
    fn new(config: &RuntimeConfig) -> Self {
        let mut roots = Vec::new();
        for root in [&config.package_root, &config.herdsman_root] {
            if let Ok(root) = root.canonicalize() {
                roots.push(root);
            }
        }
        let mut files = HashSet::new();
        if !config.external_path.as_os_str().is_empty() {
            if let Ok(file) = config.external_path.canonicalize() {
                files.insert(file);
            }
            if let Some(parent) = config
                .external_path
                .parent()
                .and_then(|parent| parent.canonicalize().ok())
            {
                roots.push(parent);
            }
        }
        roots.sort();
        roots.dedup();
        Self { roots, files }
    }

    fn allowed(&self, path: &Path) -> bool {
        if let Ok(canonical) = path.canonicalize() {
            if self.files.contains(&canonical) {
                return true;
            }
            return self.roots.iter().any(|root| canonical.starts_with(root));
        }
        // Nonexistent candidates (loader probes) are allowed only when their
        // existing parent is inside a root; symlink escapes fail here too.
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .is_some_and(|parent| self.roots.iter().any(|root| parent.starts_with(root)))
    }

    fn read(&self, path: &Path) -> std::io::Result<String> {
        if !self.allowed(path) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "path is outside the converter sandbox",
            ));
        }
        std::fs::read_to_string(path)
    }
}

struct TimerEntry {
    id: u64,
    deadline: Instant,
    interval: Option<Duration>,
    callback: CallableRef,
    args: Vec<Value>,
    refed: bool,
    job_id: JobId,
    device: String,
}

struct Timers {
    next_id: u64,
    entries: HashMap<u64, TimerEntry>,
}

impl Timers {
    fn new() -> Self {
        Self {
            next_id: 1,
            entries: HashMap::new(),
        }
    }

    fn insert(
        &mut self,
        callback: CallableRef,
        delay: Duration,
        interval: Option<Duration>,
        args: Vec<Value>,
        job_id: JobId,
        device: String,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.entries.insert(
            id,
            TimerEntry {
                id,
                deadline: Instant::now() + delay,
                interval,
                callback,
                args,
                refed: true,
                job_id,
                device,
            },
        );
        id
    }

    fn due(&mut self, now: Instant) -> Vec<TimerEntry> {
        let due_ids: Vec<u64> = self
            .entries
            .iter()
            .filter(|(_, timer)| timer.deadline <= now)
            .map(|(id, _)| *id)
            .collect();
        due_ids
            .into_iter()
            .filter_map(|id| self.entries.remove(&id))
            .collect()
    }

    fn cancel_job(&mut self, job_id: JobId) {
        self.entries.retain(|_, timer| timer.job_id != job_id);
    }

    fn cancel_device(&mut self, device: &str) {
        self.entries.retain(|_, timer| timer.device != device);
    }
}

struct Watchdog {
    deadline: Arc<(Mutex<Option<Instant>>, Condvar)>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Watchdog {
    fn new(interrupt: InterruptHandle) -> Self {
        let deadline = Arc::new((Mutex::new(None::<Instant>), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_deadline = Arc::clone(&deadline);
        let thread_stop = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name("converter-watchdog".into())
            .spawn(move || {
                let (lock, cvar) = &*thread_deadline;
                loop {
                    let mut guard = lock.lock().unwrap();
                    while !thread_stop.load(Ordering::Relaxed) && guard.is_none() {
                        guard = cvar.wait(guard).unwrap();
                    }
                    if thread_stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let Some(deadline) = *guard else { continue };
                    let now = Instant::now();
                    if now >= deadline {
                        *guard = None;
                        drop(guard);
                        interrupt.interrupt();
                    } else {
                        let (next, _) = cvar.wait_timeout(guard, deadline - now).unwrap();
                        guard = next;
                    }
                }
            })
            .expect("spawn converter watchdog");
        Self {
            deadline,
            stop,
            handle: Some(handle),
        }
    }

    fn arm(&self, deadline: Instant) {
        let (lock, cvar) = &*self.deadline;
        *lock.lock().unwrap() = Some(deadline);
        cvar.notify_all();
    }

    fn disarm(&self) {
        let (lock, cvar) = &*self.deadline;
        *lock.lock().unwrap() = None;
        cvar.notify_all();
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            self.disarm();
            handle.join().ok();
        }
    }
}

struct Worker {
    runtime: Runtime,
    context: quickjs_oxide::engine::api::core::Context,
    config: RuntimeConfig,
    rx: mpsc::Receiver<Command>,
    response_rx: mpsc::Receiver<ResponseEvent>,
    admission: Arc<Admission>,
    outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
    counters: Arc<Mutex<RuntimeMetrics>>,
    states: Arc<Mutex<HashMap<JobId, JobState>>>,
    cancel_requests: Arc<Mutex<HashSet<JobId>>>,
    device_cancel_requests: Arc<Mutex<HashSet<String>>>,
    admitted_ids: Arc<Mutex<HashSet<JobId>>>,
    event_tx: mpsc::SyncSender<JobEvent>,
    pending_events: VecDeque<JobEvent>,
    jobs: HashMap<JobId, JobRecord>,
    device_queues: HashMap<String, VecDeque<JobId>>,
    ready_devices: VecDeque<String>,
    ready_set: HashSet<String>,
    active_devices: HashMap<String, JobId>,
    total_queued: usize,
    pending: Rc<RefCell<PendingHosts>>,
    completions: Rc<RefCell<Vec<Completion>>>,
    active_jobs: Rc<RefCell<HashMap<JobId, u64>>>,
    pending_outcomes: VecDeque<JobId>,
    next_generation: u64,
    watchdog: Watchdog,
    timers: Rc<RefCell<Timers>>,
    heartbeat: Arc<Mutex<Instant>>,
    health: Arc<Mutex<WorkerHealth>>,
    startup_ms: f64,
}

impl Worker {
    fn new(
        config: RuntimeConfig,
        bootstrap: String,
        zcl: String,
        rx: mpsc::Receiver<Command>,
        response_rx: mpsc::Receiver<ResponseEvent>,
        host_tx: mpsc::SyncSender<HostRequest>,
        event_tx: mpsc::SyncSender<JobEvent>,
        admission: Arc<Admission>,
        outcomes: Arc<(Mutex<HashMap<JobId, JobOutcome>>, Condvar)>,
        counters: Arc<Mutex<RuntimeMetrics>>,
        states: Arc<Mutex<HashMap<JobId, JobState>>>,
        cancel_requests: Arc<Mutex<HashSet<JobId>>>,
        device_cancel_requests: Arc<Mutex<HashSet<String>>>,
        admitted_ids: Arc<Mutex<HashSet<JobId>>>,
        heartbeat: Arc<Mutex<Instant>>,
        health: Arc<Mutex<WorkerHealth>>,
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
        let watchdog = Watchdog::new(runtime.interrupt_handle());
        let timers = Rc::new(RefCell::new(Timers::new()));
        let timers_set = Rc::clone(&timers);
        let timers_clear = Rc::clone(&timers);
        let timers_unref = Rc::clone(&timers);
        let timers_ref = Rc::clone(&timers);
        let allowed_files = AllowedFiles::new(&config);
        let read_access = allowed_files.clone();
        let exists_access = allowed_files.clone();

        let pending = Rc::new(RefCell::new(PendingHosts {
            next_correlation: 1,
            map: HashMap::new(),
        }));
        let completions = Rc::new(RefCell::new(Vec::<Completion>::new()));
        let host_pending = Rc::clone(&pending);
        let host_completions = Rc::clone(&completions);
        let host_completions_failed = Rc::clone(&completions);
        let host_tx_op = host_tx.clone();
        let host_metrics = Arc::clone(&counters);
        let max_host_payload_bytes = config.max_host_payload_bytes;
        let max_pending_host_ops = config.max_pending_host_ops;
        let host_active_jobs = Rc::new(RefCell::new(HashMap::<JobId, u64>::new()));
        let active_jobs_for_op = Rc::clone(&host_active_jobs);
        let active_jobs_done = Rc::clone(&host_active_jobs);
        let active_jobs_failed = Rc::clone(&host_active_jobs);

        context
            .define_function(&global, "__readFile", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let path = scope.to_js_string(&arg)?.to_utf8_lossy();
                match read_access.read(Path::new(&path)) {
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
            .define_function(&global, "__fileExists", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let path = scope.to_js_string(&arg)?.to_utf8_lossy();
                let path = Path::new(&path);
                Ok(Value::Bool(exists_access.allowed(path) && path.is_file()))
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
            .define_function(&global, "__setTimer", 4, move |scope: &mut Scope<'_>| {
                let args = scope.args();
                if args.len() < 3 {
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Type,
                        "setTimer expects callback, delay and repeat",
                    ));
                }
                let Value::Object(callback_object) = &args[0] else {
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Type,
                        "timer callback must be a function",
                    ));
                };
                let Some(callback) = scope.as_callable(callback_object)? else {
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Type,
                        "timer callback must be callable",
                    ));
                };
                let delay_ms = scope.to_number(&args[1])?.max(0.0);
                let repeat = matches!(args[2], Value::Bool(true));
                let job_id = args
                    .get(3)
                    .and_then(|value| match value {
                        Value::Int(value) => Some(*value as u64),
                        Value::Float(value) => Some(*value as u64),
                        _ => None,
                    })
                    .unwrap_or(0);
                let device = args
                    .get(4)
                    .and_then(|value| match value {
                        Value::String(value) => Some(value.to_utf8_lossy()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let timer_args = args.iter().skip(5).cloned().collect();
                let delay = Duration::from_secs_f64(delay_ms / 1000.0);
                let interval = repeat.then_some(delay);
                let id = timers_set
                    .borrow_mut()
                    .insert(callback, delay, interval, timer_args, job_id, device);
                Ok(Value::Int(id as i32))
            })
            .map_err(|error| format!("__setTimer: {error:?}"))?;
        context
            .define_function(&global, "__clearTimer", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let id = scope.to_number(&arg)? as u64;
                timers_clear.borrow_mut().entries.remove(&id);
                Ok(Value::Undefined)
            })
            .map_err(|error| format!("__clearTimer: {error:?}"))?;
        context
            .define_function(&global, "__unrefTimer", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let id = scope.to_number(&arg)? as u64;
                if let Some(timer) = timers_unref.borrow_mut().entries.get_mut(&id) {
                    timer.refed = false;
                }
                Ok(Value::Undefined)
            })
            .map_err(|error| format!("__unrefTimer: {error:?}"))?;
        context
            .define_function(&global, "__refTimer", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let id = scope.to_number(&arg)? as u64;
                if let Some(timer) = timers_ref.borrow_mut().entries.get_mut(&id) {
                    timer.refed = true;
                }
                Ok(Value::Undefined)
            })
            .map_err(|error| format!("__refTimer: {error:?}"))?;
        context
            .define_function(&global, "__hostOp", 1, move |scope: &mut Scope<'_>| {
                let arg = scope.arg(0);
                let operation = scope.to_js_string(&arg)?.to_utf8_lossy();
                if operation.len() > max_host_payload_bytes {
                    host_metrics.lock().unwrap().payload_rejected += 1;
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        "host operation payload exceeds limit",
                    ));
                }
                let parsed: Json = serde_json::from_str(&operation).unwrap_or(Json::Null);
                let job_id = parsed["jobId"].as_u64().unwrap_or(0);
                let generation = parsed["generation"].as_u64().unwrap_or(0);
                if active_jobs_for_op.borrow().get(&job_id).copied() != Some(generation) {
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        "job is cancelled or stale",
                    ));
                }
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
                    if pending.map.len() >= max_pending_host_ops {
                        drop(pending);
                        host_metrics.lock().unwrap().host_request_backpressure += 1;
                        return Err(scope.throw_error(
                            quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                            "too many pending host operations",
                        ));
                    }
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
                if host_tx_op.try_send(request).is_err() {
                    host_pending.borrow_mut().map.remove(&correlation_id);
                    host_metrics.lock().unwrap().host_request_backpressure += 1;
                    return Err(scope.throw_error(
                        quickjs_oxide::engine::api::core::NativeErrorKind::Internal,
                        "host request queue is full",
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
                if !active_jobs_done.borrow().contains_key(&id) {
                    return Ok(Value::Undefined);
                }
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
                    if !active_jobs_failed.borrow().contains_key(&id) {
                        return Ok(Value::Undefined);
                    }
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
            response_rx,
            admission,
            outcomes,
            counters,
            states,
            cancel_requests,
            device_cancel_requests,
            admitted_ids,
            event_tx,
            pending_events: VecDeque::new(),
            jobs: HashMap::new(),
            device_queues: HashMap::new(),
            ready_devices: VecDeque::new(),
            ready_set: HashSet::new(),
            active_devices: HashMap::new(),
            total_queued: 0,
            pending,
            completions,
            active_jobs: host_active_jobs,
            pending_outcomes: VecDeque::new(),
            next_generation: 1,
            watchdog,
            timers,
            heartbeat,
            health,
            startup_ms,
        })
    }

    fn interrupt_handle(&self) -> InterruptHandle {
        self.runtime.interrupt_handle()
    }

    fn run(&mut self) {
        *self.health.lock().unwrap() = WorkerHealth::Running;
        loop {
            *self.heartbeat.lock().unwrap() = Instant::now();
            let mut shutdown = None;
            let mut worked = false;
            // Drain a bounded batch of commands; timeouts are checked after
            // every batch, so a continuous stream cannot starve them.
            for _ in 0..64 {
                match self.rx.try_recv() {
                    Ok(Command::Submit { id, request }) => {
                        self.enqueue(id, request);
                        worked = true;
                    }
                    Ok(Command::Cancel { id }) => {
                        self.cancel(id);
                        worked = true;
                    }
                    Ok(Command::Wake) => {}
                    Ok(Command::Shutdown { reply }) => {
                        shutdown = Some(reply);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
            // Host responses have their own unbounded queue; bound the drain
            // per turn so a burst cannot starve timers either.
            for _ in 0..256 {
                match self.response_rx.try_recv() {
                    Ok(response) => {
                        self.on_host_response(response.correlation_id, response.result);
                        worked = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
            self.process_cancellations();
            self.check_timeouts();
            self.fire_timers();
            self.drain_completions();
            self.start_ready_jobs();
            self.flush_events();
            if let Some(reply) = shutdown {
                self.cancel_all();
                reply.send(()).ok();
                return;
            }
            if !worked {
                match self.rx.recv_timeout(Duration::from_millis(2)) {
                    Ok(Command::Submit { id, request }) => {
                        self.enqueue(id, request);
                    }
                    Ok(Command::Cancel { id }) => {
                        self.cancel(id);
                    }
                    Ok(Command::Wake) => {}
                    Ok(Command::Shutdown { reply }) => {
                        self.cancel_all();
                        reply.send(()).ok();
                        return;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    }

    fn enqueue(&mut self, id: JobId, request: JobRequest) {
        let record = JobRecord {
            device: request.device.clone(),
            scenario: request.scenario,
            state: JobState::Pending,
            submitted_at: Instant::now(),
            started_at: None,
            metrics: JobMetrics::default(),
            pending_hosts: 0,
            generation: 0,
        };
        self.jobs.insert(id, record);
        self.states.lock().unwrap().insert(id, JobState::Pending);
        self.device_queues
            .entry(request.device.clone())
            .or_default()
            .push_back(id);
        self.total_queued += 1;
        self.counters.lock().unwrap().submitted += 1;
        if self.ready_set.insert(request.device.clone()) {
            self.ready_devices.push_back(request.device.clone());
        }
        self.start_ready_jobs();
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
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        let spec = {
            let Some(job) = self.jobs.get_mut(&job_id) else {
                return;
            };
            job.state = JobState::Running;
            job.started_at = Some(Instant::now());
            job.metrics.queue_wait_ms = job
                .started_at
                .unwrap()
                .duration_since(job.submitted_at)
                .as_secs_f64()
                * 1000.0;
            job.generation = generation;
            let mut scenario = job.scenario.clone();
            if let Some(object) = scenario.as_object_mut() {
                object.insert("__deviceKey".to_owned(), Json::String(job.device.clone()));
            }
            scenario.to_string()
        };
        self.states
            .lock()
            .unwrap()
            .insert(job_id, JobState::Running);
        if let Some(job) = self.jobs.get(&job_id) {
            self.emit_event(JobEvent::Started {
                id: job_id,
                device: job.device.clone(),
            });
        }
        self.active_jobs.borrow_mut().insert(job_id, generation);
        let source = format!(
            "__startRuntimeJob({}, {}, {})",
            job_id,
            generation,
            serde_json::to_string(&spec).unwrap()
        );
        let started = Instant::now();
        self.arm_watchdog();
        let eval_result = self.context.eval(&source);
        if let Err(error) = eval_result {
            self.watchdog.disarm();
            self.handle_pump_error(job_id, &error, "start job");
            return;
        }
        if let Some(job) = self.jobs.get_mut(&job_id) {
            job.metrics.eval_ms = started.elapsed().as_secs_f64() * 1000.0;
        }
        let pump_result = self.runtime.run_jobs();
        self.watchdog.disarm();
        if let Err(error) = pump_result {
            self.handle_pump_error(job_id, &error, "job pump");
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
        self.arm_watchdog();
        let settle = match result {
            HostResult::Ok(value) => {
                let value = json_to_js(&self.runtime, &mut self.context, &value);
                match value {
                    Ok(value) => pending.capability.resolve(&mut self.context, value),
                    Err(error) => {
                        self.complete_job(
                            pending.job_id,
                            Err(format!("host value bridge: {error:?}")),
                        );
                        return;
                    }
                }
            }
            HostResult::Err(error) => pending.capability.reject(
                &mut self.context,
                Value::String(JsString::try_from_utf8(&error).unwrap()),
            ),
        };
        if let Err(error) = settle {
            self.watchdog.disarm();
            self.handle_pump_error(pending.job_id, &error, "settle host promise");
            return;
        }
        let pump_result = self.runtime.run_jobs();
        self.watchdog.disarm();
        if let Err(error) = pump_result {
            self.handle_pump_error(pending.job_id, &error, "resume job");
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

    fn arm_watchdog(&self) {
        let deadline = self
            .jobs
            .values()
            .filter(|job| matches!(job.state, JobState::Running | JobState::Waiting))
            .filter_map(|job| {
                job.started_at
                    .map(|started| started + self.config.job_timeout)
            })
            .min();
        if let Some(deadline) = deadline {
            self.watchdog.arm(deadline);
        }
    }

    fn handle_pump_error(&mut self, job_id: JobId, error: &RuntimeError, context: &str) {
        if matches!(error, RuntimeError::Interrupted) {
            self.runtime.interrupt_handle().clear();
            let ids: Vec<JobId> = self.active_devices.values().copied().collect();
            let requested = self.cancel_requests.lock().unwrap().clone();
            for id in ids {
                let reason = if requested.contains(&id) {
                    "cancelled during synchronous JS"
                } else {
                    "JS execution interrupted (shared watchdog/cancellation)"
                };
                self.finish_job(id, JobState::Cancelled, None, Some(reason.to_owned()));
            }
            self.cancel_requests.lock().unwrap().clear();
        } else {
            self.complete_job(job_id, Err(format!("{context}: {error:?}")));
        }
    }

    fn complete_job(&mut self, job_id: JobId, result: Result<Json, String>) {
        let (state, error) = match &result {
            Ok(_) => (JobState::Completed, None),
            Err(error) => (JobState::Failed, Some(error.clone())),
        };
        self.finish_job(job_id, state, result.ok(), error);
    }

    fn finish_job(
        &mut self,
        job_id: JobId,
        final_state: JobState,
        result: Option<Json>,
        error: Option<String>,
    ) {
        let Some(mut job) = self.jobs.remove(&job_id) else {
            return;
        };
        let completed_at = Instant::now();
        job.state = final_state;
        job.metrics.total_ms = completed_at.duration_since(job.submitted_at).as_secs_f64() * 1000.0;
        if let Some(value) = &result {
            job.metrics.js_exec_ms = value["converterMs"].as_f64().unwrap_or(0.0);
            job.metrics.host_calls = value["hostCalls"]
                .as_u64()
                .unwrap_or(job.metrics.host_calls);
        }
        if let Ok(gc) = self.runtime.gc_stats() {
            job.metrics.gc_collections = gc.collections;
            job.metrics.gc_max_pause_ms = gc.max_pause.as_secs_f64() * 1000.0;
            job.metrics.heap_live_bytes = gc.live_bytes as u64;
        }

        // Every terminal path releases the same resources: the active
        // device slot, the pending host capabilities, the generation guard,
        // admission credit and the live state entry.
        if self
            .active_devices
            .get(&job.device)
            .is_some_and(|active| *active == job_id)
        {
            self.active_devices.remove(&job.device);
        }
        self.pending
            .borrow_mut()
            .map
            .retain(|_, pending| pending.job_id != job_id);
        self.active_jobs.borrow_mut().remove(&job_id);
        self.admission.release(&job.device);
        self.admitted_ids.lock().unwrap().remove(&job_id);
        self.states.lock().unwrap().remove(&job_id);
        if job.state == JobState::Cancelled {
            self.timers.borrow_mut().cancel_job(job_id);
        }

        let outcome = JobOutcome {
            id: job_id,
            device: job.device.clone(),
            state: job.state,
            result,
            error,
            metrics: job.metrics.clone(),
        };
        self.emit_event(JobEvent::Finished(outcome.clone()));
        {
            let mut metrics = self.counters.lock().unwrap();
            match job.state {
                JobState::Completed => metrics.completed += 1,
                JobState::Failed => metrics.failed += 1,
                JobState::Cancelled => metrics.cancelled += 1,
                _ => {}
            }
            push_sample(
                &mut metrics.queue_wait_ms,
                job.metrics.queue_wait_ms,
                self.config.max_metric_samples,
            );
            push_sample(
                &mut metrics.js_exec_ms,
                job.metrics.js_exec_ms,
                self.config.max_metric_samples,
            );
            push_sample(
                &mut metrics.promise_sched_ms,
                job.metrics.promise_sched_ms,
                self.config.max_metric_samples,
            );
            push_sample(
                &mut metrics.host_wait_ms,
                job.metrics.host_wait_ms,
                self.config.max_metric_samples,
            );
            push_sample(
                &mut metrics.total_ms,
                job.metrics.total_ms,
                self.config.max_metric_samples,
            );
            metrics.gc_collections = metrics.gc_collections.max(job.metrics.gc_collections);
            metrics.gc_max_pause_ms = metrics.gc_max_pause_ms.max(job.metrics.gc_max_pause_ms);
            metrics.heap_live_bytes = job.metrics.heap_live_bytes;
            metrics.heap_live_peak_bytes = metrics
                .heap_live_peak_bytes
                .max(job.metrics.heap_live_bytes);
        }
        {
            let (lock, cvar) = &*self.outcomes;
            lock.lock().unwrap().insert(job_id, outcome);
            cvar.notify_all();
        }
        self.pending_outcomes.push_back(job_id);
        while self.pending_outcomes.len() > self.config.max_retained_outcomes {
            if let Some(oldest) = self.pending_outcomes.pop_front() {
                self.outcomes.0.lock().unwrap().remove(&oldest);
            }
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
            self.finish_job(
                job_id,
                JobState::Cancelled,
                None,
                Some("cancelled before start".to_owned()),
            );
            return true;
        }
        if self.active_devices.values().any(|id| *id == job_id) {
            self.finish_job(
                job_id,
                JobState::Cancelled,
                None,
                Some("cancelled while running".to_owned()),
            );
            return true;
        }
        false
    }

    fn cancel_job(&mut self, job_id: JobId, reason: String) {
        self.finish_job(job_id, JobState::Cancelled, None, Some(reason));
    }

    fn cancel_all(&mut self) {
        let ids: Vec<_> = self.jobs.keys().copied().collect();
        for id in ids {
            self.finish_job(
                id,
                JobState::Cancelled,
                None,
                Some("runtime shutdown".to_owned()),
            );
        }
        self.timers.borrow_mut().entries.clear();
        self.device_queues.clear();
        self.ready_devices.clear();
        self.ready_set.clear();
        self.active_devices.clear();
        self.total_queued = 0;
    }

    fn cancel_device(&mut self, device: &str) {
        let queued: Vec<JobId> = self
            .device_queues
            .remove(device)
            .map(|queue| queue.into_iter().collect())
            .unwrap_or_default();
        self.total_queued = self.total_queued.saturating_sub(queued.len());
        for id in queued {
            self.finish_job(
                id,
                JobState::Cancelled,
                None,
                Some("device removed".to_owned()),
            );
        }
        if let Some(id) = self.active_devices.get(device).copied() {
            self.finish_job(
                id,
                JobState::Cancelled,
                None,
                Some("device removed".to_owned()),
            );
        }
        self.timers.borrow_mut().cancel_device(device);
        self.ready_set.remove(device);
    }

    fn process_cancellations(&mut self) {
        let requested: Vec<JobId> = self.cancel_requests.lock().unwrap().drain().collect();
        for id in requested {
            self.cancel(id);
        }
        let devices: Vec<String> = self
            .device_cancel_requests
            .lock()
            .unwrap()
            .drain()
            .collect();
        for device in devices {
            self.cancel_device(&device);
        }
    }

    fn emit_event(&mut self, event: JobEvent) {
        self.pending_events.push_back(event);
        self.flush_events();
    }

    fn flush_events(&mut self) {
        while let Some(event) = self.pending_events.front().cloned() {
            match self.event_tx.try_send(event) {
                Ok(()) => {
                    self.pending_events.pop_front();
                }
                Err(mpsc::TrySendError::Full(_)) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    self.pending_events.clear();
                    break;
                }
            }
        }
    }

    fn fire_timers(&mut self) {
        let due = self.timers.borrow_mut().due(Instant::now());
        if due.is_empty() {
            return;
        }
        for timer in due {
            if let Some(interval) = timer.interval {
                self.timers.borrow_mut().entries.insert(
                    timer.id,
                    TimerEntry {
                        id: timer.id,
                        deadline: Instant::now() + interval,
                        interval: Some(interval),
                        callback: timer.callback.clone(),
                        args: timer.args.clone(),
                        refed: timer.refed,
                        job_id: timer.job_id,
                        device: timer.device.clone(),
                    },
                );
            }
            let call = self
                .context
                .call(&timer.callback, Value::Undefined, &timer.args);
            if let Err(error) = call {
                eprintln!("[runtime] timer callback error: {error:?}");
            }
        }
        if let Err(error) = self.runtime.run_jobs() {
            eprintln!("[runtime] timer job error: {error:?}");
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

// Recursive JSON -> JavaScript value conversion. Integers that fit i32 stay
// `Int`; larger exact binary64 values stay `Float`; integers beyond 2^53 use
// BigInt so host data is never silently truncated. Arrays and objects are
// built recursively.
fn json_to_js(
    runtime: &Runtime,
    context: &mut quickjs_oxide::engine::api::core::Context,
    value: &Json,
) -> Result<Value, RuntimeError> {
    match value {
        Json::Null => Ok(Value::Null),
        Json::Bool(value) => Ok(Value::Bool(*value)),
        Json::Number(number) => number_to_js(number),
        Json::String(value) => JsString::try_from_utf8(value)
            .map(Value::String)
            .map_err(|_| RuntimeError::Internal("invalid host JSON string".to_owned())),
        Json::Array(values) => {
            let mut converted = Vec::with_capacity(values.len());
            for value in values {
                converted.push(json_to_js(runtime, context, value)?);
            }
            context.new_array_from_values(converted).map(Value::Object)
        }
        Json::Object(values) => {
            let object = context.new_object()?;
            for (key, value) in values {
                let key = runtime.intern_property_key(key)?;
                let value = json_to_js(runtime, context, value)?;
                if !context.set_property(&object, &key, value)? {
                    return Err(RuntimeError::Internal(
                        "host object property was not writable".to_owned(),
                    ));
                }
            }
            Ok(Value::Object(object))
        }
    }
}

fn number_to_js(number: &serde_json::Number) -> Result<Value, RuntimeError> {
    if let Some(value) = number.as_i64() {
        if (i32::MIN as i64..=i32::MAX as i64).contains(&value) {
            return Ok(Value::Int(value as i32));
        }
        if value.unsigned_abs() <= (1u64 << 53) {
            return Ok(Value::Float(value as f64));
        }
        return JsBigInt::parse_js_string(&value.to_string())
            .map(Value::BigInt)
            .map_err(|error| RuntimeError::Internal(format!("host BigInt: {error}")));
    }
    if let Some(value) = number.as_u64() {
        if value <= i32::MAX as u64 {
            return Ok(Value::Int(value as i32));
        }
        if value <= (1u64 << 53) {
            return Ok(Value::Float(value as f64));
        }
        return JsBigInt::parse_js_string(&value.to_string())
            .map(Value::BigInt)
            .map_err(|error| RuntimeError::Internal(format!("host BigInt: {error}")));
    }
    number
        .as_f64()
        .map(Value::Float)
        .ok_or_else(|| RuntimeError::Internal("invalid host JSON number".to_owned()))
}

fn push_sample(values: &mut Vec<f64>, value: f64, limit: usize) {
    if values.len() >= limit {
        values.remove(0);
    }
    values.push(value);
}

fn process_rss_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
                * 1024;
        }
    }
    0
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn host_numbers_do_not_truncate() {
        let value = number_to_js(json!(42).as_number().unwrap()).unwrap();
        assert!(matches!(value, Value::Int(42)));
        let value = number_to_js(json!(4_294_967_296i64).as_number().unwrap()).unwrap();
        assert!(matches!(value, Value::Float(value) if value == 4_294_967_296.0));
        let value = number_to_js(json!(9_007_199_254_740_993i64).as_number().unwrap()).unwrap();
        assert!(matches!(value, Value::BigInt(_)));
        let value = number_to_js(json!(u64::MAX).as_number().unwrap()).unwrap();
        assert!(matches!(value, Value::BigInt(_)));
    }

    #[test]
    fn admission_is_bounded_per_device_and_total() {
        let admission = Admission::new(2, 1);
        assert!(admission.try_admit("a"));
        assert!(!admission.try_admit("a"));
        assert!(admission.try_admit("b"));
        assert!(!admission.try_admit("c"));
        admission.release("a");
        assert!(admission.try_admit("c"));
    }

    #[test]
    fn host_value_bridge_preserves_objects_and_arrays() {
        let runtime = Runtime::new();
        let mut context = runtime.new_context().expect("context");
        let value = json!({"a": [1, true, "x"], "n": 9007199254740993i64});
        let converted = json_to_js(&runtime, &mut context, &value).expect("convert");
        let rendered = runtime.print_value(&converted).expect("print");
        assert!(rendered.contains("a"));
        assert!(rendered.contains("9007199254740993"));
    }

    #[test]
    fn metric_samples_are_capped() {
        let mut values = Vec::new();
        for index in 0..10 {
            push_sample(&mut values, index as f64, 3);
        }
        assert_eq!(values, vec![7.0, 8.0, 9.0]);
    }
}
