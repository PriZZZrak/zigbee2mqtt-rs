// Original: .upstream/zigbee-herdsman/src/controller/greenPower.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::adapter::events::ZclPayload;
use crate::controller::events::GreenPowerDeviceJoinedPayload;
use crate::controller::helpers::zcl_transaction_sequence_number;
use crate::controller::model::device::Device;
use crate::utils::logger::logger;
use crate::zspec::consts::{
    COORDINATOR_ADDRESS, GP_ENDPOINT, GP_GROUP_ID, INTEROPERABILITY_LINK_KEY,
};
use crate::zspec::enums::BroadcastAddress;
use crate::zspec::utils::{aes128_ccm_encrypt, aes128_ccm_star};
use crate::zspec::zcl::utils::CustomClusters;
use crate::zspec::zcl::zcl_frame::{try_from_buffer, Frame};
use crate::zspec::zcl::zcl_header::{Direction, FrameType};

const NS: &str = "zh:controller:greenpower";

/// Upstream `ZigbeeNWKGPAppId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZigbeeNwkGpAppId {
    Default = 0x00,
    Lped = 0x01,
    Zgp = 0x02,
}

/// Upstream `ZigbeeNWKGPSecurityLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZigbeeNwkGpSecurityLevel {
    No = 0x00,
    OneLsb = 0x01,
    Full = 0x02,
    FullEncr = 0x03,
}

/// Upstream `ZigbeeNWKGPSecurityKeyType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZigbeeNwkGpSecurityKeyType {
    NoKey = 0x00,
    ZbNwkKey = 0x01,
    GpdGroupKey = 0x02,
    NwkKeyDerivedGpdKeyGroupKey = 0x03,
    PreconfiguredIndividualGpdKey = 0x04,
    DerivedIndividualGpdKey = 0x07,
}

/// Upstream `GPCommunicationMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GpCommunicationMode {
    FullUnicast = 0,
    GroupcastToDgroupId = 1,
    GroupcastToPrecommissionedGroupId = 2,
    LightweightUnicast = 3,
}

/// Upstream `PairingOptions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingOptions {
    pub app_id: u8,
    pub add_sink: bool,
    pub remove_gpd: bool,
    pub communication_mode: u8,
    pub gpd_fixed: bool,
    pub gpd_mac_seq_num_capabilities: bool,
    pub security_level: u8,
    pub security_key_type: u8,
    pub gpd_security_frame_counter_present: bool,
    pub gpd_security_key_present: bool,
    pub assigned_alias_present: bool,
    pub groupcast_radius_present: bool,
}

/// Upstream `CommissioningModeOptions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommissioningModeOptions {
    pub action: u8,
    pub commissioning_window_present: bool,
    pub exit_mode: u8,
    pub channel_present: bool,
    pub unicast_communication: bool,
}

/// Upstream `GreenPowerEventMap` member `deviceJoined`.
pub enum GreenPowerEvent {
    DeviceJoined(GreenPowerDeviceJoinedPayload),
    DeviceLeave(u32),
}

type GreenPowerFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Upstream `getNetworkParameters()` result surface used by Green Power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GreenPowerNetworkParameters {
    pub channel: u8,
}

/// Controller-facing subset of the upstream `Adapter` used by GreenPower.
/// Implemented by the concrete adapter wrapper.
pub trait GreenPowerAdapter: Send + Sync {
    /// Upstream `getCoordinatorIEEE()`.
    fn get_coordinator_ieee(&self) -> GreenPowerFuture<'_, String>;

    /// Upstream `getNetworkParameters()`.
    fn get_network_parameters(&self) -> GreenPowerFuture<'_, GreenPowerNetworkParameters>;

    /// Upstream `sendZclFrameToAll(...)`.
    fn send_zcl_frame_to_all(
        &self,
        endpoint: u8,
        frame: &Frame,
        source_endpoint: u8,
        destination: u16,
        profile_id: Option<u16>,
    ) -> GreenPowerFuture<'_, ()>;

    /// Upstream `sendZclFrameToEndpoint(...)`.
    #[allow(clippy::too_many_arguments)]
    fn send_zcl_frame_to_endpoint(
        &self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
        disable_recovery: bool,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> GreenPowerFuture<'_, Option<ZclPayload>>;
}

type Listener = Arc<dyn Fn(&GreenPowerEvent) + Send + Sync>;

/// Upstream `class GreenPower extends EventEmitter<GreenPowerEventMap>`.
pub struct GreenPower {
    adapter: Arc<dyn GreenPowerAdapter>,
    listeners: Mutex<Vec<Listener>>,
}

fn payload_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(Value::as_u64)
}

fn buffer_like(value: &Value) -> Vec<u8> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(|item| item.as_u64().unwrap_or(0) as u8)
            .collect();
    }
    if let Some(text) = value.as_str() {
        return text.as_bytes().to_vec();
    }
    Vec::new()
}

impl GreenPower {
    /// Upstream `constructor(adapter)`.
    pub fn new(adapter: Arc<dyn GreenPowerAdapter>) -> Self {
        Self {
            adapter,
            listeners: Mutex::new(Vec::new()),
        }
    }

    /// Upstream `on(event, listener)` for `deviceJoined`/`deviceLeave`.
    pub fn on<F>(&self, listener: F)
    where
        F: Fn(&GreenPowerEvent) + Send + Sync + 'static,
    {
        self.listeners
            .lock()
            .expect("green power listeners poisoned")
            .push(Arc::new(listener));
    }

    fn emit(&self, event: &GreenPowerEvent) {
        let listeners = self
            .listeners
            .lock()
            .expect("green power listeners poisoned")
            .clone();
        for listener in listeners {
            listener(event);
        }
    }

