// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/znp.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Upstream 11.2.0 owns UNPI framing itself (the separate unpi Parser/Writer were
// removed) and receives raw chunks from `AdapterTransport`. Function names below
// intentionally mirror `znp.ts` so upstream diffs can be applied line by line.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::super::unpi::constants::{
    Subsystem, Type, DATA_START, MIN_MESSAGE_LENGTH, POSITION_DATA_LENGTH, SOF,
};
use super::super::unpi::frame::Frame;
use super::definition;
use super::zpi_object::{ParameterDef, ZpiObject};
use crate::adapter::transport::{AdapterTransport, ModemBits, TransportEvent};
use crate::utils::async_mutex::AsyncMutex;
use crate::utils::logger::logger;
use crate::utils::wait::wait;
use crate::utils::waitress::Waitress;

/// Upstream `timeouts`.
const SREQ_TIMEOUT: u64 = 6_000;
const RESET_TIMEOUT: u64 = 30_000;
const DEFAULT_TIMEOUT: u64 = 10_000;
const LONG_TIMEOUT: u64 = 40_000;

/// Upstream `NS`.
const NS: &str = "zh:zstack:znp";

/// Upstream `WaitressMatcher`.
#[derive(Debug, Clone)]
pub struct ZnpWaiterMatcher {
    pub r#type: Type,
    pub subsystem: Subsystem,
    pub command: String,
    pub target: Option<Value>,
    pub transid: Option<u8>,
    pub state: Option<u8>,
}

/// Upstream `Znp`.
pub struct Znp {
    /// Upstream `private readonly transport: AdapterTransport`.
    pub transport: AdapterTransport,
    /// Upstream `queue: AsyncMutex`.
    pub queue: Arc<AsyncMutex>,
    /// Upstream `waitress: Waitress<ZpiObject, WaitressMatcher>`.
    pub waitress: Arc<Waitress<ZpiObject, ZnpWaiterMatcher>>,
    /// Upstream `received` event payloads (drained by the adapter/controller).
    pub received: Arc<tokio::sync::Mutex<Vec<ZpiObject>>>,
    dispatcher: Option<tokio::task::JoinHandle<()>>,
}

impl Znp {
    /// Upstream `constructor(transport)` — znp.ts:48.
    pub fn new(transport: AdapterTransport) -> Self {
        let waitress = Arc::new(Waitress::new(
            Self::waitress_validator,
            |matcher: &ZnpWaiterMatcher, timeout: u64| {
                format!(
                    "{} - {} - {} after {}ms",
                    matcher.r#type.name(),
                    matcher.subsystem.name(),
                    matcher.command,
                    timeout
                )
            },
        ));

        Self {
            transport,
            queue: Arc::new(AsyncMutex::new()),
            waitress,
            received: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            dispatcher: None,
        }
    }

    /// Upstream `open()` — znp.ts:112.
    pub async fn open(&mut self) -> Result<(), String> {
        // Upstream always calls `transport.open(true)`. If a caller attached an
        // already opened port, keep it instead of tearing it down.
        if !self.transport.is_open() {
            self.transport.open(true).await?;
        }
        self.start_dispatcher()?;
        self.skip_bootloader().await;
        Ok(())
    }

    /// Upstream `onTransportData(chunk)` — znp.ts:58.
    async fn on_transport_data(
        input: &mut Vec<u8>,
        chunk: &[u8],
        waitress: &Waitress<ZpiObject, ZnpWaiterMatcher>,
        received: &tokio::sync::Mutex<Vec<ZpiObject>>,
    ) {
        input.extend_from_slice(chunk);

        if !input.is_empty() && input[0] != SOF {
            // Buffer doesn't start with SOF, skip till SOF.
            if let Some(start) = input.iter().position(|byte| *byte == SOF) {
                input.drain(..start);
            }
        }

        while input.len() >= MIN_MESSAGE_LENGTH && input[0] == SOF {
            let data_length = input[POSITION_DATA_LENGTH] as usize;
            let fcs_position = DATA_START + data_length;
            let frame_length = fcs_position + 1;

            if input.len() < frame_length {
                return;
            }

            Self::on_frame(
                &input[..frame_length],
                data_length as u8,
                fcs_position,
                waitress,
                received,
            )
            .await;

            input.drain(..frame_length);

            if !input.is_empty() && input[0] != SOF {
                // Buffer doesn't start with SOF, skip till SOF.
                if let Some(start) = input.iter().position(|byte| *byte == SOF) {
                    input.drain(..start);
                }
            }
        }
    }

