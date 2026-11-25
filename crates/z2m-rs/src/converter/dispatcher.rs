// Production converter dispatcher for the Rust bridge.
//
// The dispatcher owns one `converter_runtime::ConverterRuntime` worker and
// exposes asynchronous operations to the existing extensions. Job completion
// is delivered through the runtime event channel; no bridge code calls the
// runtime's blocking `wait()`.
//
// Definitions prepared by the runtime are exposed as ordinary
// `zigbee_converters::Definition` values, so Receive/Publish/Configure/OnEvent
// keep their existing upstream flow. Built-in and external JS definitions use
// the same adapter and the same worker.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use converter_runtime::{
    CancelDeviceStatus, ConverterRuntime, JobEvent, JobId, JobOutcome, JobRequest, JobState,
    RuntimeConfig, ShutdownStatus, SubmitError,
};
use serde_json::{json, Map, Value};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;
use zigbee_apiarist::controller::model::endpoint::Endpoint as ZhEndpoint;
use zigbee_converters::{
    ConvertData, ConvertMeta, Definition, DefinitionRef, FromZigbeeConverter, OnEventEvent,
    PublishCallback, ToMeta, ToTarget, ToZigbeeConverter, ToZigbeeResult,
};

use converter_artifact::{
    DeviceEndpoint, DeviceProfile, DeviceType, LazyConfig, LazyLoader, ModuleFetcher, StartupReport,
};

use crate::converter::transport::{ConverterTransport, OfflineMockTransport};
use crate::zigbee::Zigbee;

type PendingResult = Result<JobOutcome, String>;

#[derive(Clone, Debug)]
pub struct DispatcherConfig {
    pub runtime: RuntimeConfig,
    /// Try the configured external converter file before the package database.
    pub prefer_external: bool,
}

