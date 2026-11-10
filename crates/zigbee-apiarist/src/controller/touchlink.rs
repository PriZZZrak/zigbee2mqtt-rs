// Original: .upstream/zigbee-herdsman/src/controller/touchlink.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};

use crate::adapter::events::ZclPayload;
use crate::utils::logger::logger;
use crate::utils::wait::wait;
use crate::zspec::zcl::zcl_frame::Frame;

const NS: &str = "zh:controller:touchlink";
const SCAN_CHANNELS: [u8; 16] = [
    11, 15, 20, 25, 12, 13, 14, 16, 17, 18, 19, 21, 22, 23, 24, 26,
];

/// Upstream `createScanRequestFrame(transaction)`.
fn create_scan_request_frame(transaction: u32) -> Frame {
    Frame::create(
        1,
        0,
        true,
        None,
        0,
        "scanRequest",
        "touchlink",
        json!({
            "transactionID": transaction,
            "zigbeeInformation": 4,
            "touchlinkInformation": 18,
        }),
        json!({}),
        0,
    )
}

/// Upstream `createIdentifyRequestFrame(transaction)`.
fn create_identify_request_frame(transaction: u32) -> Frame {
    Frame::create(
        1,
        0,
        true,
        None,
        0,
        "identifyRequest",
        "touchlink",
        json!({"transactionID": transaction, "duration": 65535}),
        json!({}),
        0,
    )
}

/// Upstream `createResetFactoryNewRequestFrame(transaction)`.
fn create_reset_factory_new_request_frame(transaction: u32) -> Frame {
    Frame::create(
        1,
        0,
        true,
        None,
        0,
        "resetToFactoryNew",
        "touchlink",
        json!({"transactionID": transaction}),
        json!({}),
        0,
    )
}

pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Controller-facing subset of the upstream `Adapter` used by Touchlink.
/// Implemented by the concrete adapter wrapper.
pub trait TouchlinkAdapter: Send + Sync {
    fn set_channel_inter_pan(&self, channel: u8) -> AdapterFuture<'_, ()>;
    fn restore_channel_inter_pan(&self) -> AdapterFuture<'_, ()>;
    fn send_zcl_frame_inter_pan_broadcast(
        &self,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
    ) -> AdapterFuture<'_, Option<ZclPayload>>;
    fn send_zcl_frame_inter_pan_to_ieee_addr(
        &self,
        frame: &Frame,
        ieee_address: &str,
    ) -> AdapterFuture<'_, ()>;
}

impl<F: TouchlinkAdapter + ?Sized> TouchlinkAdapter for std::sync::Arc<F> {
    fn set_channel_inter_pan(&self, channel: u8) -> AdapterFuture<'_, ()> {
        (**self).set_channel_inter_pan(channel)
    }

    fn restore_channel_inter_pan(&self) -> AdapterFuture<'_, ()> {
        (**self).restore_channel_inter_pan()
    }

    fn send_zcl_frame_inter_pan_broadcast(
        &self,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
    ) -> AdapterFuture<'_, Option<ZclPayload>> {
        (**self).send_zcl_frame_inter_pan_broadcast(frame, timeout, disable_response)
    }

    fn send_zcl_frame_inter_pan_to_ieee_addr(
        &self,
        frame: &Frame,
        ieee_address: &str,
    ) -> AdapterFuture<'_, ()> {
        (**self).send_zcl_frame_inter_pan_to_ieee_addr(frame, ieee_address)
    }
}

/// Upstream `scan()` result element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    /// Upstream camelCase: `ieeeAddr`.
    pub ieee_addr: String,
    pub channel: u8,
}

/// Upstream `Touchlink` — touchlink.ts:45.
pub struct Touchlink<A: TouchlinkAdapter> {
    adapter: A,
    locked: bool,
}