    /// Upstream `onFrame(buffer, dataLength, fcsPosition)` — znp.ts:94.
    async fn on_frame(
        buffer: &[u8],
        data_length: u8,
        fcs_position: usize,
        waitress: &Waitress<ZpiObject, ZnpWaiterMatcher>,
        received: &tokio::sync::Mutex<Vec<ZpiObject>>,
    ) {
        let Some(frame) = Frame::from_buffer(data_length, fcs_position, buffer) else {
            logger().debug("--> error Invalid checksum", NS);
            return;
        };

        match catch_unwind(AssertUnwindSafe(|| ZpiObject::from_unpi_frame(&frame))) {
            Ok(object) => {
                waitress.resolve(&object).await;
                received.lock().await.push(object);
            }
            Err(_) => logger().error("Error while parsing to ZpiObject", NS),
        }
    }

    fn start_dispatcher(&mut self) -> Result<(), String> {
        let events = self
            .transport
            .take_events()
            .ok_or_else(|| "Transport did not provide an event stream".to_string())?;
        let waitress = self.waitress.clone();
        let received = self.received.clone();

        self.dispatcher = Some(tokio::spawn(async move {
            let mut input_buffer = Vec::new();
            let mut events = events;
            while let Some(event) = events.recv().await {
                match event {
                    TransportEvent::Data(chunk) => {
                        Self::on_transport_data(&mut input_buffer, &chunk, &waitress, &received)
                            .await;
                    }
                    TransportEvent::Close(error) => {
                        if let Some(error) = error {
                            logger().info(&format!("Port closed: {error}"), NS);
                        } else {
                            logger().info("Port closed", NS);
                        }
                        break;
                    }
                }
            }
        }));

        Ok(())
    }

    /// Upstream `skipBootloader()` — znp.ts:117.
    async fn skip_bootloader(&mut self) {
        let payload = json!({"capabilities": 1});
        if self
            .request(
                Subsystem::SYS,
                "ping",
                payload.clone(),
                None,
                Some(250),
                &[0],
            )
            .await
            .is_ok()
        {
            return;
        }

        // Skip bootloader on CC2530/CC2531: magic byte 0xef, then one second.
        if let Err(error) = self.transport.write(&[0xef]).await {
            logger().error(
                &format!("Writing CC2530/CC2531 skip bootloader payload failed: {error}"),
                NS,
            );
        }
        logger().info("Writing CC2530/CC2531 skip bootloader payload", NS);
        wait(1000).await;
        if self
            .request(Subsystem::SYS, "ping", payload, None, Some(250), &[0])
            .await
            .is_ok()
        {
            return;
        }

        // Skip bootloader on some CC2652 devices (e.g. zzh-p).
        logger().info("Skip bootloader for CC2652/CC1352", NS);
        if self.transport.is_serial() {
            let _ = self
                .transport
                .set(ModemBits {
                    dtr: Some(false),
                    rts: Some(false),
                })
                .await;
            wait(150).await;
            let _ = self
                .transport
                .set(ModemBits {
                    dtr: Some(false),
                    rts: Some(true),
                })
                .await;
            wait(150).await;
            let _ = self
                .transport
                .set(ModemBits {
                    dtr: Some(false),
                    rts: Some(false),
                })
                .await;
            wait(150).await;
        }
    }

    /// Upstream `close()` — znp.ts:144.
    pub async fn close(&mut self) -> Result<(), String> {
        logger().info("closing", NS);
        self.queue.clear();
        if let Some(dispatcher) = self.dispatcher.take() {
            dispatcher.abort();
        }
        self.transport.close().await
    }