impl DispatcherConfig {
    pub fn new(
        package_root: impl Into<std::path::PathBuf>,
        herdsman_root: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            runtime: RuntimeConfig::new(package_root, herdsman_root),
            prefer_external: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DefinitionMetadata {
    pub model: String,
    pub vendor: String,
    pub description: String,
    pub zigbee_model: Vec<String>,
    pub endpoint: Vec<(String, u8)>,
    pub version: String,
    from_zigbee: Vec<(String, Vec<String>)>,
    to_zigbee: Vec<Vec<String>>,
    has_configure: bool,
    has_on_event: bool,
}

#[derive(Clone, Debug)]
pub struct PreparedDefinition {
    pub definition: DefinitionRef,
    pub metadata: DefinitionMetadata,
    pub source: String,
    /// Model key used to locate the definition in the runtime (the device
    /// modelID / zigbeeModel that matched the index).
    pub lookup_model: String,
}

pub struct ConverterDispatcher {
    runtime: Arc<Mutex<ConverterRuntime>>,
    pending: Arc<Mutex<HashMap<JobId, oneshot::Sender<PendingResult>>>>,
    transport: Arc<dyn ConverterTransport>,
    config: DispatcherConfig,
    generation: AtomicU64,
    event_task: Mutex<Option<JoinHandle<()>>>,
    host_task: Mutex<Option<JoinHandle<()>>>,
    lazy: Mutex<Option<Arc<LazyLoader>>>,
    lazy_fetcher: Mutex<Option<Arc<dyn ModuleFetcher>>>,
    runtime_root: Mutex<Option<std::path::PathBuf>>,
    startup_report: Mutex<Option<StartupReport>>,
}

impl ConverterDispatcher {
    /// Start the JS worker off the Tokio reactor and install the event/host
    /// forwarding channels.
    pub async fn start(
        config: DispatcherConfig,
        transport: Arc<dyn ConverterTransport>,
    ) -> Result<Arc<Self>, String> {
        let runtime_config = config.runtime.clone();
        let runtime = tokio::task::spawn_blocking(move || ConverterRuntime::new(runtime_config))
            .await
            .map_err(|error| format!("converter runtime task: {error}"))??;
        let dispatcher = Arc::new(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            transport,
            config,
            generation: AtomicU64::new(1),
            event_task: Mutex::new(None),
            host_task: Mutex::new(None),
            lazy: Mutex::new(None),
            lazy_fetcher: Mutex::new(None),
            runtime_root: Mutex::new(None),
            startup_report: Mutex::new(None),
        });
        dispatcher.install_channels()?;
        Ok(dispatcher)
    }

    pub async fn with_offline_transport(config: DispatcherConfig) -> Result<Arc<Self>, String> {
        let transport: Arc<dyn ConverterTransport> = Arc::new(OfflineMockTransport::new());
        Self::start(config, transport).await
    }

    /// Start the dispatcher from a trusted artifact cache and the existing
    /// device database. Only modules needed by paired devices are materialized,
    /// into a new immutable runtime generation; the full device catalog is
    /// never copied and the previous generation is left intact for a worker
    /// that is still shutting down.
    pub async fn start_lazy(
        config: DispatcherConfig,
        transport: Arc<dyn ConverterTransport>,
        lazy_config: LazyConfig,
        database: std::path::PathBuf,
        fetcher: Arc<dyn ModuleFetcher>,
        runtime_base: std::path::PathBuf,
        allow_network: bool,
    ) -> Result<Arc<Self>, String> {
        let artifact_dir = lazy_config.artifact_dir.clone();
        let fetcher_for_task = Arc::clone(&fetcher);
        let prepared = tokio::task::spawn_blocking(move || {
            let lazy = LazyLoader::open(lazy_config)?;
            let report = lazy.startup(&database, fetcher_for_task.as_ref(), allow_network)?;
            let root = lazy.materialize_runtime_package(&artifact_dir, &report, &runtime_base)?;
            Ok::<_, converter_artifact::ArtifactError>((Arc::new(lazy), report, root))
        })
        .await
        .map_err(|error| format!("lazy loader task: {error}"))?
        .map_err(|error| error.to_string())?;
        let (lazy, report, root) = prepared;
        let mut config = config;
        config.runtime.package_root = root.clone();
        let dispatcher = Self::start(config, transport).await?;
        *dispatcher.startup_report.lock().unwrap() = Some(report);
        *dispatcher.lazy.lock().unwrap() = Some(lazy);
        *dispatcher.lazy_fetcher.lock().unwrap() = Some(fetcher);
        *dispatcher.runtime_root.lock().unwrap() = Some(root);
        Ok(dispatcher)
    }

    pub fn startup_report(&self) -> Option<StartupReport> {
        self.startup_report.lock().unwrap().clone()
    }

    /// Active immutable runtime package root used by the running worker.
    pub fn runtime_root(&self) -> Option<std::path::PathBuf> {
        self.runtime_root.lock().unwrap().clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn transport(&self) -> Arc<dyn ConverterTransport> {
        self.transport.clone()
    }

    pub fn metrics(&self) -> converter_runtime::RuntimeMetrics {
        self.runtime.lock().unwrap().metrics()
    }

    fn install_channels(self: &Arc<Self>) -> Result<(), String> {
        let (events, host_receiver) = {
            let runtime = self.runtime.lock().unwrap();
            (
                runtime
                    .take_events()
                    .ok_or_else(|| "converter runtime events already taken".to_owned())?,
                runtime
                    .take_host_receiver()
                    .ok_or_else(|| "converter runtime host receiver already taken".to_owned())?,
            )
        };

        // The runtime is std-channel based. A dedicated forwarding thread keeps
        // the blocking receive off the Tokio reactor.
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::Builder::new()
            .name("converter-events".into())
            .spawn(move || {
                while let Ok(event) = events.recv() {
                    if event_tx.send(event).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("spawn converter event forwarder: {error}"))?;

        let pending = Arc::clone(&self.pending);
        let event_task = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let JobEvent::Finished(outcome) = event {
                    if let Some(sender) = pending.lock().unwrap().remove(&outcome.id) {
                        let _ = sender.send(Ok(outcome));
                    }
                }
            }
        });
        *self.event_task.lock().unwrap() = Some(event_task);

        // Host requests are forwarded on a bounded Tokio channel; the
        // forwarding thread blocks with backpressure instead of the reactor.
        let max_host_ops = self.config.runtime.max_pending_host_ops.max(1);
        let (host_tx, mut host_rx) = tokio::sync::mpsc::channel(max_host_ops);
        std::thread::Builder::new()
            .name("converter-host".into())
            .spawn(move || {
                while let Ok(request) = host_receiver.recv() {
                    if host_tx.blocking_send(request).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("spawn converter host forwarder: {error}"))?;

        let runtime = Arc::clone(&self.runtime);
        let transport = Arc::clone(&self.transport);
        let host_task = tokio::spawn(async move {
            while let Some(request) = host_rx.recv().await {
                let result = transport.execute(&request).await;
                let mut retries = 0;
                loop {
                    let status = {
                        let runtime = runtime.lock().unwrap();
                        runtime.respond(request.correlation_id, result.clone())
                    };
                    match status {
                        converter_runtime::RespondStatus::Delivered
                        | converter_runtime::RespondStatus::Closed => break,
                        converter_runtime::RespondStatus::Backpressure => {
                            retries += 1;
                            if retries > 10_000 {
                                eprintln!(
                                    "[converter] response backpressure persisted for correlation {}",
                                    request.correlation_id
                                );
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                    }
                }
            }
        });
        *self.host_task.lock().unwrap() = Some(host_task);
        Ok(())
    }

    fn abort_tasks(&self) {
        if let Some(task) = self.event_task.lock().unwrap().take() {
            task.abort();
        }
        if let Some(task) = self.host_task.lock().unwrap().take() {
            task.abort();
        }
    }

    fn fail_pending(&self, reason: &str) {
        let mut pending = self.pending.lock().unwrap();
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(reason.to_owned()));
        }
    }

    async fn dispatch(&self, device: &str, scenario: Value) -> Result<JobOutcome, String> {
        let (sender, receiver) = oneshot::channel();
        let id = {
            let runtime = self.runtime.lock().unwrap();
            let id = runtime
                .submit(JobRequest {
                    device: device.to_owned(),
                    scenario,
                })
                .map_err(|error| match error {
                    SubmitError::Backpressure => "converter runtime backpressure".to_owned(),
                    SubmitError::Closed => "converter runtime closed".to_owned(),
                })?;
            self.pending.lock().unwrap().insert(id, sender);
            // A very fast job may finish before the pending entry or the event
            // task observes it; the retained outcome closes that race.
            if let Some(outcome) = runtime.outcome(id) {
                self.pending.lock().unwrap().remove(&id);
                return Ok(outcome);
            }
            id
        };

        let timeout = self.config.runtime.job_timeout + self.config.runtime.host_timeout;
        match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(Ok(outcome))) => Ok(outcome),
            Ok(Ok(Err(reason))) => Err(reason),
            Ok(Err(_)) => Err("converter event channel closed".to_owned()),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                let _ = self.runtime.lock().unwrap().cancel(id);
                Err("converter dispatch timeout".to_owned())
            }
        }
    }