    /// Upstream `static sourceIdToIeeeAddress(sourceId)`.
    pub fn source_id_to_ieee_address(source_id: u32) -> String {
        format!("0x{source_id:016x}")
    }

    /// Upstream `encryptSecurityKey(sourceID, securityKey)`.
    fn encrypt_security_key(&self, source_id: u32, security_key: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; 13];
        nonce[0..4].copy_from_slice(&source_id.to_le_bytes());
        nonce[4..8].copy_from_slice(&source_id.to_le_bytes());
        nonce[8..12].copy_from_slice(&source_id.to_le_bytes());
        nonce[12] = 0x05;

        aes128_ccm_encrypt(&INTEROPERABILITY_LINK_KEY, &nonce, security_key)
    }

    /// Upstream `decryptPayload(sourceID, frameCounter, securityKey, payload)`.
    fn decrypt_payload(
        &self,
        source_id: u32,
        frame_counter: u32,
        security_key: &[u8],
        payload: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut nonce = [0u8; 13];
        nonce[0..4].copy_from_slice(&source_id.to_le_bytes());
        nonce[4..8].copy_from_slice(&source_id.to_le_bytes());
        nonce[8..12].copy_from_slice(&frame_counter.to_le_bytes());
        nonce[12] = 0x05;

        let key: [u8; 16] = security_key
            .try_into()
            .map_err(|_| "Invalid green power security key length".to_string())?;
        let (_, decrypted) = aes128_ccm_star(4, &key, &nonce, payload);
        Ok(decrypted)
    }

    /// Upstream `static encodePairingOptions(options)` — greenPower.ts:151.
    pub fn encode_pairing_options(options: &PairingOptions) -> u32 {
        (u32::from(options.app_id) & 0x7)
            | (((options.add_sink as u32) << 3) & 0x8)
            | (((options.remove_gpd as u32) << 4) & 0x10)
            | ((u32::from(options.communication_mode) << 5) & 0x60)
            | (((options.gpd_fixed as u32) << 7) & 0x80)
            | (((options.gpd_mac_seq_num_capabilities as u32) << 8) & 0x100)
            | ((u32::from(options.security_level) << 9) & 0x600)
            | ((u32::from(options.security_key_type) << 11) & 0x3800)
            | (((options.gpd_security_frame_counter_present as u32) << 14) & 0x4000)
            | (((options.gpd_security_key_present as u32) << 15) & 0x8000)
            | (((options.assigned_alias_present as u32) << 16) & 0x10000)
            | (((options.groupcast_radius_present as u32) << 17) & 0x20000)
        // bits 18..23 reserved
    }

    /// Upstream `static decodePairingOptions(byte)` — greenPower.ts:169.
    pub fn decode_pairing_options(byte: u32) -> PairingOptions {
        PairingOptions {
            app_id: (byte & 0x7) as u8,
            add_sink: (byte & 0x8) >> 3 == 1,
            remove_gpd: (byte & 0x10) >> 4 == 1,
            communication_mode: ((byte & 0x60) >> 5) as u8,
            gpd_fixed: (byte & 0x80) >> 7 == 1,
            gpd_mac_seq_num_capabilities: (byte & 0x100) >> 8 == 1,
            security_level: ((byte & 0x600) >> 9) as u8,
            security_key_type: ((byte & 0x3800) >> 11) as u8,
            gpd_security_frame_counter_present: (byte & 0x4000) >> 14 == 1,
            gpd_security_key_present: (byte & 0x8000) >> 15 == 1,
            assigned_alias_present: (byte & 0x10000) >> 16 == 1,
            groupcast_radius_present: (byte & 0x20000) >> 17 == 1,
            // bits 18..23 reserved
        }
    }

    /// Upstream `sendPairingCommand(options, payload, gppNwkAddr)` — greenPower.ts:188.
    async fn send_pairing_command(
        &self,
        options: &PairingOptions,
        payload: Value,
        gpp_nwk_addr: Option<u16>,
    ) -> Result<Option<ZclPayload>, String> {
        let mut payload = payload;
        let encoded_options = Self::encode_pairing_options(options);
        let object = payload
            .as_object_mut()
            .ok_or_else(|| "Pairing payload must be an object".to_string())?;
        object.insert("options".to_string(), json!(encoded_options));

        let src_id = payload_u64(&payload, "srcID")
            .map(|value| value.to_string())
            .unwrap_or_else(|| "undefined".to_string());
        let gpp = gpp_nwk_addr
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NO".to_string());
        logger().debug(
            &format!(
                "[PAIRING] srcID={src_id} gpp={gpp} options={encoded_options} (addSink={} commMode={})",
                options.add_sink, options.communication_mode
            ),
            NS,
        );

        if let Some(object) = payload.as_object_mut() {
            match options.communication_mode {
                value
                    if value == GpCommunicationMode::GroupcastToPrecommissionedGroupId as u8
                        || value == GpCommunicationMode::GroupcastToDgroupId as u8 =>
                {
                    object.insert("sinkGroupID".to_string(), json!(GP_GROUP_ID));
                }
                value
                    if value == GpCommunicationMode::FullUnicast as u8
                        || value == GpCommunicationMode::LightweightUnicast as u8 =>
                {
                    object.insert(
                        "sinkIEEEAddr".to_string(),
                        json!(self.adapter.get_coordinator_ieee().await?),
                    );
                    object.insert("sinkNwkAddr".to_string(), json!(COORDINATOR_ADDRESS));
                }
                _ => {}
            }
        }

        let reply_frame = Frame::create(
            FrameType::SPECIFIC as u8,
            Direction::SERVER_TO_CLIENT as u8,
            true,
            None,
            zcl_transaction_sequence_number::next(),
            "pairing",
            "greenPower",
            payload,
            json!({}),
            0,
        );

        if options.communication_mode != GpCommunicationMode::LightweightUnicast as u8 {
            self.adapter
                .send_zcl_frame_to_all(
                    GP_ENDPOINT,
                    &reply_frame,
                    GP_ENDPOINT,
                    BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                    None,
                )
                .await?;
            return Ok(None);
        }

        let device = Device::by_network_address(gpp_nwk_addr.unwrap_or(COORDINATOR_ADDRESS), false)
            .ok_or_else(|| "Failed to find green power proxy device".to_string())?;

        self.adapter
            .send_zcl_frame_to_endpoint(
                &device.ieee_addr(),
                device.network_address(),
                GP_ENDPOINT,
                &reply_frame,
                10000,
                false,
                false,
                Some(GP_ENDPOINT),
                None,
            )
            .await
    }

    /// Upstream `processCommand(dataPayload, frame, securityKey)` — greenPower.ts:247.
    pub async fn process_command(
        &self,
        data_payload: &ZclPayload,
        mut frame: Frame,
        security_key: Option<&[u8]>,
    ) -> Frame {
        match self
            .process_command_fallible(data_payload, &mut frame, security_key)
            .await
        {
            Ok(()) => frame,
            Err(error) => {
                logger().error(&error, NS);
                frame
            }
        }
    }

    async fn process_command_fallible(
        &self,
        data_payload: &ZclPayload,
        frame: &mut Frame,
        security_key: Option<&[u8]>,
    ) -> Result<(), String> {
        // notification: A.3.3.4.1
        // commissioningNotification: A.3.3.4.3
        let is_commissioning_notification =
            frame.header.command_identifier == COMMISSIONING_NOTIFICATION_COMMAND_ID;
        let security_level = if is_commissioning_notification {
            (payload_u64(&frame.payload, "options").unwrap_or(0) >> 4) & 0x3
        } else {
            (payload_u64(&frame.payload, "options").unwrap_or(0) >> 6) & 0x3
        };

        if security_level == ZigbeeNwkGpSecurityLevel::FullEncr as u64
            && (!is_commissioning_notification
                || ((payload_u64(&frame.payload, "options").unwrap_or(0) >> 9) & 0x1) == 1)
        /* security processing failed */
        {
            let Some(security_key) = security_key else {
                let src_id = payload_u64(&frame.payload, "srcID")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "undefined".to_string());
                let gpp = payload_u64(&frame.payload, "gppNwkAddr")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "NO".to_string());
                logger().error(
                    &format!(
                        "[FULLENCR] srcID={src_id} gpp={gpp} commandIdentifier={} Unknown security key",
                        frame.header.command_identifier
                    ),
                    NS,
                );
                return Ok(());
            };

            let old_header = data_payload.data.get(0..15).unwrap_or_default();
            let mut data_end_offset = data_payload.data.len();

            if is_commissioning_notification {
                let options = payload_u64(&frame.payload, "options").unwrap_or(0);
                let has_mic = options & 0x200 != 0;
                let has_gpp_data = options & 0x800 != 0;

                if has_gpp_data {
                    data_end_offset = data_end_offset.saturating_sub(3);
                }

                if has_mic {
                    data_end_offset = data_end_offset.saturating_sub(4);
                }
            } else {
                let has_gpp_data =
                    payload_u64(&frame.payload, "options").unwrap_or(0) & 0x4000 != 0;

                if has_gpp_data {
                    data_end_offset = data_end_offset.saturating_sub(3);
                }
            }

            let source_id = payload_u64(&frame.payload, "srcID").unwrap_or(0) as u32;
            let frame_counter = payload_u64(&frame.payload, "frameCounter").unwrap_or(0) as u32;
            let command_id = payload_u64(&frame.payload, "commandID").unwrap_or(0) as u8;
            let hashed_key = self.encrypt_security_key(source_id, security_key);
            // 4 bytes appended for MIC placeholder (just needs the bytes present for decrypt)
            let mut payload = vec![command_id];
            payload.extend_from_slice(
                data_payload
                    .data
                    .get(15..data_end_offset)
                    .unwrap_or_default(),
            );
            payload.extend_from_slice(&[0, 0, 0, 0]);
            let decrypted =
                self.decrypt_payload(source_id, frame_counter, &hashed_key, &payload)?;

            let mut new_header = vec![0u8; 15];
            let old_len = old_header.len().min(15);
            new_header[..old_len].copy_from_slice(&old_header[..old_len]);

            let options = payload_u64(&frame.payload, "options").unwrap_or(0);
            let masked_options = if is_commissioning_notification {
                (options & !0x30 & !0x200) as u16
            } else {
                (options & !0xc0) as u16
            };
            new_header[3..5].copy_from_slice(&masked_options.to_le_bytes());
            new_header[13] = decrypted.first().copied().unwrap_or(0);
            new_header[14] = decrypted.len().saturating_sub(1) as u8;

            // re-parse with decrypted data
            let mut reparsed = new_header;
            reparsed.extend_from_slice(decrypted.get(1..).unwrap_or_default());
            reparsed
                .extend_from_slice(data_payload.data.get(data_end_offset..).unwrap_or_default());

            *frame = try_from_buffer(
                data_payload.cluster_id,
                data_payload.header,
                &reparsed,
                &CustomClusters::empty(),
            )
            .map_err(|error| error.to_string())?;
        }

        let log_str = match payload_u64(&frame.payload, "gppGpdLink") {
            Some(gpp_gpd_link) => {
                let rssi = gpp_gpd_link & 0x3f;
                let link_quality = (gpp_gpd_link >> 6) & 0x3;
                let link_quality_str = match link_quality {
                    0b00 => "Poor",
                    0b01 => "Moderate",
                    0b10 => "High",
                    0b11 => "Excellent",
                    _ => "Unknown",
                };
                format!(
                    "srcID={} gpp={} rssi={rssi} linkQuality={link_quality_str}",
                    payload_u64(&frame.payload, "srcID")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "undefined".to_string()),
                    payload_u64(&frame.payload, "gppNwkAddr")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "undefined".to_string()),
                )
            }
            None => format!(
                "srcID={} gpp=NO",
                payload_u64(&frame.payload, "srcID")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "undefined".to_string()),
            ),
        };

        let command_id = payload_u64(&frame.payload, "commandID").unwrap_or(0);
        let command_frame = frame
            .payload
            .get("commandFrame")
            .cloned()
            .unwrap_or(Value::Null);
        let src_id = payload_u64(&frame.payload, "srcID").unwrap_or(0) as u32;
        let gpp_nwk_addr = payload_u64(&frame.payload, "gppNwkAddr").map(|value| value as u16);

        match command_id {
            0xe0 => {
                logger().info(&format!("[COMMISSIONING] {log_str}"), NS);

                let options = payload_u64(&frame.payload, "options").unwrap_or(0);
                if options & 0x200 != 0 {
                    logger().warning(
                        &format!("[COMMISSIONING] {log_str} Security processing marked as failed"),
                        NS,
                    );
                }

                let command_frame_options = payload_u64(&command_frame, "options").unwrap_or(0);
                let rx_on_cap = command_frame_options & 0x2 != 0;
                let gpd_mac_seq_num_capabilities = command_frame_options & 0x1 != 0;
                let gpd_fixed = command_frame_options & 0x40 != 0;
                let device_id = payload_u64(&command_frame, "deviceID").unwrap_or(0) as u8;

                if rx_on_cap {
                    // RX capable GPD needs GP Commissioning Reply
                    logger().debug(
                        &format!(
                            "[COMMISSIONING] {log_str} GPD has receiving capabilities in operational mode (RxOnCapability)"
                        ),
                        NS,
                    );
                    // NOTE: currently encryption is disabled for RX capable GPDs

                    let network_parameters = self.adapter.get_network_parameters().await?;
                    // Commissioning reply
                    let payload_response = json!({
                        "options": 0,
                        "tempMaster": u64::from(gpp_nwk_addr.unwrap_or(COORDINATOR_ADDRESS)),
                        "tempMasterTx": i64::from(network_parameters.channel) - 11,
                        "srcID": src_id,
                        "gpdCmd": 0xf0,
                        "gpdPayload": {
                            "commandID": 0xf0,
                            "options": 0,
                            // panID: number,
                            // securityKey: frame.payload.commandFrame.securityKey,
                            // keyMic: frame.payload.commandFrame.keyMic,
                            // frameCounter: number,
                        },
                    });

                    let reply_frame = Frame::create(
                        FrameType::SPECIFIC as u8,
                        Direction::SERVER_TO_CLIENT as u8,
                        true,
                        None,
                        zcl_transaction_sequence_number::next(),
                        "response",
                        "greenPower",
                        payload_response,
                        json!({}),
                        0,
                    );

                    self.adapter
                        .send_zcl_frame_to_all(
                            GP_ENDPOINT,
                            &reply_frame,
                            GP_ENDPOINT,
                            BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                            None,
                        )
                        .await?;

                    self.send_pairing_command(
                        &PairingOptions {
                            app_id: ZigbeeNwkGpAppId::Default as u8,
                            add_sink: true,
                            remove_gpd: false,
                            communication_mode: GpCommunicationMode::GroupcastToDgroupId as u8,
                            gpd_fixed,
                            gpd_mac_seq_num_capabilities,
                            security_level: ZigbeeNwkGpSecurityLevel::No as u8,
                            security_key_type: ZigbeeNwkGpSecurityKeyType::NoKey as u8,
                            gpd_security_frame_counter_present: false,
                            gpd_security_key_present: false,
                            assigned_alias_present: false,
                            groupcast_radius_present: false,
                        },
                        json!({
                            "options": 0, // set from first param in `sendPairingCommand`
                            "srcID": src_id,
                            "deviceID": device_id,
                        }),
                        gpp_nwk_addr,
                    )
                    .await?;
                } else {
                    let gpd_key = self.encrypt_security_key(
                        src_id,
                        &buffer_like(command_frame.get("securityKey").unwrap_or(&Value::Null)),
                    );
                    let pairing_rsp = self
                        .send_pairing_command(
                            &PairingOptions {
                                app_id: ZigbeeNwkGpAppId::Default as u8,
                                add_sink: true,
                                remove_gpd: false,
                                // keep communication mode matching incoming tx mode
                                communication_mode: if data_payload.was_broadcast {
                                    GpCommunicationMode::GroupcastToPrecommissionedGroupId as u8
                                } else {
                                    GpCommunicationMode::LightweightUnicast as u8
                                },
                                gpd_fixed,
                                gpd_mac_seq_num_capabilities,
                                security_level: ZigbeeNwkGpSecurityLevel::Full as u8,
                                security_key_type:
                                    ZigbeeNwkGpSecurityKeyType::PreconfiguredIndividualGpdKey as u8,
                                gpd_security_frame_counter_present: true,
                                gpd_security_key_present: true,
                                assigned_alias_present: false,
                                groupcast_radius_present: false,
                            },
                            json!({
                                "options": 0, // set from first param in `sendPairingCommand`
                                "srcID": src_id,
                                "deviceID": device_id,
                                "frameCounter": payload_u64(&command_frame, "outgoingCounter").unwrap_or(0),
                                "gpdKey": gpd_key,
                            }),
                            gpp_nwk_addr,
                        )
                        .await?;

                    // undefined if wasBroadcast
                    if let Some(pairing_rsp) = pairing_rsp {
                        let pairing_rsp_frame = Frame::from_buffer(
                            pairing_rsp.cluster_id,
                            pairing_rsp.header,
                            &pairing_rsp.data,
                            &CustomClusters::empty(),
                        );

                        let status_code =
                            payload_u64(&pairing_rsp_frame.payload, "statusCode").unwrap_or(0);
                        if status_code != 0 {
                            return Err(format!("ZCL status error '{status_code}'"));
                        }
                    }
                }

                self.emit(&GreenPowerEvent::DeviceJoined(
                    GreenPowerDeviceJoinedPayload {
                        source_id: src_id,
                        device_id: u32::from(device_id),
                        // XXX: this has the potential to create conflicting network addresses
                        network_address: (src_id & 0xffff) as u16,
                        security_key: command_frame
                            .get("securityKey")
                            .filter(|value| !value.is_null())
                            .map(buffer_like),
                    },
                ));
            }
            0xe1 => {
                logger().debug(&format!("[DECOMMISSIONING] {log_str}"), NS);

                self.send_pairing_command(
                    &PairingOptions {
                        app_id: ZigbeeNwkGpAppId::Default as u8,
                        add_sink: false,
                        remove_gpd: true,
                        communication_mode: GpCommunicationMode::GroupcastToDgroupId as u8,
                        gpd_fixed: false,
                        gpd_mac_seq_num_capabilities: false,
                        security_level: ZigbeeNwkGpSecurityLevel::No as u8,
                        security_key_type: ZigbeeNwkGpSecurityKeyType::NoKey as u8,
                        gpd_security_frame_counter_present: false,
                        gpd_security_key_present: false,
                        assigned_alias_present: false,
                        groupcast_radius_present: false,
                    },
                    json!({
                        "options": 0, // set from first param in `sendPairingCommand`
                        "srcID": src_id,
                    }),
                    gpp_nwk_addr,
                )
                .await?;

                self.emit(&GreenPowerEvent::DeviceLeave(src_id));
            }
            0xe2 => {
                logger().debug(&format!("[SUCCESS] {log_str}"), NS);
            }
            0xe3 => {
                logger().debug(&format!("[CHANNEL_REQUEST] {log_str}"), NS);
                let network_parameters = self.adapter.get_network_parameters().await?;
                // Channel notification
                let payload = json!({
                    "options": 0,
                    "tempMaster": u64::from(gpp_nwk_addr.unwrap_or(COORDINATOR_ADDRESS)),
                    "tempMasterTx": payload_u64(&command_frame, "nextChannel").unwrap_or(0),
                    "srcID": src_id,
                    "gpdCmd": 0xf3,
                    "gpdPayload": {
                        "commandID": 0xf3,
                        "operationalChannel": i64::from(network_parameters.channel) - 11,
                        // If EITHER the sink is a GP Basic sink OR the sink is a GP Advanced sink,
                        // but all of the candidate TempMasters are GP Basic proxies (as indicated by the BidirectionalCommunicationCapability
                        // sub-field of the Options field of the received GP Commissioning Notification set to 0b0),
                        // the sink SHALL set the Basic sub-field of the Channel field to 0b1.
                        "basic": true,
                    },
                });

                let reply_frame = Frame::create(
                    FrameType::SPECIFIC as u8,
                    Direction::SERVER_TO_CLIENT as u8,
                    true,
                    None,
                    zcl_transaction_sequence_number::next(),
                    "response",
                    "greenPower",
                    payload,
                    json!({}),
                    0,
                );

                self.adapter
                    .send_zcl_frame_to_all(
                        GP_ENDPOINT,
                        &reply_frame,
                        GP_ENDPOINT,
                        BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                        None,
                    )
                    .await?;
            }
            0xe4 => {
                logger().debug(&format!("[APP_DESCRIPTION] {log_str}"), NS);
            }
            0xa0 | 0xa1 | 0xa2 | 0xa3 | 0xa4 | 0xa5 => {}
            _ => {
                // NOTE: this is spammy because it logs everything that is handed back to Controller without special processing here
                logger().debug(
                    &format!("[UNHANDLED_CMD/PASSTHROUGH] command=0x{command_id:x} {log_str}"),
                    NS,
                );
            }
        }

        Ok(())
    }

    /// Upstream `static encodeCommissioningModeOptions(options)` — greenPower.ts:563.
    pub fn encode_commissioning_mode_options(options: &CommissioningModeOptions) -> u8 {
        ((options.action & 0x1)
            | (((options.commissioning_window_present as u8) << 1) & 0x2)
            | ((options.exit_mode << 2) & 0x0c)
            | (((options.channel_present as u8) << 4) & 0x10)
            | (((options.unicast_communication as u8) << 5) & 0x20)) as u8
    }

    /// Upstream `static decodeCommissioningModeOptions(byte)` — greenPower.ts:573.
    pub fn decode_commissioning_mode_options(byte: u8) -> CommissioningModeOptions {
        CommissioningModeOptions {
            action: byte & 0x1,
            commissioning_window_present: (byte & 0x2) >> 1 == 1,
            exit_mode: (byte & 0x0c) >> 2,
            channel_present: (byte & 0x10) >> 4 == 1,
            unicast_communication: (byte & 0x20) >> 5 == 1,
        }
    }

    /// Upstream `permitJoin(time, networkAddress)` — greenPower.ts:583.
    pub async fn permit_join(&self, time: u16, network_address: Option<u16>) -> Result<(), String> {
        let payload = json!({
            "options": Self::encode_commissioning_mode_options(&CommissioningModeOptions {
                action: if time > 0 { 1 } else { 0 },
                commissioning_window_present: true,
                exit_mode: 0b10,
                channel_present: false,
                unicast_communication: network_address.is_some(),
            }),
            "commisioningWindow": time,
        });

        let frame = Frame::create(
            FrameType::SPECIFIC as u8,
            Direction::SERVER_TO_CLIENT as u8,
            true, // avoid receiving many responses, especially from the nodes not supporting this functionality
            None,
            zcl_transaction_sequence_number::next(),
            "commisioningMode",
            "greenPower",
            payload,
            json!({}),
            0,
        );

        match network_address {
            None => {
                self.adapter
                    .send_zcl_frame_to_all(
                        GP_ENDPOINT,
                        &frame,
                        GP_ENDPOINT,
                        BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                        None,
                    )
                    .await?;
            }
            Some(network_address) => {
                let device = Device::by_network_address(network_address, false)
                    .ok_or_else(|| "Failed to find device to permit GP join on".to_string())?;

                self.adapter
                    .send_zcl_frame_to_endpoint(
                        &device.ieee_addr(),
                        network_address,
                        GP_ENDPOINT,
                        &frame,
                        10000,
                        false,
                        false,
                        Some(GP_ENDPOINT),
                        None,
                    )
                    .await?;
            }
        }

        Ok(())
    }
}