impl<A: TouchlinkAdapter> Touchlink<A> {
    /// Upstream `constructor(adapter)`.
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            locked: false,
        }
    }

    /// Upstream `stop()` — touchlink.ts:52.
    pub async fn stop(&mut self) -> Result<(), String> {
        if self.locked {
            self.restore_channel_inter_pan().await?;
            self.locked = false;
        }
        Ok(())
    }

    /// Upstream `lock(lock)` — touchlink.ts:60.
    pub fn lock(&mut self, lock: bool) -> Result<(), String> {
        if lock && self.locked {
            return Err("Touchlink operation already in progress".to_string());
        }
        self.locked = lock;
        Ok(())
    }

    /// Upstream `transactionNumber()` — `Math.floor(Math.random() * 0xffffffff)`.
    fn transaction_number() -> u32 {
        let mut bytes = [0u8; 4];
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            if file.read_exact(&mut bytes).is_ok() {
                return u32::from_le_bytes(bytes);
            }
        }
        (crate::utils::datetime::now_ms() & 0xffff_ffff) as u32
    }

    /// Upstream `setChannelInterPAN(channel)` — touchlink.ts:69.
    pub async fn set_channel_inter_pan(&self, channel: u8) -> Result<(), String> {
        logger().info(&format!("Set InterPAN channel to '{channel}'"), NS);
        self.adapter.set_channel_inter_pan(channel).await
    }

    /// Upstream `restoreChannelInterPAN()` — touchlink.ts:74.
    pub async fn restore_channel_inter_pan(&self) -> Result<(), String> {
        logger().info("Restore InterPAN channel", NS);
        self.adapter.restore_channel_inter_pan().await
    }

    /// Upstream `scan()` — touchlink.ts:79.
    pub async fn scan(&mut self) -> Result<Vec<ScanResult>, String> {
        self.lock(true)?;
        let result = self.scan_inner().await;
        // Upstream `finally`: restore first, unlock only if restore succeeded.
        self.restore_channel_inter_pan().await?;
        self.lock(false)?;
        result
    }

    async fn scan_inner(&self) -> Result<Vec<ScanResult>, String> {
        let mut result = Vec::new();
        for channel in SCAN_CHANNELS {
            self.set_channel_inter_pan(channel).await?;
            match self
                .adapter
                .send_zcl_frame_inter_pan_broadcast(
                    &create_scan_request_frame(Self::transaction_number()),
                    500,
                    false,
                )
                .await
            {
                Ok(Some(response)) => match response.address.as_str() {
                    Some(address) => {
                        logger().debug(
                            &format!("Got scan response on channel '{channel}' of '{address}'"),
                            NS,
                        );
                        result.push(ScanResult {
                            ieee_addr: address.to_string(),
                            channel,
                        });
                    }
                    None => logger().warning(
                        &format!(
                            "Scan request failed or was not answered: 'address is not a string'"
                        ),
                        NS,
                    ),
                },
                Ok(None) => {
                    logger().warning("Scan request failed or was not answered: 'no response'", NS)
                }
                Err(error) => logger().warning(
                    &format!("Scan request failed or was not answered: '{error}'"),
                    NS,
                ),
            }
        }
        Ok(result)
    }

    /// Upstream `identify(ieeeAddr, channel)` — touchlink.ts:97.
    pub async fn identify(&mut self, ieee_addr: &str, channel: u8) -> Result<(), String> {
        self.lock(true)?;
        let result = self.identify_inner(ieee_addr, channel).await;
        self.restore_channel_inter_pan().await?;
        self.lock(false)?;
        result
    }

    async fn identify_inner(&self, ieee_addr: &str, channel: u8) -> Result<(), String> {
        let transaction = Self::transaction_number();
        self.set_channel_inter_pan(channel).await?;
        self.adapter
            .send_zcl_frame_inter_pan_broadcast(&create_scan_request_frame(transaction), 500, false)
            .await?;
        logger().debug(&format!("Got scan response on channel '{channel}'"), NS);
        logger().debug(&format!("Identifying '{ieee_addr}'"), NS);
        self.adapter
            .send_zcl_frame_inter_pan_to_ieee_addr(
                &create_identify_request_frame(transaction),
                ieee_addr,
            )
            .await
    }

    /// Upstream `factoryReset(ieeeAddr, channel)` — touchlink.ts:113.
    pub async fn factory_reset(&mut self, ieee_addr: &str, channel: u8) -> Result<(), String> {
        self.lock(true)?;
        let result = self.factory_reset_inner(ieee_addr, channel).await;
        self.restore_channel_inter_pan().await?;
        self.lock(false)?;
        result
    }

    async fn factory_reset_inner(&self, ieee_addr: &str, channel: u8) -> Result<(), String> {
        let transaction = Self::transaction_number();
        self.set_channel_inter_pan(channel).await?;
        self.adapter
            .send_zcl_frame_inter_pan_broadcast(&create_scan_request_frame(transaction), 500, false)
            .await?;
        logger().debug(&format!("Got scan response on channel '{channel}'"), NS);
        logger().debug(&format!("Identifying '{ieee_addr}'"), NS);
        self.adapter
            .send_zcl_frame_inter_pan_to_ieee_addr(
                &create_identify_request_frame(transaction),
                ieee_addr,
            )
            .await?;
        wait(2000).await;
        logger().debug(&format!("Reset to factory new '{ieee_addr}'"), NS);
        self.adapter
            .send_zcl_frame_inter_pan_to_ieee_addr(
                &create_reset_factory_new_request_frame(transaction),
                ieee_addr,
            )
            .await
    }

    /// Upstream `factoryResetFirst()` — touchlink.ts:133.
    pub async fn factory_reset_first(&mut self) -> Result<bool, String> {
        self.lock(true)?;
        let result = self.factory_reset_first_inner().await;
        self.restore_channel_inter_pan().await?;
        self.lock(false)?;
        result
    }

    async fn factory_reset_first_inner(&self) -> Result<bool, String> {
        let mut done = false;
        for channel in SCAN_CHANNELS {
            self.set_channel_inter_pan(channel).await?;
            let transaction = Self::transaction_number();
            match self
                .adapter
                .send_zcl_frame_inter_pan_broadcast(
                    &create_scan_request_frame(transaction),
                    500,
                    false,
                )
                .await
            {
                Ok(Some(response)) => {
                    logger().debug(&format!("Got scan response on channel '{channel}'"), NS);
                    let Some(address) = response.address.as_str() else {
                        logger().warning(
                            "Scan request failed or was not answered: 'address is not a string'",
                            NS,
                        );
                        continue;
                    };
                    logger().debug("Identifying", NS);
                    self.adapter
                        .send_zcl_frame_inter_pan_to_ieee_addr(
                            &create_identify_request_frame(transaction),
                            address,
                        )
                        .await?;
                    wait(2000).await;
                    logger().debug("Reset to factory new", NS);
                    self.adapter
                        .send_zcl_frame_inter_pan_to_ieee_addr(
                            &create_reset_factory_new_request_frame(transaction),
                            address,
                        )
                        .await?;
                    done = true;
                }
                Ok(None) => {
                    logger().warning("Scan request failed or was not answered: 'no response'", NS)
                }
                Err(error) => logger().warning(
                    &format!("Scan request failed or was not answered: '{error}'"),
                    NS,
                ),
            }
            if done {
                break;
            }
        }
        Ok(done)
    }

    /// Test helper mirroring the private `locked` field (upstream has no getter).
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