    fn completed_result(outcome: JobOutcome) -> Result<Value, String> {
        match outcome.state {
            JobState::Completed => Ok(outcome.result.unwrap_or(Value::Null)),
            JobState::Cancelled => Err(outcome
                .error
                .unwrap_or_else(|| "converter job cancelled".to_owned())),
            JobState::Failed => Err(outcome
                .error
                .unwrap_or_else(|| "converter job failed".to_owned())),
            state => Err(format!("converter job ended in state {state:?}")),
        }
    }

    fn definition_spec(source: &str, model: &str) -> Value {
        json!({"source": source, "model": model})
    }

    fn base_scenario(&self, source: &str, model: &str) -> Map<String, Value> {
        let mut scenario = Map::new();
        scenario.insert(
            "definition".to_owned(),
            Self::definition_spec(source, model),
        );
        scenario.insert("__useHostReadResult".to_owned(), json!(true));
        scenario
    }

    /// Prepare (evaluate and merge) a definition in the JS realm and expose it
    /// as an ordinary Definition for the existing extensions.
    pub async fn prepare(self: &Arc<Self>, zh: &ZhDevice) -> Result<PreparedDefinition, String> {
        self.prepare_with_file(zh, None).await
    }

    /// Prepare a definition with an explicit artifact module file. The lazy
    /// loader uses this so bootstrap does not fall back to an unselected
    /// catalog file.
    pub async fn prepare_with_file(
        self: &Arc<Self>,
        zh: &ZhDevice,
        definition_file: Option<&str>,
    ) -> Result<PreparedDefinition, String> {
        let model = zh
            .model_id()
            .ok_or_else(|| "device has no modelID".to_owned())?;
        let sources: &[&str] = if self.config.prefer_external
            && !self.config.runtime.external_path.as_os_str().is_empty()
        {
            &["external", "package"]
        } else {
            &["package"]
        };
        let mut last_error = String::new();
        for source in sources {
            let mut scenario = self.base_scenario(source, &model);
            if let Some(file) = definition_file {
                if let Some(definition) = scenario
                    .get_mut("definition")
                    .and_then(Value::as_object_mut)
                {
                    definition.insert("file".to_owned(), json!(file));
                }
            }
            scenario.insert("id".to_owned(), json!("bridge-prepare"));
            scenario.insert("kind".to_owned(), json!("prepare"));
            scenario.insert("__metadata".to_owned(), json!(true));
            scenario.insert("device".to_owned(), device_spec(zh));
            let outcome = match self
                .dispatch(
                    &format!("prepare:{}", zh.ieee_addr()),
                    Value::Object(scenario),
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };
            let result = match Self::completed_result(outcome) {
                Ok(result) => result,
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };
            let metadata_value = result.get("result").cloned().unwrap_or(Value::Null);
            let metadata = parse_metadata(&metadata_value)?;
            let definition = build_definition(Arc::clone(self), source, &model, &metadata);
            return Ok(PreparedDefinition {
                definition,
                metadata,
                source: (*source).to_owned(),
                lookup_model: model.clone(),
            });
        }
        Err(if last_error.is_empty() {
            format!("definition not found for '{model}'")
        } else {
            last_error
        })
    }

