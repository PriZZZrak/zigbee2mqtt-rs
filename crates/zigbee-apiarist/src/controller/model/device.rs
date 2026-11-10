// Original: .upstream/zigbee-herdsman/src/controller/model/device.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// R1 scope note: OTA is explicitly excluded from R1 (user decision). The
// upstream OTA members (`checkOta` :1495, `updateOta` :?, `abortOta`,
// `scheduleOta`, `unscheduleOta`, `findMatchingOtaImage` :?, `#waitForOtaCommand`
// :1495, `#notifyOta`) and the OTA-only fields (#scheduledOta/#otaInProgress/
// #otaAbortController) are intentionally NOT ported. Everything else in
// device.ts is ported.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use serde_json::{json, Map, Value};

use crate::controller::database::{DeviceType, EntityType};
use crate::controller::helpers::zcl_transaction_sequence_number;
use crate::controller::model::endpoint::{BindInternal, Endpoint};
use crate::controller::model::entity::Entity;
use crate::controller::tstype::KeyValue;
use crate::utils::logger::logger;
use crate::utils::time_service::TimeService;
use crate::utils::wait::wait;
use crate::zspec::consts::ZIGBEE_REVISION;
use crate::zspec::enums::BroadcastAddress;
use crate::zspec::zcl::definition::status::Status;
use crate::zspec::zcl::utils::Cluster;
use crate::zspec::zcl::utils::{CustomClusters, Utils};
use crate::zspec::zcl::zcl_frame::Frame;
use crate::zspec::zcl::zcl_header::Direction;
use crate::zspec::zcl::zcl_header::FrameType;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;
use crate::zspec::zdo::definition::consts::WITHOUT_REJOIN;
use crate::zspec::zdo::definition::status::Status as ZdoStatus;

const NS: &str = "zh:controller:device";

const INTERVIEW_GENBASIC_ATTRIBUTES: [&str; 9] = [
    "modelId",
    "manufacturerName",
    "powerSource",
    "zclVersion",
    "appVersion",
    "stackVersion",
    "hwVersion",
    "dateCode",
    "swBuildId",
];

const GEN_BASIC_CLUSTER_ID: u16 = 0x0000;
const GEN_TIME_CLUSTER_ID: u16 = 0x000a;
const GEN_POLL_CTRL_CLUSTER_ID: u16 = 0x0020;

/// Upstream `InterviewState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterviewState {
    Pending,
    InProgress,
    Successful,
    Failed,
}

impl InterviewState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Successful => "SUCCESSFUL",
            Self::Failed => "FAILED",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "PENDING" => Self::Pending,
            "IN_PROGRESS" => Self::InProgress,
            "SUCCESSFUL" => Self::Successful,
            "FAILED" => Self::Failed,
            _ => return None,
        })
    }
}

type CustomReadResponse = Box<dyn Fn(&Frame, &Endpoint) -> bool + Send + Sync>;

static DEVICES: OnceLock<RwLock<HashMap<String, Arc<Device>>>> = OnceLock::new();
static DELETED_DEVICES: OnceLock<RwLock<HashMap<String, Arc<Device>>>> = OnceLock::new();
static NWK_TO_IEEE: OnceLock<RwLock<HashMap<u16, String>>> = OnceLock::new();
static LOADED: AtomicBool = AtomicBool::new(false);

fn devices_slot() -> &'static RwLock<HashMap<String, Arc<Device>>> {
    DEVICES.get_or_init(|| RwLock::new(HashMap::new()))
}

fn deleted_slot() -> &'static RwLock<HashMap<String, Arc<Device>>> {
    DELETED_DEVICES.get_or_init(|| RwLock::new(HashMap::new()))
}

fn nwk_slot() -> &'static RwLock<HashMap<u16, String>> {
    NWK_TO_IEEE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Upstream `Device`.
pub struct Device {
    id: u64,
    gen_basic: Mutex<Map<String, Value>>,
    endpoints: Mutex<Vec<Arc<Endpoint>>>,
    ieee_addr: Mutex<String>,
    interview_state: Mutex<InterviewState>,
    last_seen: Mutex<Option<u64>>,
    manufacturer_id: Mutex<Option<u16>>,
    network_address: Mutex<u16>,
    r#type: Mutex<DeviceType>,
    linkquality: Mutex<Option<u8>>,
    skip_default_response: AtomicBool,
    custom_read_response: Mutex<Option<CustomReadResponse>>,
    last_default_response_sequence_number: Mutex<Option<u8>>,
    checkin_interval: Mutex<Option<u64>>,
    pending_request_timeout: Mutex<u64>,
    custom_clusters: Mutex<CustomClusters>,
    gp_security_key: Mutex<Option<Vec<u8>>>,
    pub meta: Mutex<KeyValue>,
}