    /// Upstream `requestWithReply(...)` — znp.ts:150.
    pub async fn request_with_reply(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: Value,
    ) -> Result<ZpiObject, String> {
        match self
            .request(subsystem, command, payload, None, None, &[0])
            .await?
        {
            Some(reply) => Ok(reply),
            None => Err(format!("Command {command} has no reply")),
        }
    }

    /// Upstream `request(...)` — znp.ts:165.
    pub async fn request(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: Value,
        waiter_id: Option<u64>,
        timeout: Option<u64>,
        expected_statuses: &[u8],
    ) -> Result<Option<ZpiObject>, String> {
        if !self.transport.is_open() {
            return Err("Cannot request when znp has not been initialized yet".to_string());
        }

        let object = Self::create_request(subsystem, command, payload)?;
        let queue = self.queue.clone();
        queue
            .run(|| async {
                self.request_locked(object, waiter_id, timeout, expected_statuses)
                    .await
            })
            .await
    }

    /// Upstream `ZpiObject.createRequest(...)` — zpiObject.ts:49.
    fn create_request(
        subsystem: Subsystem,
        command: &str,
        payload: Value,
    ) -> Result<ZpiObject, String> {
        Ok(ZpiObject::create_request(subsystem, command, payload))
    }

    async fn request_locked(
        &mut self,
        object: ZpiObject,
        waiter_id: Option<u64>,
        timeout: Option<u64>,
        expected_statuses: &[u8],
    ) -> Result<Option<ZpiObject>, String> {
        if object.r#type == Type::SREQ {
            let default_timeout = if object.command.name == "bdbStartCommissioning"
                || object.command.name == "startupFromApp"
            {
                LONG_TIMEOUT
            } else {
                SREQ_TIMEOUT
            };
            let (_, receiver) = self
                .waitress
                .wait_for(
                    ZnpWaiterMatcher {
                        r#type: Type::SRSP,
                        subsystem: object.subsystem,
                        command: object.command.name.clone(),
                        target: None,
                        transid: None,
                        state: None,
                    },
                    timeout.unwrap_or(default_timeout),
                )
                .await;

            self.transport
                .write(&object.to_unpi_frame().to_buffer())
                .await?;
            let result = receiver.await.map_err(|_| "Waiter dropped".to_string())??;

            if let Some(status) = result.payload.get("status").and_then(Value::as_u64) {
                if !expected_statuses.contains(&(status as u8)) {
                    if let Some(waiter_id) = waiter_id {
                        self.waitress.remove(waiter_id).await;
                    }
                    return Err(format!(
                        "--> '{}' failed with status '{}' (expected '{}')",
                        object.to_string(),
                        crate::adapter::z_stack::constants::utils::status_description(status as u8),
                        expected_statuses
                            .iter()
                            .map(|status| {
                                crate::adapter::z_stack::constants::utils::status_description(
                                    *status,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
                }
            }

            return Ok(Some(result));
        }

        if object.r#type == Type::AREQ && object.is_reset_command() {
            let (_, receiver) = self
                .waitress
                .wait_for(
                    ZnpWaiterMatcher {
                        r#type: Type::AREQ,
                        subsystem: Subsystem::SYS,
                        command: "resetInd".to_string(),
                        target: None,
                        transid: None,
                        state: None,
                    },
                    timeout.unwrap_or(RESET_TIMEOUT),
                )
                .await;

            self.queue.clear();
            self.transport
                .write(&object.to_unpi_frame().to_buffer())
                .await?;
            return Ok(Some(
                receiver.await.map_err(|_| "Waiter dropped".to_string())??,
            ));
        }

        if object.r#type == Type::AREQ {
            self.transport
                .write(&object.to_unpi_frame().to_buffer())
                .await?;
            return Ok(None);
        }

        Err(format!("Unknown type '{}'", object.r#type.name()))
    }

    /// Upstream `requestZdo(clusterId, payload, waiterID)` — znp.ts:219.
    pub async fn request_zdo(
        &mut self,
        cluster_id: u16,
        payload: Vec<u8>,
        waiter_id: Option<u64>,
    ) -> Result<(), String> {
        let queue = self.queue.clone();
        queue
            .run(|| async {
                let Some(definition) = definition::find_zdo_request(Subsystem::ZDO, cluster_id)
                else {
                    return Err(format!(
                        "Command for ZDO cluster ID '{cluster_id}' not supported."
                    ));
                };

                let (_, receiver) = self
                    .waitress
                    .wait_for(
                        ZnpWaiterMatcher {
                            r#type: Type::SRSP,
                            subsystem: Subsystem::ZDO,
                            command: definition.name.to_string(),
                            target: None,
                            transid: None,
                            state: None,
                        },
                        SREQ_TIMEOUT,
                    )
                    .await;

                let frame = Frame::new(
                    Type::SREQ,
                    Subsystem::ZDO,
                    definition.id,
                    payload.clone(),
                    0,
                    0,
                );
                self.transport.write(&frame.to_buffer()).await?;

                let result = receiver.await.map_err(|_| "Waiter dropped".to_string())??;

                if let Some(status) = result.payload.get("status").and_then(Value::as_u64) {
                    if status != 0 {
                        if let Some(waiter_id) = waiter_id {
                            self.waitress.remove(waiter_id).await;
                        }
                        return Err(format!(
                            "--> 'SREQ: ZDO - {} - {}' failed with status '{}'",
                            zdo_cluster_name(cluster_id),
                            payload
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>(),
                            crate::adapter::z_stack::constants::utils::status_description(
                                status as u8
                            ),
                        ));
                    }
                }

                Ok(())
            })
            .await
    }

    /// Upstream `waitFor(...)` — znp.ts:247.
    pub async fn wait_for(
        &self,
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        target: Option<Value>,
        transid: Option<u8>,
        state: Option<u8>,
        timeout: Option<u64>,
    ) -> (
        u64,
        tokio::sync::oneshot::Receiver<Result<ZpiObject, String>>,
    ) {
        self.waitress
            .wait_for(
                ZnpWaiterMatcher {
                    r#type,
                    subsystem,
                    command: command.to_string(),
                    target,
                    transid,
                    state,
                },
                timeout.unwrap_or(DEFAULT_TIMEOUT),
            )
            .await
    }

    /// Upstream `waitressValidator(zpiObject, matcher)` — znp.ts:259.
    fn waitress_validator(zpi_object: &ZpiObject, matcher: &ZnpWaiterMatcher) -> bool {
        if matcher.r#type != zpi_object.r#type
            || matcher.subsystem != zpi_object.subsystem
            || matcher.command != zpi_object.command.name
        {
            return false;
        }

        let target_matches = match &matcher.target {
            None => true,
            Some(Value::Number(target)) => {
                zpi_object.payload.get("srcaddr").and_then(Value::as_u64) == target.as_u64()
            }
            Some(Value::String(target)) => {
                zpi_object
                    .payload
                    .get("zdo")
                    .and_then(|zdo| zdo.get(1))
                    .and_then(|zdo| zdo.get("eui64"))
                    .and_then(Value::as_str)
                    == Some(target.as_str())
            }
            _ => false,
        };
        let transid_matches = matcher.transid.is_none_or(|transid| {
            zpi_object.payload.get("transid").and_then(Value::as_u64) == Some(transid as u64)
        });
        let state_matches = matcher.state.is_none_or(|state| {
            zpi_object.payload.get("state").and_then(Value::as_u64) == Some(state as u64)
        });

        target_matches && transid_matches && state_matches
    }

    /// Drain the objects emitted through the upstream `received` event.
    pub async fn drain_received(&self) -> Vec<ZpiObject> {
        std::mem::take(&mut *self.received.lock().await)
    }
}

fn zdo_cluster_name(cluster_id: u16) -> &'static str {
    use crate::zspec::zdo::definition::clusters::ClusterId;
    match cluster_id {
        ClusterId::NETWORK_ADDRESS_REQUEST => "NETWORK_ADDRESS_REQUEST",
        ClusterId::IEEE_ADDRESS_REQUEST => "IEEE_ADDRESS_REQUEST",
        ClusterId::NODE_DESCRIPTOR_REQUEST => "NODE_DESCRIPTOR_REQUEST",
        ClusterId::POWER_DESCRIPTOR_REQUEST => "POWER_DESCRIPTOR_REQUEST",
        ClusterId::SIMPLE_DESCRIPTOR_REQUEST => "SIMPLE_DESCRIPTOR_REQUEST",
        ClusterId::ACTIVE_ENDPOINTS_REQUEST => "ACTIVE_ENDPOINTS_REQUEST",
        ClusterId::MATCH_DESCRIPTORS_REQUEST => "MATCH_DESCRIPTORS_REQUEST",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attached_znp() -> (Znp, serial2_tokio::SerialPort) {
        let (reader, peer) = serial2_tokio::SerialPort::pair().expect("pair");
        let mut transport = AdapterTransport::new(
            crate::adapter::transport::TransportOptions::new("/dev/null"),
        );
        transport.attach_serial(reader).expect("attach");
        (Znp::new(transport), peer)
    }

    #[tokio::test]
    async fn parses_a_srsp_frame() {
        let (mut znp, mut peer) = attached_znp();
        znp.start_dispatcher().expect("dispatcher");
        let frame = Frame::new(
            Type::SRSP,
            Subsystem::SYS,
            0x02,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x01, 0x01, 0x01, 0x01],
            0,
            0,
        );
        tokio::io::AsyncWriteExt::write_all(&mut peer, &frame.to_buffer())
            .await
            .expect("write");

        let objects = loop {
            let objects = znp.drain_received().await;
            if !objects.is_empty() {
                break objects;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].command.name, "version");
        assert_eq!(objects[0].payload["product"], json!(2));
    }

    #[tokio::test]
    async fn rejects_a_frame_with_bad_fcs() {
        let (mut znp, mut peer) = attached_znp();
        znp.start_dispatcher().expect("dispatcher");
        let frame = Frame::new(Type::SRSP, Subsystem::SYS, 0x08, vec![0x00, 0x01], 0, 0);
        let mut buffer = frame.to_buffer();
        let fcs = buffer.len() - 1;
        buffer[fcs] ^= 0xff;
        tokio::io::AsyncWriteExt::write_all(&mut peer, &buffer)
            .await
            .expect("write");

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(znp.drain_received().await.is_empty());
    }

    #[tokio::test]
    async fn request_sreq_resolves_the_waiter() {
        let (mut znp, mut peer) = attached_znp();
        znp.start_dispatcher().expect("dispatcher");

        let responder = tokio::spawn(async move {
            let mut buffer = [0u8; 64];
            let read = tokio::io::AsyncReadExt::read(&mut peer, &mut buffer)
                .await
                .expect("read request");
            assert!(read > 0);
            let reply = Frame::new(
                Type::SRSP,
                Subsystem::SYS,
                0x08,
                vec![0x00, 0x02, 0x01, 0x02],
                0,
                0,
            );
            tokio::io::AsyncWriteExt::write_all(&mut peer, &reply.to_buffer())
                .await
                .expect("write reply");
        });

        let result = znp
            .request_with_reply(Subsystem::SYS, "osalNvRead", json!({"id": 1, "offset": 2}))
            .await
            .expect("reply");
        assert_eq!(result.command.name, "osalNvRead");
        assert_eq!(result.payload["status"], json!(0));
        responder.await.expect("responder");
    }

    #[tokio::test]
    async fn request_sreq_reports_a_failed_status() {
        let (mut znp, mut peer) = attached_znp();
        znp.start_dispatcher().expect("dispatcher");

        let responder = tokio::spawn(async move {
            let mut buffer = [0u8; 64];
            let _ = tokio::io::AsyncReadExt::read(&mut peer, &mut buffer)
                .await
                .expect("read request");
            let reply = Frame::new(
                Type::SRSP,
                Subsystem::SYS,
                0x08,
                vec![0x01, 0x02, 0x01, 0x02],
                0,
                0,
            );
            tokio::io::AsyncWriteExt::write_all(&mut peer, &reply.to_buffer())
                .await
                .expect("write reply");
        });

        let error = znp
            .request_with_reply(Subsystem::SYS, "osalNvRead", json!({"id": 1, "offset": 2}))
            .await
            .expect_err("failed status");
        assert!(error.contains("failed with status"));
        responder.await.expect("responder");
    }
}