    /// Prepare every paired device and register the adapter definitions in the
    /// existing registry. Unsupported devices remain definition-less, matching
    /// the upstream not-found fallback.
    pub async fn prepare_and_register(self: &Arc<Self>, zigbee: &Arc<Zigbee>) -> usize {
        let mut registered = 0;
        for device in zigbee.devices_iterator(|device| {
            device.r#type() != zigbee_apiarist::controller::database::DeviceType::Coordinator
        }) {
            let definition_file = self
                .lazy
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|lazy| lazy.resolve(&device_profile(&device.zh)))
                .map(|resolution| runtime_definition_file(&resolution.file.path));
            match self
                .prepare_with_file(&device.zh, definition_file.as_deref())
                .await
            {
                Ok(prepared) => {
                    zigbee
                        .converters()
                        .lock()
                        .unwrap()
                        .register_shared(prepared.definition);
                    device.resolve_definition(true);
                    if device.definition().is_some() {
                        registered += 1;
                    }
                }
                Err(error) => {
                    zigbee.logger().lock().unwrap().debug(
                        &format!(
                            "Converter runtime has no definition for '{}': {error}",
                            device.ieee_addr()
                        ),
                        "z2m",
                    );
                }
            }
        }
        registered
    }

    /// Cancel all work and timers owned by a device. Non-blocking and safe to
    /// call from the bridge loop; the worker polls the shared cancel set.
    pub fn cancel_device(&self, device: impl Into<String>) -> CancelDeviceStatus {
        self.runtime.lock().unwrap().cancel_device(device)
    }

    pub async fn from_zigbee(
        &self,
        source: &str,
        model: &str,
        data: ConvertData,
        publish: PublishCallback,
        options: Value,
        meta: ConvertMeta,
    ) -> Result<(Option<Value>, converter_runtime::JobMetrics), String> {
        let mut scenario = self.base_scenario(source, model);
        scenario.insert("id".to_owned(), json!("bridge-from-zigbee"));
        scenario.insert("kind".to_owned(), json!("fromZigbee"));
        scenario.insert("device".to_owned(), device_spec(&data.device));
        scenario.insert("endpointId".to_owned(), json!(data.endpoint.ID));
        scenario.insert(
            "msg".to_owned(),
            json!({
                "type": data.r#type,
                "cluster": data.cluster,
                "data": data.data,
                "linkquality": data.linkquality,
                "meta": data.meta,
                "groupID": data.group_id,
            }),
        );
        scenario.insert("state".to_owned(), meta.state);
        scenario.insert("options".to_owned(), options);
        let outcome = self
            .dispatch(&data.device.ieee_addr(), Value::Object(scenario))
            .await?;
        let metrics = outcome.metrics.clone();
        let result = Self::completed_result(outcome)?;
        let converted = result.get("result").cloned().unwrap_or(Value::Null);
        // `published` is the merged result for fromZigbee; Receive publishes
        // the return value, so only direct publish callbacks would use this.
        let _ = publish;
        Ok((
            if converted.is_null() {
                None
            } else {
                Some(converted)
            },
            metrics,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn to_zigbee(
        &self,
        source: &str,
        model: &str,
        kind: &str,
        target: ToTarget,
        key: String,
        value: Value,
        meta: ToMeta,
    ) -> Result<(ToZigbeeResult, converter_runtime::JobMetrics), String> {
        let endpoint = match &target {
            ToTarget::Endpoint(endpoint) => endpoint.clone(),
            ToTarget::Group(_) => {
                return Err(
                    "offline converter runtime does not execute group converters yet".to_owned(),
                )
            }
        };
        let device = meta
            .device
            .clone()
            .map(|device| device_spec(&device))
            .unwrap_or_else(|| endpoint_device_spec(&endpoint));
        let mut scenario = self.base_scenario(source, model);
        scenario.insert("id".to_owned(), json!("bridge-to-zigbee"));
        scenario.insert("kind".to_owned(), json!(kind));
        scenario.insert("device".to_owned(), device);
        scenario.insert("endpointId".to_owned(), json!(endpoint.ID));
        scenario.insert("key".to_owned(), json!(key));
        scenario.insert("value".to_owned(), value);
        scenario.insert("state".to_owned(), meta.state.clone());
        scenario.insert("options".to_owned(), meta.options.clone());
        scenario.insert("message".to_owned(), meta.message.clone());
        if let Some(name) = &meta.endpoint_name {
            scenario.insert("endpointName".to_owned(), json!(name));
        }
        let device_key = endpoint.device_ieee_address.lock().unwrap().clone();
        let outcome = self.dispatch(&device_key, Value::Object(scenario)).await?;
        let metrics = outcome.metrics.clone();
        let result = Self::completed_result(outcome)?;
        if let Some(published) = result.get("published").and_then(Value::as_array) {
            for payload in published {
                if let Some(publish) = &meta.publish {
                    publish(payload.clone()).await;
                }
            }
        }
        let converted = result.get("result").cloned().unwrap_or(Value::Null);
        let converted = if converted.is_null() {
            ToZigbeeResult::default()
        } else {
            ToZigbeeResult {
                state: converted.get("state").cloned(),
                members_state: converted.get("membersState").cloned(),
            }
        };
        Ok((converted, metrics))
    }

    pub async fn configure(
        &self,
        source: &str,
        model: &str,
        device: Arc<ZhDevice>,
    ) -> Result<converter_runtime::JobMetrics, String> {
        let mut scenario = self.base_scenario(source, model);
        scenario.insert("id".to_owned(), json!("bridge-configure"));
        scenario.insert("kind".to_owned(), json!("configure"));
        scenario.insert("device".to_owned(), device_spec(&device));
        let outcome = self
            .dispatch(&device.ieee_addr(), Value::Object(scenario))
            .await?;
        let metrics = outcome.metrics.clone();
        Self::completed_result(outcome)?;
        Ok(metrics)
    }

    pub async fn on_event(
        &self,
        source: &str,
        model: &str,
        device: Arc<ZhDevice>,
        event: OnEventEvent,
    ) -> Result<(), String> {
        let (event_type, status) = match &event {
            OnEventEvent::Start(_) => ("start", None),
            OnEventEvent::DeviceJoined(_) => ("deviceJoined", None),
            OnEventEvent::DeviceInterview { status, .. } => {
                ("deviceInterview", Some(status.clone()))
            }
            OnEventEvent::DeviceAnnounce(_) => ("deviceAnnounce", None),
            OnEventEvent::DeviceNetworkAddressChanged(_) => ("deviceNetworkAddressChanged", None),
            OnEventEvent::DeviceOptionsChanged { .. } => ("deviceOptionsChanged", None),
            OnEventEvent::Stop { .. } => ("stop", None),
        };
        let mut scenario = self.base_scenario(source, model);
        scenario.insert("id".to_owned(), json!("bridge-on-event"));
        scenario.insert("kind".to_owned(), json!("onEvent"));
        scenario.insert("eventType".to_owned(), json!(event_type));
        if let Some(status) = status {
            scenario.insert("status".to_owned(), json!(status));
        }
        scenario.insert("device".to_owned(), device_spec(&device));
        let outcome = self
            .dispatch(&device.ieee_addr(), Value::Object(scenario))
            .await?;
        Self::completed_result(outcome).map(|_| ())
    }

    /// Ensure a newly paired device's module closure and prepare its definition
    /// through the same runtime. Cache/network work happens off the reactor.
    pub async fn pair_device(
        self: &Arc<Self>,
        zh: &ZhDevice,
    ) -> Result<PreparedDefinition, String> {
        let lazy = self
            .lazy
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "lazy loader not configured".to_owned())?;
        let fetcher = self
            .lazy_fetcher
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "lazy fetcher not configured".to_owned())?;
        let root = self
            .runtime_root
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "runtime root not configured".to_owned())?;
        let profile = device_profile(zh);
        let lazy_for_task = Arc::clone(&lazy);
        let device = tokio::task::spawn_blocking(move || {
            let device = lazy_for_task.load_device(&profile, fetcher.as_ref(), true)?;
            lazy_for_task.materialize_modules(&device.modules, &root)?;
            Ok::<_, converter_artifact::ArtifactError>(device)
        })
        .await
        .map_err(|error| format!("pairing task: {error}"))?
        .map_err(|error| error.to_string())?;
        self.prepare_with_file(zh, Some(&runtime_definition_file(&device.file)))
            .await
    }

    /// Stop the old worker, fail pending work explicitly and create a fresh
    /// realm. Returns `TimedOut` without creating a second runtime when the old
    /// worker cannot be stopped.
    pub async fn restart(self: &Arc<Self>) -> Result<ShutdownStatus, String> {
        self.abort_tasks();
        self.fail_pending("converter runtime restarted");
        let runtime = Arc::clone(&self.runtime);
        let timeout = self.config.runtime.shutdown_timeout;
        let status = tokio::task::spawn_blocking(move || {
            runtime.lock().unwrap().shutdown_with_timeout(timeout)
        })
        .await
        .map_err(|error| format!("converter shutdown task: {error}"))?;
        if status == ShutdownStatus::TimedOut {
            return Ok(status);
        }
        let runtime_config = self.config.runtime.clone();
        let fresh = tokio::task::spawn_blocking(move || ConverterRuntime::new(runtime_config))
            .await
            .map_err(|error| format!("converter restart task: {error}"))??;
        *self.runtime.lock().unwrap() = fresh;
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.install_channels()?;
        Ok(ShutdownStatus::Stopped)
    }

    /// Bounded bridge shutdown: fail waiting requests, stop the worker and
    /// abort the forwarding tasks.
    pub async fn shutdown(&self) {
        self.abort_tasks();
        self.fail_pending("converter runtime stopped");
        let runtime = Arc::clone(&self.runtime);
        let timeout = self.config.runtime.shutdown_timeout;
        let _ = tokio::task::spawn_blocking(move || {
            runtime.lock().unwrap().shutdown_with_timeout(timeout)
        })
        .await;
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

fn runtime_definition_file(path: &str) -> String {
    path.trim_start_matches("dist/")
        .trim_start_matches("devices/")
        .to_owned()
}

pub fn device_profile(zh: &ZhDevice) -> DeviceProfile {
    DeviceProfile {
        ieee_addr: Some(zh.ieee_addr()),
        r#type: match zh.r#type() {
            zigbee_apiarist::controller::database::DeviceType::Router => DeviceType::Router,
            zigbee_apiarist::controller::database::DeviceType::EndDevice => DeviceType::EndDevice,
            zigbee_apiarist::controller::database::DeviceType::Coordinator => {
                DeviceType::Coordinator
            }
            _ => DeviceType::Unknown,
        },
        model_id: zh.model_id(),
        manufacturer_name: zh.manufacturer_name(),
        manufacturer_id: zh.manufacturer_id().map(u32::from),
        power_source: zh.power_source(),
        application_version: zh.application_version().map(|value| value as u32),
        stack_version: zh.stack_version().map(|value| value as u32),
        hardware_version: zh.hardware_version().map(|value| value as u32),
        zcl_version: zh.zcl_version().map(|value| value as u32),
        date_code: zh.date_code(),
        software_build_id: zh.software_build_id(),
        endpoints: zh
            .endpoints()
            .iter()
            .map(|endpoint| DeviceEndpoint {
                id: Some(endpoint.ID),
                profile_id: *endpoint.profile_id.lock().unwrap(),
                device_id: *endpoint.device_id.lock().unwrap(),
                input_clusters: endpoint.input_clusters.lock().unwrap().clone(),
                output_clusters: endpoint.output_clusters.lock().unwrap().clone(),
            })
            .collect(),
    }
}