impl Device {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: u64,
        r#type: DeviceType,
        ieee_addr: String,
        network_address: u16,
        manufacturer_id: Option<u16>,
        endpoints: Vec<Arc<Endpoint>>,
        manufacturer_name: Option<String>,
        power_source: Option<String>,
        model_id: Option<String>,
        application_version: Option<u32>,
        stack_version: Option<u32>,
        zcl_version: Option<u32>,
        hardware_version: Option<u32>,
        date_code: Option<String>,
        software_build_id: Option<String>,
        interview_state: InterviewState,
        meta: KeyValue,
        last_seen: Option<u64>,
        checkin_interval: Option<u64>,
        pending_request_timeout: u64,
        gp_security_key: Option<Vec<u8>>,
    ) -> Self {
        let mut gen_basic = Map::new();
        if let Some(value) = manufacturer_name {
            gen_basic.insert("manufacturerName".to_string(), json!(value));
        }
        gen_basic.insert(
            "powerSource".to_string(),
            json!(power_source.unwrap_or_else(|| "Unknown".to_string())),
        );
        if let Some(value) = model_id {
            gen_basic.insert("modelId".to_string(), json!(value));
        }
        if let Some(value) = application_version {
            gen_basic.insert("appVersion".to_string(), json!(value));
        }
        if let Some(value) = stack_version {
            gen_basic.insert("stackVersion".to_string(), json!(value));
        }
        if let Some(value) = zcl_version {
            gen_basic.insert("zclVersion".to_string(), json!(value));
        }
        if let Some(value) = hardware_version {
            gen_basic.insert("hwVersion".to_string(), json!(value));
        }
        if let Some(value) = date_code {
            gen_basic.insert("dateCode".to_string(), json!(value));
        }
        if let Some(value) = software_build_id {
            gen_basic.insert("swBuildId".to_string(), json!(value));
        }
        Self {
            id,
            gen_basic: Mutex::new(gen_basic),
            endpoints: Mutex::new(endpoints),
            ieee_addr: Mutex::new(ieee_addr),
            interview_state: Mutex::new(interview_state),
            last_seen: Mutex::new(last_seen),
            manufacturer_id: Mutex::new(manufacturer_id),
            network_address: Mutex::new(network_address),
            r#type: Mutex::new(r#type),
            linkquality: Mutex::new(None),
            skip_default_response: AtomicBool::new(false),
            custom_read_response: Mutex::new(None),
            last_default_response_sequence_number: Mutex::new(None),
            checkin_interval: Mutex::new(checkin_interval),
            pending_request_timeout: Mutex::new(pending_request_timeout),
            custom_clusters: Mutex::new(CustomClusters::empty()),
            gp_security_key: Mutex::new(gp_security_key),
            meta: Mutex::new(meta),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn ieee_addr(&self) -> String {
        self.ieee_addr.lock().unwrap().clone()
    }

    pub fn set_ieee_addr(&self, ieee_addr: String) {
        *self.ieee_addr.lock().unwrap() = ieee_addr;
    }

    pub fn network_address(&self) -> u16 {
        *self.network_address.lock().unwrap()
    }

    pub fn set_network_address(&self, address: u16) {
        let old = {
            let mut current = self.network_address.lock().unwrap();
            let old = *current;
            *current = address;
            old
        };
        {
            let mut cache = nwk_slot().write().unwrap();
            cache.remove(&old);
            cache.insert(address, self.ieee_addr());
        }
        for endpoint in self.endpoints() {
            *endpoint.device_network_address.lock().unwrap() = address;
        }
    }

    pub fn r#type(&self) -> DeviceType {
        *self.r#type.lock().unwrap()
    }

    pub fn set_type(&self, r#type: DeviceType) {
        *self.r#type.lock().unwrap() = r#type;
    }

    pub fn interview_state(&self) -> InterviewState {
        *self.interview_state.lock().unwrap()
    }

    pub fn last_seen(&self) -> Option<u64> {
        *self.last_seen.lock().unwrap()
    }

    pub fn manufacturer_id(&self) -> Option<u16> {
        *self.manufacturer_id.lock().unwrap()
    }

    pub fn linkquality(&self) -> Option<u8> {
        *self.linkquality.lock().unwrap()
    }

    pub fn set_linkquality(&self, value: u8) {
        *self.linkquality.lock().unwrap() = Some(value);
    }

    pub fn skip_default_response(&self) -> bool {
        self.skip_default_response.load(Ordering::SeqCst)
    }

    pub fn set_skip_default_response(&self, value: bool) {
        self.skip_default_response.store(value, Ordering::SeqCst);
    }

    pub fn set_custom_read_response(
        &self,
        callback: Option<Box<dyn Fn(&Frame, &Endpoint) -> bool + Send + Sync>>,
    ) {
        *self.custom_read_response.lock().unwrap() = callback;
    }

    pub fn checkin_interval(&self) -> Option<u64> {
        *self.checkin_interval.lock().unwrap()
    }

    pub fn set_checkin_interval(&self, value: Option<u64>) {
        *self.checkin_interval.lock().unwrap() = value;
    }

    pub fn pending_request_timeout(&self) -> u64 {
        *self.pending_request_timeout.lock().unwrap()
    }

    pub fn set_pending_request_timeout(&self, value: u64) {
        *self.pending_request_timeout.lock().unwrap() = value;
    }

    pub fn custom_clusters(&self) -> CustomClusters {
        self.custom_clusters.lock().unwrap().clone()
    }

    pub fn gp_security_key(&self) -> Option<Vec<u8>> {
        self.gp_security_key.lock().unwrap().clone()
    }

    pub fn gen_basic(&self) -> Value {
        Value::Object(self.gen_basic.lock().unwrap().clone())
    }

    pub fn endpoints(&self) -> Vec<Arc<Endpoint>> {
        self.endpoints.lock().unwrap().clone()
    }

    pub fn application_version(&self) -> Option<u64> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("appVersion")
            .and_then(Value::as_u64)
    }
    pub fn set_application_version(&self, value: u32) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("appVersion".to_string(), json!(value));
    }
    pub fn date_code(&self) -> Option<String> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("dateCode")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
    pub fn set_date_code(&self, value: String) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("dateCode".to_string(), json!(value));
    }
    pub fn hardware_version(&self) -> Option<u64> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("hwVersion")
            .and_then(Value::as_u64)
    }
    pub fn set_hardware_version(&self, value: u32) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("hwVersion".to_string(), json!(value));
    }
    pub fn manufacturer_name(&self) -> Option<String> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("manufacturerName")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
    pub fn set_manufacturer_name(&self, value: Option<String>) {
        let mut gen_basic = self.gen_basic.lock().unwrap();
        match value {
            Some(value) => {
                gen_basic.insert("manufacturerName".to_string(), json!(value));
            }
            None => {
                gen_basic.remove("manufacturerName");
            }
        }
    }
    pub fn model_id(&self) -> Option<String> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("modelId")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
    pub fn set_model_id(&self, value: String) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("modelId".to_string(), json!(value));
    }
    pub fn power_source(&self) -> Option<String> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("powerSource")
            .and_then(|value| {
                value.as_str().map(str::to_string).or_else(|| {
                    value.as_u64().map(|code| {
                        crate::zspec::zcl::definition::consts::power_sources()
                            .get(&(code as u8))
                            .copied()
                            .unwrap_or("Unknown")
                            .to_string()
                    })
                })
            })
    }
    pub fn set_power_source(&self, source: Value) {
        let mut gen_basic = self.gen_basic.lock().unwrap();
        if let Some(number) = source.as_u64() {
            gen_basic.insert("powerSource".to_string(), json!(number & !(1 << 7)));
        } else if let Some(name) = source.as_str() {
            for (key, value) in crate::zspec::zcl::definition::consts::power_sources() {
                if value == name {
                    gen_basic.insert("powerSource".to_string(), json!(key));
                    break;
                }
            }
        }
    }
    pub fn software_build_id(&self) -> Option<String> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("swBuildId")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
    pub fn set_software_build_id(&self, value: String) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("swBuildId".to_string(), json!(value));
    }
    pub fn stack_version(&self) -> Option<u64> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("stackVersion")
            .and_then(Value::as_u64)
    }
    pub fn set_stack_version(&self, value: u32) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("stackVersion".to_string(), json!(value));
    }
    pub fn zcl_version(&self) -> Option<u64> {
        self.gen_basic
            .lock()
            .unwrap()
            .get("zclVersion")
            .and_then(Value::as_u64)
    }
    pub fn set_zcl_version(&self, value: u32) {
        self.gen_basic
            .lock()
            .unwrap()
            .insert("zclVersion".to_string(), json!(value));
    }
    pub fn update_gen_basic(&self, data: Value) {
        let mut gen_basic = self.gen_basic.lock().unwrap();
        if let Some(object) = data.as_object() {
            for (key, value) in object {
                gen_basic.insert(key.clone(), value.clone());
            }
        }
    }

    pub fn reset_transient(&self, cache: bool) {
        *self.last_default_response_sequence_number.lock().unwrap() = None;
        if cache {
            *self.checkin_interval.lock().unwrap() = None;
            *self.pending_request_timeout.lock().unwrap() = 0;
        }
    }

    pub fn create_endpoint(&self, id: u8) -> Result<Arc<Endpoint>, String> {
        if self.get_endpoint(id).is_some() {
            return Err(format!(
                "Device '{}' already has an endpoint '{id}'",
                self.ieee_addr()
            ));
        }
        let endpoint = Endpoint::create(
            id,
            None,
            None,
            Vec::new(),
            Vec::new(),
            self.network_address(),
            &self.ieee_addr(),
        );
        self.endpoints.lock().unwrap().push(endpoint.clone());
        self.save(true);
        Ok(endpoint)
    }

    pub fn change_ieee_address(self: &Arc<Self>, ieee_addr: String) {
        let old = self.ieee_addr();
        devices_slot().write().unwrap().remove(&old);
        self.set_ieee_addr(ieee_addr.clone());
        devices_slot()
            .write()
            .unwrap()
            .insert(ieee_addr.clone(), self.arc_self());
        nwk_slot()
            .write()
            .unwrap()
            .insert(self.network_address(), ieee_addr.clone());
        for endpoint in self.endpoints() {
            *endpoint.device_ieee_address.lock().unwrap() = ieee_addr.clone();
        }
        self.save(true);
    }

    pub fn get_endpoint(&self, id: u8) -> Option<Arc<Endpoint>> {
        self.endpoints
            .lock()
            .unwrap()
            .iter()
            .find(|endpoint| endpoint.ID == id)
            .cloned()
    }

    pub fn get_endpoint_by_device_type(&self, device_type: &str) -> Option<Arc<Endpoint>> {
        let device_id = crate::zspec::zcl::definition::consts::endpoint_device_type()
            .get(device_type)
            .copied();
        self.endpoints
            .lock()
            .unwrap()
            .iter()
            .find(|endpoint| *endpoint.device_id.lock().unwrap() == device_id)
            .cloned()
    }

    pub fn implicit_checkin(&self) {
        for endpoint in self.endpoints() {
            tokio::spawn(async move {
                endpoint.send_pending_requests(false).await;
            });
        }
    }

    pub fn update_last_seen(&self) {
        *self.last_seen.lock().unwrap() = Some(crate::utils::datetime::now_ms());
    }

    fn reset_pending_request_timeout(&self) {
        let interval = self.checkin_interval().unwrap_or(0);
        *self.pending_request_timeout.lock().unwrap() = interval * 1000;
    }

    fn has_pending_requests(&self) -> bool {
        self.endpoints()
            .iter()
            .any(|endpoint| endpoint.has_pending_requests())
    }

    pub fn is_deleted(&self) -> bool {
        deleted_slot()
            .read()
            .unwrap()
            .contains_key(&self.ieee_addr())
    }

    /// Upstream `onZclData(dataPayload, frame, endpoint, defaultResponse)`.
    pub async fn on_zcl_data(
        &self,
        data_payload: &crate::adapter::events::ZclPayload,
        frame: Option<&Frame>,
        endpoint: &Arc<Endpoint>,
        default_response: Option<u8>,
    ) -> Result<(), String> {
        if !devices_slot()
            .read()
            .unwrap()
            .contains_key(&self.ieee_addr())
        {
            return Ok(());
        }
        if self.r#type() == DeviceType::GreenPower {
            return Ok(());
        }

        let Some(frame) = frame else {
            let header = data_payload.header;
            if !data_payload.was_broadcast
                && !(header.is_global()
                    && header.command_identifier
                        == crate::zspec::zcl::definition::foundation::Foundation::defaultRsp.ID)
            {
                self.send_default_response(
                    endpoint,
                    data_payload.cluster_id,
                    &header,
                    default_response.unwrap_or(Status::SUCCESS as u8),
                )
                .await;
            }
            return Ok(());
        };

        let header = frame.header;
        let command = &frame.command;
        let cluster = &frame.cluster;
        let mut send_default_response = !data_payload.was_broadcast && command.response.is_none();
        let mut default_response_status = default_response.unwrap_or(Status::SUCCESS as u8);

        if header.is_global() {
            match command.name.as_str() {
                "read" => {
                    let custom = self.custom_read_response.lock().unwrap();
                    if custom
                        .as_ref()
                        .is_some_and(|callback| callback(frame, endpoint))
                    {
                        return Ok(());
                    }
                    drop(custom);

                    let mut response = Map::new();
                    match data_payload.cluster_id {
                        GEN_TIME_CLUSTER_ID => {
                            let mut time_service = TimeService::new(
                                &std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string()),
                                crate::utils::datetime::now_ms() / 1000,
                            );
                            let time_attrs = time_service.get_time_cluster_attributes();
                            let time_json = json!({
                                "time": time_attrs.time,
                                "timeStatus": time_attrs.time_status,
                                "timeZone": time_attrs.time_zone,
                                "dstStart": time_attrs.dst_start,
                                "dstEnd": time_attrs.dst_end,
                                "dstShift": time_attrs.dst_shift,
                                "standardTime": time_attrs.standard_time,
                                "localTime": time_attrs.local_time,
                                "lastSetTime": time_attrs.last_set_time,
                                "validUntilTime": time_attrs.valid_until_time,
                            });
                            for entry in frame.payload.as_array().cloned().unwrap_or_default() {
                                let attr_id =
                                    entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16;
                                let name = Utils::get_cluster_attribute(
                                    cluster,
                                    attr_id as u32,
                                    self.manufacturer_id(),
                                )
                                .map(|attribute| attribute.name);
                                if let Some(name) = name {
                                    response.insert(
                                        name.clone(),
                                        time_json.get(&name).cloned().unwrap_or(Value::Null),
                                    );
                                } else {
                                    response.insert(
                                        attr_id.to_string(),
                                        json!({"value": null, "type": 0x00}),
                                    );
                                }
                            }
                        }
                        GEN_BASIC_CLUSTER_ID => {
                            for entry in frame.payload.as_array().cloned().unwrap_or_default() {
                                let attr_id =
                                    entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16;
                                let attribute = Utils::get_cluster_attribute(
                                    cluster,
                                    attr_id as u32,
                                    self.manufacturer_id(),
                                );
                                match attribute.and_then(|attribute| attribute.default) {
                                    Some(default) => {
                                        let name = Utils::get_cluster_attribute(
                                            cluster,
                                            attr_id as u32,
                                            self.manufacturer_id(),
                                        )
                                        .map(|attribute| attribute.name)
                                        .unwrap_or_default();
                                        response.insert(name, default.to_json());
                                    }
                                    None => {
                                        response.insert(
                                            attr_id.to_string(),
                                            json!({"value": null, "type": 0x00}),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            for entry in frame.payload.as_array().cloned().unwrap_or_default() {
                                let attr_id =
                                    entry.get("attrId").and_then(Value::as_u64).unwrap_or(0);
                                response.insert(
                                    attr_id.to_string(),
                                    json!({"value": null, "type": 0x00}),
                                );
                            }
                        }
                    }

                    if let Err(error) = endpoint
                        .read_response(
                            Value::from(cluster.ID),
                            header.transaction_sequence_number,
                            Value::Object(response),
                            Some(crate::controller::model::endpoint::Options {
                                src_endpoint: Some(data_payload.destination_endpoint),
                                ..Default::default()
                            }),
                        )
                        .await
                    {
                        logger().error(
                            &format!("Read response to {} failed ({error})", self.ieee_addr()),
                            NS,
                        );
                    }
                }
                "defaultRsp" => {
                    send_default_response = false;
                }
                _ => {}
            }
        } else if header.is_specific() {
            match cluster.name.as_str() {
                "ssIasZone" => {
                    if command.name == "enrollReq" {
                        logger().debug(
                            &format!("IAS - '{}' responding to enroll response", self.ieee_addr()),
                            NS,
                        );
                        match endpoint
                            .command(
                                Value::from("ssIasZone"),
                                Value::from("enrollRsp"),
                                json!({"enrollrspcode": 0, "zoneid": 23}),
                                Some(crate::controller::model::endpoint::Options {
                                    transaction_sequence_number: Some(
                                        header.transaction_sequence_number,
                                    ),
                                    disable_default_response: Some(true),
                                    ..Default::default()
                                }),
                            )
                            .await
                        {
                            Ok(_) => send_default_response = false,
                            Err(error) => {
                                logger().error(
                                    &format!(
                                        "Handling of IAS zone enroll for {} failed ({error})",
                                        self.ieee_addr()
                                    ),
                                    NS,
                                );
                                default_response_status = Status::FAILURE as u8;
                            }
                        }
                    }
                }
                "genPollCtrl" => {
                    if command.name == "checkin" {
                        let mut started_fast_polling = false;
                        let result: Result<(), String> = async {
                            if self.has_pending_requests() || self.checkin_interval().is_none() {
                                logger().debug(
                                    &format!(
                                        "check-in from {}: accepting fast-poll",
                                        self.ieee_addr()
                                    ),
                                    NS,
                                );
                                endpoint
                                    .command(
                                        Value::from("genPollCtrl"),
                                        Value::from("checkinRsp"),
                                        json!({"startFastPolling": 1, "fastPollTimeout": 0}),
                                        Some(crate::controller::model::endpoint::Options {
                                            transaction_sequence_number: Some(
                                                header.transaction_sequence_number,
                                            ),
                                            disable_default_response: Some(true),
                                            send_policy: Some(
                                                crate::controller::tstype::SendPolicy::Immediate,
                                            ),
                                            ..Default::default()
                                        }),
                                    )
                                    .await?;
                                started_fast_polling = true;

                                if self.checkin_interval().is_none() {
                                    let poll_period = endpoint
                                        .read(
                                            Value::from("genPollCtrl"),
                                            json!(["checkinInterval"]),
                                            Some(crate::controller::model::endpoint::Options {
                                                send_policy: Some(
                                                    crate::controller::tstype::SendPolicy::Immediate,
                                                ),
                                                ..Default::default()
                                            }),
                                        )
                                        .await?;
                                    let interval = poll_period
                                        .get("checkinInterval")
                                        .and_then(Value::as_u64)
                                        .unwrap_or(0)
                                        / 4;
                                    self.set_checkin_interval(Some(interval));
                                    self.reset_pending_request_timeout();
                                    logger().debug(
                                        &format!(
                                            "Request Queue ({}): default expiration timeout set to {}",
                                            self.ieee_addr(),
                                            self.pending_request_timeout()
                                        ),
                                        NS,
                                    );
                                }
                                for endpoint in self.endpoints() {
                                    endpoint.send_pending_requests(true).await;
                                }
                            } else {
                                logger().debug(
                                    &format!(
                                        "check-in from {}: declining fast-poll",
                                        self.ieee_addr()
                                    ),
                                    NS,
                                );
                                endpoint
                                    .command(
                                        Value::from("genPollCtrl"),
                                        Value::from("checkinRsp"),
                                        json!({"startFastPolling": 0, "fastPollTimeout": 0}),
                                        Some(crate::controller::model::endpoint::Options {
                                            transaction_sequence_number: Some(
                                                header.transaction_sequence_number,
                                            ),
                                            disable_default_response: Some(true),
                                            send_policy: Some(
                                                crate::controller::tstype::SendPolicy::Immediate,
                                            ),
                                            ..Default::default()
                                        }),
                                    )
                                    .await?;
                            }
                            send_default_response = false;
                            Ok(())
                        }
                        .await;
                        if let Err(error) = result {
                            logger().error(
                                &format!(
                                    "Handling of poll check-in from {} failed ({error})",
                                    self.ieee_addr()
                                ),
                                NS,
                            );
                            default_response_status = Status::FAILURE as u8;
                        }
                        if started_fast_polling {
                            logger().debug(
                                &format!("check-in from {}: stopping fast-poll", self.ieee_addr()),
                                NS,
                            );
                            if let Err(error) = endpoint
                                .command(
                                    Value::from("genPollCtrl"),
                                    Value::from("fastPollStop"),
                                    json!({}),
                                    Some(crate::controller::model::endpoint::Options {
                                        send_policy: Some(
                                            crate::controller::tstype::SendPolicy::Immediate,
                                        ),
                                        ..Default::default()
                                    }),
                                )
                                .await
                            {
                                logger().error(
                                    &format!(
                                        "Failed to stop fast poll for {} ({error})",
                                        self.ieee_addr()
                                    ),
                                    NS,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if send_default_response {
            self.send_default_response(
                endpoint,
                cluster.ID as u16,
                &header,
                default_response_status,
            )
            .await;
        }
        Ok(())
    }

    /// Upstream `sendDefaultResponse(...)`.
    async fn send_default_response(
        &self,
        endpoint: &Arc<Endpoint>,
        cluster_id: u16,
        header: &crate::zspec::zcl::zcl_header::Header,
        status: u8,
    ) {
        let disable_tuya_default_response = self
            .manufacturer_name()
            .is_some_and(|name| name.starts_with("_TZ"))
            && std::env::var("DISABLE_TUYA_DEFAULT_RESPONSE").is_ok();
        let already_responded = *self.last_default_response_sequence_number.lock().unwrap()
            == Some(header.transaction_sequence_number);

        if !self.skip_default_response()
            && (!header.frame_control.disable_default_response || status != Status::SUCCESS as u8)
            && !already_responded
            && !disable_tuya_default_response
        {
            if let Err(error) = endpoint
                .default_response(
                    header.command_identifier,
                    status,
                    cluster_id,
                    header.transaction_sequence_number,
                    Some(crate::controller::model::endpoint::Options {
                        direction: Some(
                            if header.frame_control.direction == Direction::CLIENT_TO_SERVER {
                                Direction::SERVER_TO_CLIENT as u8
                            } else {
                                Direction::CLIENT_TO_SERVER as u8
                            },
                        ),
                        ..Default::default()
                    }),
                )
                .await
            {
                logger().debug(
                    &format!("Default response to {} failed ({error})", self.ieee_addr()),
                    NS,
                );
            } else {
                *self.last_default_response_sequence_number.lock().unwrap() =
                    Some(header.transaction_sequence_number);
            }
        }
    }

    /// Upstream `resetCache()`.
    pub fn reset_cache() {
        devices_slot().write().unwrap().clear();
        LOADED.store(false, Ordering::SeqCst);
        deleted_slot().write().unwrap().clear();
        nwk_slot().write().unwrap().clear();
    }

    fn from_database_entry(entry: &Value) -> Arc<Device> {
        let network_address = entry.get("nwkAddr").and_then(Value::as_u64).unwrap_or(0) as u16;
        let ieee_addr = entry
            .get("ieeeAddr")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut endpoints = Vec::new();
        if let Some(map) = entry.get("endpoints").and_then(Value::as_object) {
            for (_, record) in map {
                endpoints.push(Endpoint::from_database_record(
                    record,
                    network_address,
                    &ieee_addr,
                ));
            }
        }
        let checkin_interval = entry.get("checkinInterval").and_then(Value::as_u64);
        let mut pending_request_timeout = 0u64;
        if endpoints.iter().any(|endpoint| {
            endpoint
                .input_clusters
                .lock()
                .unwrap()
                .contains(&GEN_POLL_CTRL_CLUSTER_ID)
        }) {
            pending_request_timeout = 86_400_000;
        }
        if let Some(interval) = checkin_interval {
            pending_request_timeout = interval * 1000;
        }
        logger().debug(
            &format!("Request Queue ({ieee_addr}): default expiration timeout set to {pending_request_timeout}"),
            NS,
        );

        let interview_state = match entry.get("interviewState").and_then(Value::as_str) {
            Some(value) => InterviewState::from_str(value).unwrap_or(InterviewState::Pending),
            None => {
                if entry
                    .get("interviewCompleted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    InterviewState::Successful
                } else {
                    InterviewState::Failed
                }
            }
        };

        let r#type = match entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
        {
            "Coordinator" => DeviceType::Coordinator,
            "Router" => DeviceType::Router,
            "EndDevice" => DeviceType::EndDevice,
            "GreenPower" => DeviceType::GreenPower,
            _ => DeviceType::Unknown,
        };
        let value_str = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_string);
        let value_u32 = |key: &str| entry.get(key).and_then(Value::as_u64).map(|v| v as u32);
        let gp_security_key = entry
            .get("gpSecurityKey")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|v| v as u8)
                    .collect::<Vec<_>>()
            });

        Arc::new(Self::new(
            entry.get("id").and_then(Value::as_u64).unwrap_or(0),
            r#type,
            ieee_addr,
            network_address,
            entry
                .get("manufId")
                .and_then(Value::as_u64)
                .map(|v| v as u16),
            endpoints,
            value_str("manufName"),
            value_str("powerSource"),
            value_str("modelId"),
            value_u32("appVersion"),
            value_u32("stackVersion"),
            value_u32("zclVersion"),
            value_u32("hwVersion"),
            value_str("dateCode"),
            value_str("swBuildId"),
            interview_state,
            entry.get("meta").cloned().unwrap_or_else(|| json!({})),
            entry.get("lastSeen").and_then(Value::as_u64),
            checkin_interval,
            pending_request_timeout,
            gp_security_key,
        ))
    }

    fn to_database_entry(&self) -> Value {
        let endpoints = self.endpoints();
        let ep_list = endpoints
            .iter()
            .map(|endpoint| endpoint.ID)
            .collect::<Vec<_>>();
        let mut endpoint_map = Map::new();
        for endpoint in &endpoints {
            endpoint_map.insert(endpoint.ID.to_string(), endpoint.to_database_record());
        }
        let interview_state = self.interview_state();
        json!({
            "id": self.id,
            "type": self.r#type().as_str(),
            "ieeeAddr": self.ieee_addr(),
            "nwkAddr": self.network_address(),
            "manufId": self.manufacturer_id(),
            "manufName": self.manufacturer_name(),
            "powerSource": self.power_source(),
            "modelId": self.model_id(),
            "epList": ep_list,
            "endpoints": Value::Object(endpoint_map),
            "appVersion": self.application_version(),
            "stackVersion": self.stack_version(),
            "hwVersion": self.hardware_version(),
            "dateCode": self.date_code(),
            "swBuildId": self.software_build_id(),
            "zclVersion": self.zcl_version(),
            "interviewCompleted": interview_state == InterviewState::Successful,
            "interviewState": if interview_state == InterviewState::InProgress {
                InterviewState::Pending.as_str()
            } else {
                interview_state.as_str()
            },
            "meta": self.meta.lock().unwrap().clone(),
            "lastSeen": self.last_seen(),
            "checkinInterval": self.checkin_interval(),
            "gpSecurityKey": self.gp_security_key(),
        })
    }

    pub fn save(&self, write_database: bool) {
        let entry = self.to_database_entry();
        let _ = Entity::database()
            .lock()
            .unwrap()
            .update(entry, write_database);
    }

    fn load_from_database_if_necessary() {
        if !LOADED.swap(true, Ordering::SeqCst) {
            let entries = Entity::database()
                .lock()
                .unwrap()
                .get_entries_iterator(&[
                    EntityType::DeviceType(DeviceType::Coordinator),
                    EntityType::DeviceType(DeviceType::EndDevice),
                    EntityType::DeviceType(DeviceType::Router),
                    EntityType::DeviceType(DeviceType::GreenPower),
                    EntityType::DeviceType(DeviceType::Unknown),
                ])
                .cloned()
                .collect::<Vec<_>>();
            for entry in &entries {
                let device = Device::from_database_entry(entry);
                devices_slot()
                    .write()
                    .unwrap()
                    .insert(device.ieee_addr(), device.clone());
                nwk_slot()
                    .write()
                    .unwrap()
                    .insert(device.network_address(), device.ieee_addr());
            }
        }
    }

    pub fn find(ieee_or_nwk: Value, include_deleted: bool) -> Option<Arc<Device>> {
        match ieee_or_nwk {
            Value::String(ieee) => Self::by_ieee_addr(&ieee, include_deleted),
            Value::Number(number) => {
                Self::by_network_address(number.as_u64().unwrap_or(0) as u16, include_deleted)
            }
            _ => None,
        }
    }

    pub fn by_ieee_addr(ieee_addr: &str, include_deleted: bool) -> Option<Arc<Device>> {
        Self::load_from_database_if_necessary();
        if include_deleted {
            let deleted = deleted_slot().read().unwrap().get(ieee_addr).cloned();
            deleted.or_else(|| devices_slot().read().unwrap().get(ieee_addr).cloned())
        } else {
            devices_slot().read().unwrap().get(ieee_addr).cloned()
        }
    }

    pub fn by_network_address(network_address: u16, include_deleted: bool) -> Option<Arc<Device>> {
        Self::load_from_database_if_necessary();
        let ieee = nwk_slot().read().unwrap().get(&network_address).cloned()?;
        Self::by_ieee_addr(&ieee, include_deleted)
    }

    pub fn by_type(r#type: DeviceType) -> Vec<Arc<Device>> {
        Self::all_iterator(|device| device.r#type() == r#type)
    }

    pub fn is_deleted_by_ieee_addr(ieee_addr: &str) -> bool {
        Self::load_from_database_if_necessary();
        deleted_slot().read().unwrap().contains_key(ieee_addr)
    }

    pub fn is_deleted_by_network_address(network_address: u16) -> bool {
        Self::load_from_database_if_necessary();
        nwk_slot()
            .read()
            .unwrap()
            .get(&network_address)
            .is_some_and(|ieee| deleted_slot().read().unwrap().contains_key(ieee))
    }

    pub fn all() -> Vec<Arc<Device>> {
        Self::load_from_database_if_necessary();
        devices_slot().read().unwrap().values().cloned().collect()
    }

    pub fn all_iterator(predicate: impl Fn(&Arc<Device>) -> bool) -> Vec<Arc<Device>> {
        Self::all()
            .into_iter()
            .filter(|device| predicate(device))
            .collect()
    }

    pub fn undelete(self: &Arc<Self>) -> Result<(), String> {
        if deleted_slot()
            .write()
            .unwrap()
            .remove(&self.ieee_addr())
            .is_some()
        {
            devices_slot()
                .write()
                .unwrap()
                .insert(self.ieee_addr(), self.clone());
            Entity::database()
                .lock()
                .unwrap()
                .insert(self.to_database_entry())
        } else {
            Err(format!("Device '{}' is not deleted", self.ieee_addr()))
        }
    }

    pub fn create(
        r#type: DeviceType,
        ieee_addr: &str,
        network_address: u16,
        manufacturer_id: Option<u16>,
        manufacturer_name: Option<String>,
        power_source: Option<String>,
        model_id: Option<String>,
        interview_state: InterviewState,
        gp_security_key: Option<Vec<u8>>,
    ) -> Result<Arc<Device>, String> {
        Self::load_from_database_if_necessary();
        if devices_slot().read().unwrap().contains_key(ieee_addr) {
            return Err(format!(
                "Device with IEEE address '{ieee_addr}' already exists"
            ));
        }
        let id = Entity::database().lock().unwrap().new_id();
        let device = Arc::new(Self::new(
            id,
            r#type,
            ieee_addr.to_string(),
            network_address,
            manufacturer_id,
            Vec::new(),
            manufacturer_name,
            power_source,
            model_id,
            None,
            None,
            None,
            None,
            None,
            None,
            interview_state,
            json!({}),
            None,
            None,
            0,
            gp_security_key,
        ));
        Entity::database()
            .lock()
            .unwrap()
            .insert(device.to_database_entry())?;
        devices_slot()
            .write()
            .unwrap()
            .insert(device.ieee_addr(), device.clone());
        nwk_slot()
            .write()
            .unwrap()
            .insert(network_address, device.ieee_addr());
        Ok(device)
    }

    /// Upstream `interview(ignoreCache = false)`.
    pub async fn interview(self: &Arc<Self>, ignore_cache: bool) -> Result<(), String> {
        if self.interview_state() == InterviewState::InProgress {
            let message = format!(
                "Interview - interview already in progress for '{}'",
                self.ieee_addr()
            );
            logger().debug(&message, NS);
            return Err(message);
        }
        *self.interview_state.lock().unwrap() = InterviewState::InProgress;
        logger().debug(
            &format!("Interview - start device '{}'", self.ieee_addr()),
            NS,
        );
        let result = self.interview_internal(ignore_cache).await;
        match result {
            Ok(()) => {
                logger().debug(
                    &format!("Interview - completed for device '{}'", self.ieee_addr()),
                    NS,
                );
                *self.interview_state.lock().unwrap() = InterviewState::Successful;
            }
            Err(error) => {
                if self.interview_quirks() {
                    *self.interview_state.lock().unwrap() = InterviewState::Successful;
                    logger().debug(
                        &format!(
                            "Interview - completed for device '{}' because of quirks ('{error}')",
                            self.ieee_addr()
                        ),
                        NS,
                    );
                } else {
                    *self.interview_state.lock().unwrap() = InterviewState::Failed;
                    logger().debug(
                        &format!(
                            "Interview - failed for device '{}' with error '{error}'",
                            self.ieee_addr()
                        ),
                        NS,
                    );
                    self.save(true);
                    return Err(error);
                }
            }
        }
        self.save(true);
        Ok(())
    }

    fn interview_quirks(&self) -> bool {
        logger().debug(
            &format!(
                "Interview - quirks check for '{:?}'-'{:?}'-'{}'",
                self.model_id(),
                self.manufacturer_name(),
                self.r#type().as_str()
            ),
            NS,
        );
        let manufacturer_name = self.manufacturer_name().unwrap_or_default();
        let model_id = self.model_id().unwrap_or_default();

        if manufacturer_name == "HOBEIAN"
            || (matches_ts_model(&model_id)
                && (matches_tz_name(&manufacturer_name)
                    || manufacturer_name.starts_with("_TYZB01_")))
        {
            if self.gen_basic.lock().unwrap().get("powerSource").is_none() {
                self.set_power_source(json!("Battery"));
            }
            logger().debug("Interview - quirks matched for Tuya end device", NS);
            return true;
        }

        if self.manufacturer_id() == Some(0xabcd) {
            self.set_manufacturer_name(Some("Control4".to_string()));
            self.set_model_id("C4-Zigbee".to_string());
            self.set_power_source(json!("Mains (single phase)"));
            logger().debug("Interview - quirks matched for Control4 device", NS);
            return true;
        }

        let mut matched: Option<&str> = None;
        for key in [
            "^3R.*?Z",
            "lumi..*",
            "TERNCY-PP01",
            "3RWS18BZ",
            "MULTI-MECI--EA01",
            "MOT003",
            "C-ZB-SEDC",
            "C-ZB-SEMO",
            "CS-T9C-A0-BG",
            "SNZB-01",
            "3011",
            "Smart Siren",
        ] {
            if model_matches(&model_id, key) {
                matched = Some(key);
                break;
            }
        }
        let Some(matched) = matched else {
            logger().debug("Interview - quirks did not match", NS);
            return false;
        };
        logger().debug(
            &format!(
                "Interview procedure failed but got modelID matching '{matched}', assuming interview succeeded"
            ),
            NS,
        );
        match matched {
            "^3R.*?Z" => {
                if self.r#type() == DeviceType::Unknown {
                    self.set_type(DeviceType::EndDevice);
                }
                self.set_power_source(json!("Battery"));
            }
            "lumi..*" => {
                if self.r#type() == DeviceType::Unknown {
                    self.set_type(DeviceType::EndDevice);
                }
                if self.manufacturer_id().is_none() {
                    *self.manufacturer_id.lock().unwrap() = Some(4151);
                }
                self.set_manufacturer_name(Some("LUMI".to_string()));
                self.set_power_source(json!("Battery"));
            }
            "TERNCY-PP01" => {
                if self.r#type() == DeviceType::Unknown {
                    self.set_type(DeviceType::EndDevice);
                }
                if self.manufacturer_id().is_none() {
                    *self.manufacturer_id.lock().unwrap() = Some(4648);
                }
                self.set_manufacturer_name(Some("TERNCY".to_string()));
                self.set_power_source(json!("Battery"));
            }
            "3011" => {
                if self.r#type() == DeviceType::Unknown {
                    self.set_type(DeviceType::EndDevice);
                }
                self.set_power_source(json!("Battery"));
            }
            "Smart Siren" => {
                self.set_manufacturer_name(Some("AduroSmart Eria".to_string()));
            }
            _ => {}
        }
        logger().debug(&format!("Interview - quirks matched on '{matched}'"), NS);
        true
    }

    async fn interview_internal(self: &Arc<Self>, ignore_cache: bool) -> Result<(), String> {
        let has_node_descriptor =
            || self.manufacturer_id().is_some() && self.r#type() != DeviceType::Unknown;
        if ignore_cache || !has_node_descriptor() {
            for attempt in 0..6 {
                match self.update_node_descriptor().await {
                    Ok(()) => break,
                    Err(error) => {
                        if self.interview_quirks() {
                            logger().debug(
                                &format!(
                                    "Interview - completed for device '{}' because of quirks ('{error}')",
                                    self.ieee_addr()
                                ),
                                NS,
                            );
                            return Ok(());
                        }
                        logger().debug(
                            &format!(
                                "Interview - node descriptor request failed for '{}', attempt {}",
                                self.ieee_addr(),
                                attempt + 1
                            ),
                            NS,
                        );
                    }
                }
            }
        } else {
            logger().debug(
                &format!(
                    "Interview - skip node descriptor request for '{}', already got it",
                    self.ieee_addr()
                ),
                NS,
            );
        }
        if !has_node_descriptor() {
            return Err(format!(
                "Interview failed because can not get node descriptor ('{}')",
                self.ieee_addr()
            ));
        }

        if self.manufacturer_id() == Some(4619) && self.r#type() == DeviceType::EndDevice {
            logger().debug(
                "Interview - Detected Tuya end device, waiting 10 seconds...",
                NS,
            );
            wait(10000).await;
        } else if matches!(self.manufacturer_id(), Some(0) | Some(4098)) {
            logger().debug(
                "Interview - Detected potential Tuya end device, reading modelID and manufacturerName...",
                NS,
            );
            let endpoint = Endpoint::create(
                1,
                None,
                None,
                Vec::new(),
                Vec::new(),
                self.network_address(),
                &self.ieee_addr(),
            );
            if let Ok(result) = endpoint
                .read(
                    Value::from("genBasic"),
                    json!(["modelId", "manufacturerName"]),
                    Some(crate::controller::model::endpoint::Options {
                        send_policy: Some(crate::controller::tstype::SendPolicy::Immediate),
                        ..Default::default()
                    }),
                )
                .await
            {
                self.update_gen_basic(result);
            } else {
                logger().debug(
                    &format!(
                        "Interview - Tuya read modelID and manufacturerName failed for {}",
                        self.ieee_addr()
                    ),
                    NS,
                );
            }
        }

        let mut got_active_endpoints = false;
        for attempt in 0..2 {
            match self.update_active_endpoints().await {
                Ok(()) => {
                    got_active_endpoints = true;
                    break;
                }
                Err(error) => logger().debug(
                    &format!(
                        "Interview - active endpoints request failed for '{}', attempt {} ({error})",
                        self.ieee_addr(),
                        attempt + 1
                    ),
                    NS,
                ),
            }
        }
        if !got_active_endpoints {
            return Err(format!(
                "Interview failed because can not get active endpoints ('{}')",
                self.ieee_addr()
            ));
        }
        logger().debug(
            &format!(
                "Interview - got active endpoints for device '{}'",
                self.ieee_addr()
            ),
            NS,
        );

        let coordinator = Device::by_type(DeviceType::Coordinator)
            .first()
            .cloned()
            .ok_or_else(|| "Coordinator not found".to_string())?;
        let mut gen_basic_attrs = INTERVIEW_GENBASIC_ATTRIBUTES
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        for endpoint in self.endpoints() {
            endpoint.update_simple_descriptor().await?;
            logger().debug(
                &format!(
                    "Interview - got simple descriptor for endpoint '{}' device '{}'",
                    endpoint.ID,
                    self.ieee_addr()
                ),
                NS,
            );

            if endpoint.supports_input_cluster(Value::from("genBasic")) {
                for key in gen_basic_attrs.clone() {
                    let cached = self.gen_basic.lock().unwrap().get(&key).cloned();
                    let cached_empty =
                        cached.is_none() || cached == Some(Value::String(String::new()));
                    let cached_power_unknown =
                        key == "powerSource" && self.power_source().as_deref() == Some("Unknown");
                    if !(ignore_cache || cached_empty || cached_power_unknown) {
                        continue;
                    }
                    let read = async {
                        match endpoint
                            .read(
                                Value::from("genBasic"),
                                json!([key]),
                                Some(crate::controller::model::endpoint::Options {
                                    send_policy: Some(
                                        crate::controller::tstype::SendPolicy::Immediate,
                                    ),
                                    ..Default::default()
                                }),
                            )
                            .await
                        {
                            Ok(value) => Ok(value),
                            Err(error) => {
                                if key == "modelId" || key == "manufacturerName" {
                                    logger().debug(
                                        &format!(
                                            "Interview - first {key} retrieval attempt failed, retrying after 10 seconds..."
                                        ),
                                        NS,
                                    );
                                    wait(10000).await;
                                    endpoint
                                        .read(
                                            Value::from("genBasic"),
                                            json!([key]),
                                            Some(crate::controller::model::endpoint::Options {
                                                send_policy: Some(
                                                    crate::controller::tstype::SendPolicy::Immediate,
                                                ),
                                                ..Default::default()
                                            }),
                                        )
                                        .await
                                } else {
                                    Err(error)
                                }
                            }
                        }
                    }
                    .await;
                    match read {
                        Ok(value) => {
                            self.update_gen_basic(value);
                            gen_basic_attrs.retain(|attribute| attribute != &key);
                            logger().debug(
                                &format!(
                                    "Interview - got '{key}' for device '{}'",
                                    self.ieee_addr()
                                ),
                                NS,
                            );
                        }
                        Err(error) => logger().debug(
                            &format!(
                                "Interview - failed to read attribute '{key}' from endpoint '{}' ({error})",
                                endpoint.ID
                            ),
                            NS,
                        ),
                    }
                }
            }

            if endpoint.supports_input_cluster(Value::from("ssIasZone")) {
                logger().debug(
                    &format!(
                        "Interview - IAS - enrolling '{}' endpoint '{}'",
                        self.ieee_addr(),
                        endpoint.ID
                    ),
                    NS,
                );
                let immediate = Some(crate::controller::model::endpoint::Options {
                    send_policy: Some(crate::controller::tstype::SendPolicy::Immediate),
                    ..Default::default()
                });
                let state_before = endpoint
                    .read(
                        Value::from("ssIasZone"),
                        json!(["iasCieAddr", "zoneState"]),
                        immediate.clone(),
                    )
                    .await?;
                logger().debug(
                    &format!("Interview - IAS - before enrolling state: '{state_before}'"),
                    NS,
                );
                if state_before.get("zoneState").and_then(Value::as_u64) != Some(1)
                    || state_before.get("iasCieAddr").and_then(Value::as_str)
                        != Some(coordinator.ieee_addr().as_str())
                {
                    logger().debug("Interview - IAS - not enrolled, enrolling", NS);
                    endpoint
                        .write(
                            Value::from("ssIasZone"),
                            json!({"iasCieAddr": coordinator.ieee_addr()}),
                            immediate.clone(),
                        )
                        .await?;
                    logger().debug("Interview - IAS - wrote iasCieAddr", NS);
                    wait(500).await;
                    logger().debug(
                        &format!(
                            "IAS - '{}' sending enroll response (auto enroll)",
                            self.ieee_addr()
                        ),
                        NS,
                    );
                    endpoint
                        .command(
                            Value::from("ssIasZone"),
                            Value::from("enrollRsp"),
                            json!({"enrollrspcode": 0, "zoneid": 23}),
                            Some(crate::controller::model::endpoint::Options {
                                disable_default_response: Some(true),
                                send_policy: Some(crate::controller::tstype::SendPolicy::Immediate),
                                ..Default::default()
                            }),
                        )
                        .await?;
                    let mut enrolled = false;
                    for attempt in 0..20 {
                        wait(500).await;
                        let state_after = endpoint
                            .read(
                                Value::from("ssIasZone"),
                                json!(["iasCieAddr", "zoneState"]),
                                immediate.clone(),
                            )
                            .await?;
                        logger().debug(
                            &format!(
                                "Interview - IAS - after enrolling state ({attempt}): '{state_after}'"
                            ),
                            NS,
                        );
                        if state_after.get("zoneState").and_then(Value::as_u64) == Some(1) {
                            enrolled = true;
                            break;
                        }
                    }
                    if !enrolled {
                        return Err(format!(
                            "Interview failed because of failed IAS enroll (zoneState didn't change ('{}')",
                            self.ieee_addr()
                        ));
                    }
                } else {
                    logger().debug("Interview - IAS - already enrolled, skipping enroll", NS);
                }
            }
        }

        let poll_endpoints = self
            .endpoints()
            .into_iter()
            .filter(|endpoint| endpoint.supports_input_cluster(Value::from("genPollCtrl")))
            .collect::<Vec<_>>();
        for endpoint in poll_endpoints {
            logger().debug(
                &format!(
                    "Interview - Poll control - binding '{}' endpoint '{}'",
                    self.ieee_addr(),
                    endpoint.ID
                ),
                NS,
            );
            let coordinator_endpoint = coordinator
                .endpoints()
                .first()
                .cloned()
                .ok_or_else(|| "Coordinator has no endpoint".to_string())?;
            if let Err(error) = endpoint
                .bind(
                    Value::from("genPollCtrl"),
                    crate::controller::model::endpoint::BindTarget::Endpoint(coordinator_endpoint),
                )
                .await
            {
                logger().debug(
                    &format!("Interview - failed to bind genPollCtrl ({error})"),
                    NS,
                );
                continue;
            }
            if let Ok(poll_period) = endpoint
                .read(
                    Value::from("genPollCtrl"),
                    json!(["checkinInterval"]),
                    Some(crate::controller::model::endpoint::Options {
                        send_policy: Some(crate::controller::tstype::SendPolicy::Immediate),
                        ..Default::default()
                    }),
                )
                .await
            {
                let interval = poll_period
                    .get("checkinInterval")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    / 4;
                self.set_checkin_interval(Some(interval));
                self.reset_pending_request_timeout();
            }
        }
        Ok(())
    }

    /// Upstream `updateNodeDescriptor()`.
    pub async fn update_node_descriptor(&self) -> Result<(), String> {
        let cluster_id = ClusterId::NODE_DESCRIPTOR_REQUEST;
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[json!(self.network_address())],
        );
        let response = Entity::adapter()
            .send_zdo(
                &self.ieee_addr(),
                self.network_address(),
                cluster_id,
                &payload,
                false,
            )
            .await?;
        if !BuffaloZdo::check_status(&response) {
            return Err(zdo_status_error(&response));
        }
        let node_descriptor = response.get(1).cloned().unwrap_or(Value::Null);
        *self.manufacturer_id.lock().unwrap() = node_descriptor
            .get("manufacturerCode")
            .and_then(Value::as_u64)
            .map(|value| value as u16);
        match node_descriptor.get("logicalType").and_then(Value::as_u64) {
            Some(0) => self.set_type(DeviceType::Coordinator),
            Some(1) => self.set_type(DeviceType::Router),
            Some(2) => self.set_type(DeviceType::EndDevice),
            _ => {}
        }
        logger().debug(
            &format!(
                "Interview - got node descriptor for device '{}'",
                self.ieee_addr()
            ),
            NS,
        );
        let revision = node_descriptor
            .get("serverMask")
            .and_then(|mask| mask.get("stackComplianceRevision"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        if revision < ZIGBEE_REVISION - 1 {
            let rev = if revision < 21 {
                "pre-21".to_string()
            } else {
                revision.to_string()
            };
            logger().info(
                &format!(
                    "Device '{}' is only compliant to revision '{rev}' of the Zigbee specification (current revision: {ZIGBEE_REVISION}).",
                    self.ieee_addr()
                ),
                NS,
            );
        }
        Ok(())
    }

    /// Upstream `updateActiveEndpoints()`.
    pub async fn update_active_endpoints(&self) -> Result<(), String> {
        let cluster_id = ClusterId::ACTIVE_ENDPOINTS_REQUEST;
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[json!(self.network_address())],
        );
        let response = Entity::adapter()
            .send_zdo(
                &self.ieee_addr(),
                self.network_address(),
                cluster_id,
                &payload,
                false,
            )
            .await?;
        if !BuffaloZdo::check_status(&response) {
            return Err(zdo_status_error(&response));
        }
        let mut active = response
            .get(1)
            .and_then(|value| value.get("endpointList"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|v| v as u8)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        active.sort_unstable();
        for endpoint_id in &active {
            if *endpoint_id != 0 && self.get_endpoint(*endpoint_id).is_none() {
                self.endpoints.lock().unwrap().push(Endpoint::create(
                    *endpoint_id,
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    self.network_address(),
                    &self.ieee_addr(),
                ));
            }
        }
        self.endpoints
            .lock()
            .unwrap()
            .retain(|endpoint| active.contains(&endpoint.ID));
        Ok(())
    }

    /// Upstream `requestNetworkAddress()`.
    pub async fn request_network_address(&self) -> Result<(), String> {
        let cluster_id = ClusterId::NETWORK_ADDRESS_REQUEST;
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[json!(self.ieee_addr()), json!(false), json!(0)],
        );
        Entity::adapter()
            .send_zdo(
                &self.ieee_addr(),
                BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                cluster_id,
                &payload,
                true,
            )
            .await?;
        Ok(())
    }

    /// Upstream `removeFromNetwork(clearCache = false)`.
    pub async fn remove_from_network(self: &Arc<Self>, clear_cache: bool) -> Result<(), String> {
        if self.r#type() == DeviceType::GreenPower {
            let payload = json!({
                "options": 0x002550,
                "srcID": self.ieee_addr().trim_start_matches("0x").parse::<u64>().unwrap_or(0),
            });
            let cluster = Utils::get_cluster(33u32, None, &self.custom_clusters());
            let command = Utils::get_cluster_command(&cluster, "pairing");
            let frame = Frame::create_with_definitions(
                cluster,
                command,
                FrameType::SPECIFIC as u8,
                Direction::SERVER_TO_CLIENT as u8,
                true,
                None,
                zcl_transaction_sequence_number::next(),
                payload,
                0,
            );
            Entity::adapter()
                .send_zcl_frame_to_all(
                    242,
                    &frame,
                    242,
                    BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                    None,
                )
                .await?;
        } else {
            let cluster_id = ClusterId::LEAVE_REQUEST;
            let payload = BuffaloZdo::build_request(
                Entity::adapter().has_zdo_message_overhead(),
                cluster_id,
                &[json!(self.ieee_addr()), json!(WITHOUT_REJOIN)],
            );
            let response = Entity::adapter()
                .send_zdo(
                    &self.ieee_addr(),
                    self.network_address(),
                    cluster_id,
                    &payload,
                    false,
                )
                .await?;
            if !BuffaloZdo::check_status(&response) {
                return Err(zdo_status_error(&response));
            }
        }
        self.remove_from_database(clear_cache);
        Ok(())
    }

    /// Upstream `removeFromDatabase(clearCache = false)`.
    pub fn remove_from_database(self: &Arc<Self>, clear_cache: bool) {
        Self::load_from_database_if_necessary();
        for endpoint in self.endpoints() {
            endpoint.remove_from_all_groups_database();
        }
        {
            let database_arc = Entity::database();
            let mut database = database_arc.lock().unwrap();
            if database.has(self.id) {
                let _ = database.remove(self.id);
            }
        }
        if clear_cache {
            devices_slot().write().unwrap().remove(&self.ieee_addr());
        } else {
            deleted_slot()
                .write()
                .unwrap()
                .insert(self.ieee_addr(), self.clone());
            devices_slot().write().unwrap().remove(&self.ieee_addr());
            if self.r#type() != DeviceType::GreenPower {
                *self.interview_state.lock().unwrap() = InterviewState::Pending;
            }
            *self.meta.lock().unwrap() = json!({});
            let mut new_endpoints = Vec::new();
            for endpoint in self.endpoints() {
                new_endpoints.push(Endpoint::create(
                    endpoint.ID,
                    *endpoint.profile_id.lock().unwrap(),
                    *endpoint.device_id.lock().unwrap(),
                    endpoint.input_clusters.lock().unwrap().clone(),
                    endpoint.output_clusters.lock().unwrap().clone(),
                    self.network_address(),
                    &self.ieee_addr(),
                ));
            }
            *self.endpoints.lock().unwrap() = new_endpoints;
        }
    }

    /// Upstream `lqi()`.
    pub async fn lqi(&self) -> Result<Vec<Value>, String> {
        let cluster_id = ClusterId::LQI_TABLE_REQUEST;
        let mut table = Vec::new();
        let mut request = |start_index: u16| {
            let ieee = self.ieee_addr();
            let network = self.network_address();
            async move {
                let payload = BuffaloZdo::build_request(
                    Entity::adapter().has_zdo_message_overhead(),
                    cluster_id,
                    &[json!(start_index)],
                );
                let response = Entity::adapter()
                    .send_zdo(&ieee, network, cluster_id, &payload, false)
                    .await?;
                if !BuffaloZdo::check_status(&response) {
                    return Err(zdo_status_error(&response));
                }
                let result = response.get(1).cloned().unwrap_or(Value::Null);
                let entries = result
                    .get("entryList")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let total = result
                    .get("neighborTableEntries")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                Ok::<(usize, Vec<Value>), String>((total, entries))
            }
        };
        let (size, entries) = request(0).await?;
        table.extend(entries);
        let mut next_start = table.len() as u16;
        while table.len() < size {
            let (_, entries) = request(next_start).await?;
            next_start += entries.len() as u16;
            table.extend(entries);
        }
        Ok(table)
    }

    /// Upstream `routingTable()`.
    pub async fn routing_table(&self) -> Result<Vec<Value>, String> {
        let cluster_id = ClusterId::ROUTING_TABLE_REQUEST;
        let mut table = Vec::new();
        let mut request = |start_index: u16| {
            let ieee = self.ieee_addr();
            let network = self.network_address();
            async move {
                let payload = BuffaloZdo::build_request(
                    Entity::adapter().has_zdo_message_overhead(),
                    cluster_id,
                    &[json!(start_index)],
                );
                let response = Entity::adapter()
                    .send_zdo(&ieee, network, cluster_id, &payload, false)
                    .await?;
                if !BuffaloZdo::check_status(&response) {
                    return Err(zdo_status_error(&response));
                }
                let result = response.get(1).cloned().unwrap_or(Value::Null);
                let entries = result
                    .get("entryList")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let total = result
                    .get("routingTableEntries")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                Ok::<(usize, Vec<Value>), String>((total, entries))
            }
        };
        let (size, entries) = request(0).await?;
        table.extend(entries);
        let mut next_start = table.len() as u16;
        while table.len() < size {
            let (_, entries) = request(next_start).await?;
            next_start += entries.len() as u16;
            table.extend(entries);
        }
        Ok(table)
    }

    /// Upstream `bindingTable()`.
    pub async fn binding_table(&self) -> Result<Vec<Value>, String> {
        let cluster_id = ClusterId::BINDING_TABLE_REQUEST;
        let mut table = Vec::new();
        let mut request = |start_index: u16| {
            let ieee = self.ieee_addr();
            let network = self.network_address();
            async move {
                let payload = BuffaloZdo::build_request(
                    Entity::adapter().has_zdo_message_overhead(),
                    cluster_id,
                    &[json!(start_index)],
                );
                let response = Entity::adapter()
                    .send_zdo(&ieee, network, cluster_id, &payload, false)
                    .await?;
                if !BuffaloZdo::check_status(&response) {
                    return Err(zdo_status_error(&response));
                }
                let result = response.get(1).cloned().unwrap_or(Value::Null);
                let entries = result
                    .get("entryList")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let total = result
                    .get("bindingTableEntries")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                Ok::<(usize, Vec<Value>), String>((total, entries))
            }
        };
        let (size, entries) = request(0).await?;
        table.extend(entries);
        let mut next_start = table.len() as u16;
        while table.len() < size {
            let (_, entries) = request(next_start).await?;
            next_start += entries.len() as u16;
            table.extend(entries);
        }
        for endpoint in self.endpoints() {
            let mut new_binds = Vec::new();
            for entry in &table {
                if entry.get("sourceEui64").and_then(Value::as_str)
                    != Some(self.ieee_addr().as_str())
                    || entry.get("sourceEndpoint").and_then(Value::as_u64)
                        != Some(endpoint.ID as u64)
                {
                    continue;
                }
                let cluster = entry.get("clusterId").and_then(Value::as_u64).unwrap_or(0) as u16;
                if entry.get("destAddrMode").and_then(Value::as_u64) == Some(1) {
                    new_binds.push(BindInternal {
                        cluster,
                        r#type: "group".to_string(),
                        device_ieee_address: None,
                        endpoint_id: None,
                        group_id: entry.get("dest").and_then(Value::as_u64).map(|v| v as u16),
                    });
                } else {
                    new_binds.push(BindInternal {
                        cluster,
                        r#type: "endpoint".to_string(),
                        device_ieee_address: entry
                            .get("dest")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        endpoint_id: entry
                            .get("destEndpoint")
                            .and_then(Value::as_u64)
                            .map(|v| v as u8),
                        group_id: None,
                    });
                }
            }
            endpoint.save_bindings(new_binds);
        }
        Ok(table)
    }

    /// Upstream `clearAllBindings(eui64List)`.
    pub async fn clear_all_bindings(&self, eui64_list: Vec<String>) -> Result<(), String> {
        let cluster_id = ClusterId::CLEAR_ALL_BINDINGS_REQUEST;
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[json!({"eui64List": eui64_list})],
        );
        let response = Entity::adapter()
            .send_zdo(
                &self.ieee_addr(),
                self.network_address(),
                cluster_id,
                &payload,
                false,
            )
            .await?;
        if !BuffaloZdo::check_status(&response) {
            return Err(zdo_status_error(&response));
        }
        if (eui64_list.len() == 1 && eui64_list[0].to_lowercase() == "0x0000000000000000")
            || eui64_list
                .iter()
                .any(|eui| eui.to_lowercase() == self.ieee_addr())
        {
            for endpoint in self.endpoints() {
                endpoint.clear_bindings();
            }
        }
        Ok(())
    }

    /// Upstream `ping(disableRecovery = true)`.
    pub async fn ping(&self, disable_recovery: bool) -> Result<(), String> {
        let endpoints = self.endpoints();
        let endpoint = endpoints
            .iter()
            .find(|endpoint| {
                endpoint
                    .input_clusters
                    .lock()
                    .unwrap()
                    .contains(&GEN_BASIC_CLUSTER_ID)
            })
            .or_else(|| endpoints.first())
            .ok_or_else(|| "Device has no endpoints".to_string())?;
        endpoint
            .read(
                Value::from("genBasic"),
                json!(["zclVersion"]),
                Some(crate::controller::model::endpoint::Options {
                    disable_recovery: Some(disable_recovery),
                    send_policy: Some(crate::controller::tstype::SendPolicy::Immediate),
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    /// Upstream `addCustomCluster(name, cluster)`.
    pub fn add_custom_cluster(&self, name: &str, cluster: Cluster) -> Result<(), String> {
        if cluster.ID == 0x1000 || cluster.ID == 0x0021 {
            return Err(
                "Overriding of greenPower or touchlink cluster is not supported".to_string(),
            );
        }
        let mut custom_clusters = self.custom_clusters.lock().unwrap();
        if Utils::is_cluster_name(name) {
            let existing = custom_clusters
                .get(name)
                .cloned()
                .unwrap_or_else(|| Utils::get_cluster(name, None, &CustomClusters::empty()));
            if existing.ID != cluster.ID {
                return Err(format!(
                    "Custom cluster ID ({}) should match existing cluster ID ({})",
                    cluster.ID, existing.ID
                ));
            }
            let mut extended = existing;
            extended.name = cluster.name.clone();
            extended.ID = cluster.ID;
            extended.manufacturerCode = cluster.manufacturerCode;
            for (key, value) in cluster.attributes.clone() {
                extended.attributes.insert(key, value);
            }
            for (key, value) in cluster.commands.clone() {
                extended.commands.insert(key, value);
            }
            for (key, value) in cluster.commandsResponse.clone() {
                extended.commandsResponse.insert(key, value);
            }
            custom_clusters.insert(extended);
        } else {
            custom_clusters.insert(cluster);
        }
        Ok(())
    }

    fn arc_self(self: &Arc<Self>) -> Arc<Self> {
        self.clone()
    }
}

fn zdo_status_error(response: &Value) -> String {
    let code = response.get(0).and_then(Value::as_u64).unwrap_or(0) as u8;
    let name = ZdoStatus::from_u8(code)
        .map(ZdoStatus::name)
        .unwrap_or("UNKNOWN");
    format!("Status '{name}'")
}

fn matches_ts_model(model_id: &str) -> bool {
    model_id.starts_with("TS")
        && model_id[2..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn matches_tz_name(name: &str) -> bool {
    name.starts_with("_TZ") && name.contains('_')
}

fn model_matches(model_id: &str, pattern: &str) -> bool {
    match pattern {
        "^3R.*?Z" => model_id.starts_with("3R") && model_id.ends_with('Z'),
        "lumi..*" => model_id.starts_with("lumi") && model_id.len() >= 6,
        other => model_id == other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::events::ZclPayload;
    use crate::controller::database::Database;
    use crate::controller::model::entity::ControllerAdapter;
    use crate::controller::model::group::Group;
    use crate::zspec::zcl::zcl_header::{FrameControl, Header};

    struct FakeAdapter {
        frames: Mutex<Vec<Frame>>,
    }

    impl FakeAdapter {
        fn new() -> Self {
            Self {
                frames: Mutex::new(Vec::new()),
            }
        }
    }

    fn response_payload(cluster_id: u16, tsn: u8) -> ZclPayload {
        ZclPayload {
            cluster_id,
            data: vec![0, 0],
            header: Header::new(
                FrameControl {
                    reserved_bits: 0,
                    frame_type: FrameType::GLOBAL,
                    direction: Direction::SERVER_TO_CLIENT,
                    disable_default_response: true,
                    manufacturer_specific: false,
                },
                None,
                tsn,
                11,
            ),
            address: Value::String("0x00124b0000000001".to_string()),
            endpoint: 1,
            linkquality: 0,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 1,
        }
    }

    impl ControllerAdapter for FakeAdapter {
        fn has_zdo_message_overhead(&self) -> bool {
            false
        }
        fn send_zdo(
            &self,
            _ieee: &str,
            _nwk: u16,
            _cluster: u16,
            _payload: &[u8],
            _disable_response: bool,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + '_>>
        {
            Box::pin(async move { Ok(json!([0, {}])) })
        }
        fn send_zcl_frame_to_endpoint(
            &self,
            _ieee: &str,
            _nwk: u16,
            _endpoint: u8,
            frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
            _disable_recovery: bool,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Option<ZclPayload>, String>> + Send + '_>,
        > {
            self.frames.lock().unwrap().push(frame.clone());
            let tsn = frame.header.transaction_sequence_number;
            Box::pin(async move { Ok(Some(response_payload(6, tsn))) })
        }
        fn send_zcl_frame_to_group(
            &self,
            _group_id: u16,
            frame: &Frame,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        {
            self.frames.lock().unwrap().push(frame.clone());
            Box::pin(async move { Ok(()) })
        }
        fn send_zcl_frame_to_all(
            &self,
            _endpoint: u8,
            _frame: &Frame,
            _source_endpoint: u8,
            _destination: u16,
            _profile_id: Option<u16>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        {
            Box::pin(async move { Ok(()) })
        }
    }

    #[tokio::test]
    async fn device_endpoint_group_roundtrip_and_write_frame() {
        let _lock = crate::controller::model::test_support::serial();
        let path =
            std::env::temp_dir().join(format!("z2m-model-{}.db", crate::utils::datetime::now_ms()));
        Entity::inject_database(Arc::new(Mutex::new(Database::open(&path))));
        let adapter = Arc::new(FakeAdapter::new());
        Entity::inject_adapter(adapter.clone());
        Device::reset_cache();
        Group::reset_cache();

        let device = Device::create(
            DeviceType::Router,
            "0x00124b0000000001",
            0x1234,
            Some(0x1234),
            Some("Test".to_string()),
            Some("Mains (single phase)".to_string()),
            Some("TEST".to_string()),
            InterviewState::Successful,
            None,
        )
        .expect("create device");

        assert_eq!(
            Device::by_ieee_addr("0x00124b0000000001", false)
                .unwrap()
                .id(),
            device.id()
        );
        assert_eq!(
            Device::by_network_address(0x1234, false).unwrap().id(),
            device.id()
        );
        assert_eq!(Device::by_type(DeviceType::Router).len(), 1);

        let endpoint = device.create_endpoint(1).expect("create endpoint");
        assert!(device.get_endpoint(1).is_some());
        endpoint
            .save_cluster_attribute_key_value(Value::from("genBasic"), json!({"modelId": "TEST"}));
        assert_eq!(
            endpoint.get_cluster_attribute_value(Value::from("genBasic"), Value::from("modelId")),
            Some(json!("TEST"))
        );

        // Frame construction via a real write.
        endpoint
            .write(
                Value::from("genOnOff"),
                json!({"onOff": true}),
                Some(crate::controller::model::endpoint::Options {
                    disable_response: Some(true),
                    timeout: Some(1000),
                    ..Default::default()
                }),
            )
            .await
            .expect("write");
        let frames = adapter.frames.lock().unwrap();
        let written = frames.last().expect("frame");
        assert_eq!(written.cluster.name, "genOnOff");
        assert_eq!(written.command.name, "write");
        assert_eq!(written.payload[0]["attrId"], json!(0));
        assert_eq!(written.payload[0]["attrData"], json!(true));
        drop(frames);

        let group = Group::create(10).expect("create group");
        group.add_member(&endpoint);
        assert!(group.has_member(&endpoint));
        group
            .write(Value::from("genOnOff"), json!({"onOff": false}), None)
            .await
            .expect("group write");
        assert_eq!(group.members().len(), 1);

        // DB round-trip.
        device.save(true);
        group.save(true);
        Device::reset_cache();
        Group::reset_cache();
        let reloaded = Device::by_ieee_addr("0x00124b0000000001", false).expect("reload device");
        assert_eq!(reloaded.id(), device.id());
        let reloaded_endpoint = reloaded.get_endpoint(1).expect("reload endpoint");
        assert_eq!(
            reloaded_endpoint
                .get_cluster_attribute_value(Value::from("genBasic"), Value::from("modelId")),
            Some(json!("TEST"))
        );
        let reloaded_group = Group::by_group_id(10).expect("reload group");
        assert_eq!(reloaded_group.members().len(), 1);

        // changeIeeeAddress keeps the registry and endpoints in sync.
        reloaded.change_ieee_address("0x00124b0000000002".to_string());
        assert!(Device::by_ieee_addr("0x00124b0000000001", false).is_none());
        let changed = Device::by_ieee_addr("0x00124b0000000002", false).expect("changed");
        assert_eq!(
            *changed
                .get_endpoint(1)
                .unwrap()
                .device_ieee_address
                .lock()
                .unwrap(),
            "0x00124b0000000002"
        );

        let _ = std::fs::remove_file(path);
    }
}