/// Upstream `assertString(value)` equivalent used by touchlink.
pub fn assert_string(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "Input must be a string!".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeState {
        channels: Vec<u8>,
        restored: usize,
        broadcasts: Vec<Frame>,
        to_ieee: Vec<(String, Frame)>,
        respond_channel: Option<u8>,
        address: String,
    }

    struct Fake {
        state: Mutex<FakeState>,
    }

    impl Fake {
        fn new(respond_channel: Option<u8>) -> Self {
            Self {
                state: Mutex::new(FakeState {
                    respond_channel,
                    address: "0x00124b0024c2d400".to_string(),
                    ..FakeState::default()
                }),
            }
        }
    }

    fn payload(address: Value) -> ZclPayload {
        ZclPayload {
            cluster_id: 0x1000,
            data: Vec::new(),
            header: Some(Header::new(
                FrameControl {
                    reserved_bits: 0,
                    frame_type: FrameType::SPECIFIC,
                    direction: Direction::SERVER_TO_CLIENT,
                    disable_default_response: true,
                    manufacturer_specific: false,
                },
                None,
                0,
                1,
            )),
            address,
            endpoint: 0,
            linkquality: 0,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 0,
        }
    }

    impl TouchlinkAdapter for Fake {
        fn set_channel_inter_pan(&self, channel: u8) -> AdapterFuture<'_, ()> {
            Box::pin(async move {
                self.state.lock().unwrap().channels.push(channel);
                Ok(())
            })
        }

        fn restore_channel_inter_pan(&self) -> AdapterFuture<'_, ()> {
            Box::pin(async move {
                self.state.lock().unwrap().restored += 1;
                Ok(())
            })
        }

        fn send_zcl_frame_inter_pan_broadcast(
            &self,
            frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
        ) -> AdapterFuture<'_, Option<ZclPayload>> {
            let frame = frame.clone();
            Box::pin(async move {
                let mut state = self.state.lock().unwrap();
                state.broadcasts.push(frame);
                let address = state.address.clone();
                if state.respond_channel == state.channels.last().copied() {
                    Ok(Some(payload(Value::String(address))))
                } else {
                    Err("timeout".to_string())
                }
            })
        }

        fn send_zcl_frame_inter_pan_to_ieee_addr(
            &self,
            frame: &Frame,
            ieee_address: &str,
        ) -> AdapterFuture<'_, ()> {
            let ieee_address = ieee_address.to_string();
            let frame = frame.clone();
            Box::pin(async move {
                self.state
                    .lock()
                    .unwrap()
                    .to_ieee
                    .push((ieee_address, frame));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn scans_all_channels_and_restores() {
        let fake = std::sync::Arc::new(Fake::new(Some(15)));
        let mut touchlink = Touchlink::new(fake.clone());
        let result = touchlink.scan().await.expect("scan");
        assert_eq!(
            result,
            vec![ScanResult {
                ieee_addr: "0x00124b0024c2d400".to_string(),
                channel: 15
            }]
        );

        let state = fake.state.lock().unwrap();
        assert_eq!(state.channels, SCAN_CHANNELS.to_vec());
        assert_eq!(state.restored, 1);
        assert_eq!(state.broadcasts.len(), SCAN_CHANNELS.len());
        for (index, frame) in state.broadcasts.iter().enumerate() {
            assert_eq!(frame.command.name, "scanRequest");
            assert_eq!(frame.payload["zigbeeInformation"], serde_json::json!(4));
            assert_eq!(frame.payload["touchlinkInformation"], serde_json::json!(18));
            assert_eq!(state.channels[index], SCAN_CHANNELS[index]);
        }
        assert!(!touchlink.is_locked());
    }

    #[tokio::test]
    async fn identify_uses_scan_transaction() {
        let fake = std::sync::Arc::new(Fake::new(Some(20)));
        let mut touchlink = Touchlink::new(fake.clone());
        touchlink
            .identify("0x00124b0024c2d400", 20)
            .await
            .expect("identify");

        let state = fake.state.lock().unwrap();
        assert_eq!(state.channels, vec![20]);
        assert_eq!(state.restored, 1);
        assert_eq!(state.broadcasts.len(), 1);
        assert_eq!(state.to_ieee.len(), 1);
        let (address, frame) = &state.to_ieee[0];
        assert_eq!(address, "0x00124b0024c2d400");
        assert_eq!(frame.command.name, "identifyRequest");
        assert_eq!(frame.payload["duration"], serde_json::json!(65535));
        assert_eq!(
            frame.payload["transactionID"],
            state.broadcasts[0].payload["transactionID"]
        );
        assert!(!touchlink.is_locked());
    }

    #[tokio::test]
    async fn factory_reset_first_stops_at_first_response() {
        let fake = std::sync::Arc::new(Fake::new(Some(15)));
        let mut touchlink = Touchlink::new(fake.clone());
        assert!(touchlink.factory_reset_first().await.expect("reset"));

        let state = fake.state.lock().unwrap();
        assert_eq!(state.channels, vec![11, 15]);
        assert_eq!(state.restored, 1);
        assert_eq!(state.to_ieee.len(), 2);
        assert_eq!(state.to_ieee[0].1.command.name, "identifyRequest");
        assert_eq!(state.to_ieee[1].1.command.name, "resetToFactoryNew");
        assert_eq!(
            state.to_ieee[0].1.payload["transactionID"],
            state.to_ieee[1].1.payload["transactionID"]
        );
    }

    #[tokio::test]
    async fn lock_rejects_concurrent_operation() {
        let fake = std::sync::Arc::new(Fake::new(Some(11)));
        let mut touchlink = Touchlink::new(fake.clone());
        touchlink.lock(true).expect("lock");
        assert_eq!(
            touchlink.lock(true),
            Err("Touchlink operation already in progress".to_string())
        );
        touchlink.stop().await.expect("stop");
        assert_eq!(fake.state.lock().unwrap().restored, 1);
        assert!(!touchlink.is_locked());
    }
}