/// Upstream `COMMISSIONING_NOTIFICATION_COMMAND_ID`.
const COMMISSIONING_NOTIFICATION_COMMAND_ID: u8 = 0x04;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::events::GreenPowerDeviceJoinedPayload;
    use crate::zspec::zcl::zcl_header::Header;

    #[derive(Default)]
    struct RecordingAdapter {
        sent_all: Mutex<Vec<Frame>>,
        sent_endpoint: Mutex<Vec<(String, u16, u8, Frame)>>,
        channel: u8,
    }

    impl RecordingAdapter {
        fn new(channel: u8) -> Self {
            Self {
                channel,
                ..Default::default()
            }
        }
    }

    impl GreenPowerAdapter for RecordingAdapter {
        fn get_coordinator_ieee(&self) -> GreenPowerFuture<'_, String> {
            Box::pin(async move { Ok("0x00124b0000000001".to_string()) })
        }

        fn get_network_parameters(&self) -> GreenPowerFuture<'_, GreenPowerNetworkParameters> {
            let channel = self.channel;
            Box::pin(async move { Ok(GreenPowerNetworkParameters { channel }) })
        }

        fn send_zcl_frame_to_all(
            &self,
            _endpoint: u8,
            frame: &Frame,
            _source_endpoint: u8,
            _destination: u16,
            _profile_id: Option<u16>,
        ) -> GreenPowerFuture<'_, ()> {
            self.sent_all.lock().unwrap().push(frame.clone());
            Box::pin(async move { Ok(()) })
        }

        fn send_zcl_frame_to_endpoint(
            &self,
            ieee_address: &str,
            network_address: u16,
            endpoint: u8,
            frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
            _disable_recovery: bool,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> GreenPowerFuture<'_, Option<ZclPayload>> {
            self.sent_endpoint.lock().unwrap().push((
                ieee_address.to_string(),
                network_address,
                endpoint,
                frame.clone(),
            ));
            Box::pin(async move { Ok(None) })
        }
    }

    fn u16le(value: u16) -> [u8; 2] {
        value.to_le_bytes()
    }

    fn u32le(value: u32) -> [u8; 4] {
        value.to_le_bytes()
    }

    fn make_header(
        sequence_number: u8,
        command_identifier: u8,
        options: u16,
        source_id: u32,
        frame_counter: u32,
        gpd_command_id: u8,
        payload_length: u8,
    ) -> Vec<u8> {
        let mut header = vec![
            0b00000001, // frameControl: FrameType.SPECIFIC + Direction.CLIENT_TO_SERVER + disableDefaultResponse=false
            sequence_number,
            command_identifier,
        ];
        header.extend_from_slice(&u16le(options));
        header.extend_from_slice(&u32le(source_id));
        header.extend_from_slice(&u32le(frame_counter));
        header.push(gpd_command_id);
        header.push(payload_length);
        header
    }

    fn make_payload(source_id: u32, data: Vec<u8>, was_broadcast: bool) -> ZclPayload {
        let header = Header::from_buffer(&data).expect("header");
        ZclPayload {
            cluster_id: 0x0021,
            data,
            header: Some(header),
            address: json!(source_id & 0xffff),
            endpoint: GP_ENDPOINT,
            linkquality: 138,
            group_id: GP_GROUP_ID,
            was_broadcast,
            destination_endpoint: GP_ENDPOINT,
        }
    }

    fn parse_frame(payload: &ZclPayload) -> Frame {
        Frame::from_buffer(
            payload.cluster_id,
            payload.header,
            &payload.data,
            &CustomClusters::empty(),
        )
    }

    #[test]
    fn encodes_and_decodes_pairing_options() {
        let cases: Vec<(u32, PairingOptions)> = vec![
            (
                0b000000000110101000,
                PairingOptions {
                    app_id: 0,
                    add_sink: true,
                    remove_gpd: false,
                    communication_mode: 0b01,
                    gpd_fixed: true,
                    gpd_mac_seq_num_capabilities: true,
                    security_level: 0,
                    security_key_type: 0,
                    gpd_security_frame_counter_present: false,
                    gpd_security_key_present: false,
                    assigned_alias_present: false,
                    groupcast_radius_present: false,
                },
            ),
            (
                0b001110010101001000,
                PairingOptions {
                    app_id: 0,
                    add_sink: true,
                    remove_gpd: false,
                    communication_mode: 0b10,
                    gpd_fixed: false,
                    gpd_mac_seq_num_capabilities: true,
                    security_level: 0b10,
                    security_key_type: 0b100,
                    gpd_security_frame_counter_present: true,
                    gpd_security_key_present: true,
                    assigned_alias_present: false,
                    groupcast_radius_present: false,
                },
            ),
            (
                0b001110010101101000,
                PairingOptions {
                    app_id: 0,
                    add_sink: true,
                    remove_gpd: false,
                    communication_mode: 0b11,
                    gpd_fixed: false,
                    gpd_mac_seq_num_capabilities: true,
                    security_level: 0b10,
                    security_key_type: 0b100,
                    gpd_security_frame_counter_present: true,
                    gpd_security_key_present: true,
                    assigned_alias_present: false,
                    groupcast_radius_present: false,
                },
            ),
            (
                0b000000000110110000,
                PairingOptions {
                    app_id: 0,
                    add_sink: false,
                    remove_gpd: true,
                    communication_mode: 0b01,
                    gpd_fixed: true,
                    gpd_mac_seq_num_capabilities: true,
                    security_level: 0b00,
                    security_key_type: 0b000,
                    gpd_security_frame_counter_present: false,
                    gpd_security_key_present: false,
                    assigned_alias_present: false,
                    groupcast_radius_present: false,
                },
            ),
            (
                0b110000000010110000,
                PairingOptions {
                    app_id: 0,
                    add_sink: false,
                    remove_gpd: true,
                    communication_mode: 0b01,
                    gpd_fixed: true,
                    gpd_mac_seq_num_capabilities: false,
                    security_level: 0b00,
                    security_key_type: 0b000,
                    gpd_security_frame_counter_present: false,
                    gpd_security_key_present: false,
                    assigned_alias_present: true,
                    groupcast_radius_present: true,
                },
            ),
        ];

        for (raw_byte, raw_options) in cases {
            assert_eq!(GreenPower::decode_pairing_options(raw_byte), raw_options);
            assert_eq!(GreenPower::encode_pairing_options(&raw_options), raw_byte);
        }
    }

    #[test]
    fn encodes_and_decodes_commissioning_mode_options() {
        let cases = [
            (
                0x0bu8,
                CommissioningModeOptions {
                    action: 1,
                    commissioning_window_present: true,
                    exit_mode: 0b10,
                    channel_present: false,
                    unicast_communication: false,
                },
            ),
            (
                0x2b,
                CommissioningModeOptions {
                    action: 1,
                    commissioning_window_present: true,
                    exit_mode: 0b10,
                    channel_present: false,
                    unicast_communication: true,
                },
            ),
            (
                0x0a,
                CommissioningModeOptions {
                    action: 0,
                    commissioning_window_present: true,
                    exit_mode: 0b10,
                    channel_present: false,
                    unicast_communication: false,
                },
            ),
            (
                0b111100,
                CommissioningModeOptions {
                    action: 0,
                    commissioning_window_present: false,
                    exit_mode: 0b11,
                    channel_present: true,
                    unicast_communication: true,
                },
            ),
        ];

        for (raw_byte, raw_options) in cases {
            assert_eq!(
                GreenPower::decode_commissioning_mode_options(raw_byte),
                raw_options
            );
            assert_eq!(
                GreenPower::encode_commissioning_mode_options(&raw_options),
                raw_byte
            );
        }
    }

    #[tokio::test]
    async fn omits_gpp_data_from_raw_payload() {
        let _guard = crate::controller::model::test_support::serial();
        let adapter = Arc::new(RecordingAdapter::new(15));
        let gp = GreenPower::new(adapter);

        let source_id = 2777252112u32;
        let options = 0x800u16;
        let gpd_command_payload = vec![0x3e];
        let mut data = make_header(18, 0x04, options, source_id, 17326, 38, 1);
        data.extend_from_slice(&gpd_command_payload);
        data.extend_from_slice(&u16le(24404));
        data.push(207);

        let payload = make_payload(source_id, data, true);
        let frame = parse_frame(&payload);

        assert_eq!(frame.payload["gppNwkAddr"], json!(24404));
        assert_eq!(frame.payload["gppGpdLink"], json!(207));

        let ret_frame = gp.process_command(&payload, frame, None).await;
        assert_eq!(
            ret_frame.payload["commandFrame"],
            json!({"raw": gpd_command_payload})
        );
        assert_eq!(ret_frame.payload["gppNwkAddr"], json!(24404));
        assert_eq!(ret_frame.payload["gppGpdLink"], json!(207));
    }

    #[tokio::test]
    async fn omits_mic_from_raw_payload_after_decryption() {
        let _guard = crate::controller::model::test_support::serial();
        let adapter = Arc::new(RecordingAdapter::new(15));
        let gp = GreenPower::new(adapter);

        let security_key: [u8; 16] = [
            227, 227, 225, 134, 235, 104, 141, 250, 162, 211, 104, 147, 201, 146, 67, 175,
        ];
        let source_id = 2777252112u32;
        let options = 0x30u16 | 0x200u16;
        let mut data = make_header(18, 0x04, options, source_id, 17326, 38, 1);
        data.push(0x3e);
        data.extend_from_slice(&u32le(1441399364));

        let payload = make_payload(source_id, data, true);
        let frame = parse_frame(&payload);
        assert!(frame.payload.get("mic").is_some());

        let ret_frame = gp
            .process_command(&payload, frame, Some(&security_key))
            .await;
        assert_eq!(ret_frame.payload["commandID"], json!(0x21));
        assert_eq!(ret_frame.payload["commandFrame"], json!({"raw": [207]}));
        assert!(ret_frame.payload.get("mic").is_none());
    }

    #[tokio::test]
    async fn channel_request_replies_with_channel_configuration() {
        let _guard = crate::controller::model::test_support::serial();
        let adapter = Arc::new(RecordingAdapter::new(25));
        let gp = GreenPower::new(adapter.clone());

        let source_id = 2888399791u32;
        let mut data = make_header(143, 0x00, 0, source_id, 3727, 0xe3, 1);
        data.push(0x0c);
        let payload = make_payload(source_id, data, true);
        let frame = parse_frame(&payload);

        let ret_frame = gp.process_command(&payload, frame, None).await;
        assert_eq!(ret_frame.payload["commandFrame"]["nextChannel"], json!(12));
        assert_eq!(
            ret_frame.payload["commandFrame"]["nextNextChannel"],
            json!(0)
        );

        let sent = adapter.sent_all.lock().unwrap();
        assert_eq!(sent.len(), 1);
        let response = &sent[0];
        assert_eq!(response.command.name, "response");
        assert_eq!(response.payload["tempMasterTx"], json!(12));
        assert_eq!(response.payload["gpdCmd"], json!(0xf3));
        assert_eq!(response.payload["gpdPayload"]["commandID"], json!(0xf3));
        assert_eq!(
            response.payload["gpdPayload"]["operationalChannel"],
            json!(14)
        );
        assert_eq!(response.payload["gpdPayload"]["basic"], json!(true));
        // GPD channel configuration payload: length=1, (channel & 0xf) | basic << 4.
        assert_eq!(response.to_buffer().ends_with(&[1, 14 | 0x10]), true);
    }

    #[tokio::test]
    async fn commissioning_notification_emits_device_joined_and_pairing_reply() {
        let _guard = crate::controller::model::test_support::serial();
        let adapter = Arc::new(RecordingAdapter::new(15));
        let gp = GreenPower::new(adapter.clone());
        let joined: Arc<Mutex<Option<GreenPowerDeviceJoinedPayload>>> = Arc::new(Mutex::new(None));
        let captured = joined.clone();
        gp.on(move |event| {
            if let GreenPowerEvent::DeviceJoined(payload) = event {
                *captured.lock().unwrap() = Some(payload.clone());
            }
        });

        let source_id = 1496140231u32;
        let command_payload = (0.."0289f31adb70a88d71196ee50c03580537767de27ad5331309000037647a62697061304047503030303157".len())
            .step_by(2)
            .map(|index| {
                u8::from_str_radix(
                    &"0289f31adb70a88d71196ee50c03580537767de27ad5331309000037647a62697061304047503030303157"
                        [index..index + 2],
                    16,
                )
                .unwrap()
            })
            .collect::<Vec<u8>>();
        let mut data = make_header(
            19,
            0x04,
            0,
            source_id,
            4294967295,
            0xe0,
            command_payload.len() as u8,
        );
        data.extend_from_slice(&command_payload);

        let payload = make_payload(source_id, data, true);
        let frame = parse_frame(&payload);
        let ret_frame = gp.process_command(&payload, frame, None).await;

        let command_frame = &ret_frame.payload["commandFrame"];
        assert_eq!(command_frame["deviceID"], json!(2));
        assert_eq!(command_frame["options"], json!(137));
        assert_eq!(command_frame["extendedOptions"], json!(243));
        assert_eq!(command_frame["keyMic"], json!(869628642));
        assert_eq!(command_frame["outgoingCounter"], json!(2323));
        assert_eq!(command_frame["applicationInfo"], json!(0));
        assert_eq!(command_frame["manufacturerID"], json!(0));
        assert_eq!(command_frame["modelID"], json!(0));
        assert_eq!(command_frame["numGpdCommands"], json!(0));
        assert_eq!(command_frame["gpdCommandIdList"], json!([]));
        assert_eq!(command_frame["numServerClusters"], json!(0));
        assert_eq!(command_frame["numClientClusters"], json!(0));
        assert_eq!(command_frame["gpdServerClusters"], json!([]));
        assert_eq!(command_frame["gpdClientClusters"], json!([]));
        assert_eq!(command_frame["genericSwitchConfig"], json!(0));
        assert_eq!(command_frame["currentContactStatus"], json!(0));

        let joined = joined.lock().unwrap().clone().expect("device joined");
        assert_eq!(joined.source_id, source_id);
        assert_eq!(joined.device_id, 2);
        assert_eq!(joined.network_address, (source_id & 0xffff) as u16);
        assert_eq!(
            joined.security_key,
            Some(vec![
                26, 219, 112, 168, 141, 113, 25, 110, 229, 12, 3, 88, 5, 55, 118, 125
            ])
        );

        let sent = adapter.sent_all.lock().unwrap();
        assert_eq!(sent.len(), 1);
        let pairing = &sent[0];
        assert_eq!(pairing.command.name, "pairing");
        assert_eq!(pairing.payload["options"], json!(58696));
        assert_eq!(pairing.payload["srcID"], json!(source_id));
        assert_eq!(pairing.payload["deviceID"], json!(2));
        assert_eq!(pairing.payload["frameCounter"], json!(2323));
        assert_eq!(pairing.payload["sinkGroupID"], json!(GP_GROUP_ID));
        // Node oracle for `encryptSecurityKey(sourceID, securityKey)`:
        // crypto.createCipheriv("aes-128-ccm", "ZigBeeAlliance09", nonce(13), {authTagLength: 16})
        assert_eq!(
            pairing.payload["gpdKey"],
            json!([
                0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
                0xce, 0xcf
            ])
        );
    }
}