pub fn device_spec(zh: &ZhDevice) -> Value {
    let endpoints: Vec<Value> = zh
        .endpoints()
        .iter()
        .map(|endpoint| {
            json!({
                "id": endpoint.ID,
                "inputClusters": endpoint.input_clusters.lock().unwrap().clone(),
                "outputClusters": endpoint.output_clusters.lock().unwrap().clone(),
            })
        })
        .collect();
    json!({
        "ieeeAddr": zh.ieee_addr(),
        "networkAddress": zh.network_address(),
        "modelID": zh.model_id(),
        "manufacturerName": zh.manufacturer_name(),
        "type": zh.r#type().as_str(),
        "endpoints": endpoints,
    })
}

fn endpoint_device_spec(endpoint: &ZhEndpoint) -> Value {
    json!({
        "ieeeAddr": endpoint.device_ieee_address.lock().unwrap().clone(),
        "networkAddress": *endpoint.device_network_address.lock().unwrap(),
        "endpoints": [{
            "id": endpoint.ID,
            "inputClusters": endpoint.input_clusters.lock().unwrap().clone(),
            "outputClusters": endpoint.output_clusters.lock().unwrap().clone(),
        }],
    })
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_metadata(value: &Value) -> Result<DefinitionMetadata, String> {
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if model.is_empty() {
        return Err("converter definition has no model".to_owned());
    }
    let endpoint = value
        .get("endpoint")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(name, id)| id.as_u64().map(|id| (name.clone(), id as u8)))
                .collect()
        })
        .unwrap_or_default();
    let from_zigbee = value
        .get("fromZigbee")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let cluster = entry.get("cluster")?.as_str()?.to_owned();
                    Some((cluster, string_array(entry.get("type"))))
                })
                .collect()
        })
        .unwrap_or_default();
    let to_zigbee = value
        .get("toZigbee")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| string_array(entry.get("key")))
                .filter(|keys| !keys.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Ok(DefinitionMetadata {
        model,
        vendor: value
            .get("vendor")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        description: value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        zigbee_model: string_array(value.get("zigbeeModel")),
        endpoint,
        version: value
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("0.0.0")
            .to_string(),
        from_zigbee,
        to_zigbee,
        has_configure: value
            .get("hasConfigure")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        has_on_event: value
            .get("hasOnEvent")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn build_definition(
    dispatcher: Arc<ConverterDispatcher>,
    source: &str,
    model: &str,
    metadata: &DefinitionMetadata,
) -> DefinitionRef {
    let mut definition = Definition::new(&metadata.model, &metadata.vendor, &metadata.description);
    definition.version = metadata.version.clone();
    definition.endpoint = metadata.endpoint.clone();
    definition.zigbee_model = metadata.zigbee_model.clone();
    if definition.zigbee_model.is_empty() {
        // Fingerprint-only definitions are not indexed by the R1 registry;
        // the lookup key keeps such devices resolvable in the offline stage.
        definition.zigbee_model.push(model.to_owned());
    }

    for (cluster, types) in &metadata.from_zigbee {
        let dispatcher = Arc::clone(&dispatcher);
        let source = source.to_owned();
        let model = model.to_owned();
        let cluster = cluster.clone();
        let types = types.clone();
        definition.from_zigbee.push(FromZigbeeConverter {
            cluster: cluster.clone(),
            types: types.clone(),
            convert: Arc::new(move |_definition, data, publish, options, meta| {
                let dispatcher = Arc::clone(&dispatcher);
                let source = source.clone();
                let model = model.clone();
                Box::pin(async move {
                    dispatcher
                        .from_zigbee(&source, &model, data, publish, options, meta)
                        .await
                        .map(|(converted, _)| converted)
                })
            }),
        });
    }

    for keys in &metadata.to_zigbee {
        let dispatcher = Arc::clone(&dispatcher);
        let source = source.to_owned();
        let model = model.to_owned();
        let keys = keys.clone();
        definition.to_zigbee.push(ToZigbeeConverter {
            key: keys,
            endpoints: None,
            convert_set: Some(Arc::new({
                let dispatcher = Arc::clone(&dispatcher);
                let source = source.clone();
                let model = model.clone();
                move |target, key, value, meta| {
                    let dispatcher = Arc::clone(&dispatcher);
                    let source = source.clone();
                    let model = model.clone();
                    Box::pin(async move {
                        dispatcher
                            .to_zigbee(&source, &model, "set", target, key, value, meta)
                            .await
                            .map(|(result, _)| result)
                    })
                }
            })),
            convert_get: Some(Arc::new({
                let dispatcher = Arc::clone(&dispatcher);
                let source = source.clone();
                let model = model.clone();
                move |target, key, value, meta| {
                    let dispatcher = Arc::clone(&dispatcher);
                    let source = source.clone();
                    let model = model.clone();
                    Box::pin(async move {
                        dispatcher
                            .to_zigbee(&source, &model, "get", target, key, value, meta)
                            .await
                            .map(|(result, _)| result)
                    })
                }
            })),
        });
    }

    if metadata.has_configure {
        let dispatcher = Arc::clone(&dispatcher);
        let source = source.to_owned();
        let model = model.to_owned();
        definition.configure = Some(Arc::new(move |device, _coordinator, _definition| {
            let dispatcher = Arc::clone(&dispatcher);
            let source = source.clone();
            let model = model.clone();
            Box::pin(async move {
                dispatcher
                    .configure(&source, &model, device)
                    .await
                    .map(|_| ())
            })
        }));
    }

    if metadata.has_on_event {
        let dispatcher = Arc::clone(&dispatcher);
        let source = source.to_owned();
        let model = model.to_owned();
        definition.on_event.push(Arc::new(move |event| {
            let dispatcher = Arc::clone(&dispatcher);
            let source = source.clone();
            let model = model.clone();
            let device = match &event {
                OnEventEvent::Start(data)
                | OnEventEvent::DeviceJoined(data)
                | OnEventEvent::DeviceInterview { data, .. }
                | OnEventEvent::DeviceAnnounce(data)
                | OnEventEvent::DeviceNetworkAddressChanged(data)
                | OnEventEvent::DeviceOptionsChanged { data, .. } => Some(data.device.clone()),
                OnEventEvent::Stop { .. } => None,
            };
            Box::pin(async move {
                let Some(device) = device else {
                    return Ok(());
                };
                dispatcher.on_event(&source, &model, device, event).await
            })
        }));
    }

    Arc::new(definition)
}
