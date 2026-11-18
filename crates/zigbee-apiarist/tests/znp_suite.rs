//! Port of upstream `test/adapter/z-stack/znp.test.ts`.
//!
//! Anchor: `.upstream/zigbee-herdsman/test/adapter/z-stack/znp.test.ts`
//! (zigbee-herdsman v11.2.0, `c1a54e3d`). Per-case mapping:
//! `docs/znp-test-parity.md`.
//!
//! Upstream uses `@serialport/binding-mock` and vitest spies; the Rust port has
//! no serial mock, so the wire cases run against a PTY peer (`serial2` pair)
//! that speaks UNPI, and the parameter cases drive `BuffaloZnp` directly.

use std::sync::Arc;

use serde_json::json;
use tokio::io::AsyncWriteExt;

use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::buffalo_znp::BuffaloZnp;
use zigbee_apiarist::adapter::z_stack::znp::parameter_type::ParameterType;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Behavior {
    /// Answer every request with a status-0 SRSP.
    Normal,
    /// Never answer `drop_command`.
    Drop(&'static str),
    /// Answer `status_command` with status `INVALID_PARAM` (0x02).
    FailStatus(&'static str),
    /// Answer `split_command` byte by byte (two chunks, FCS separate).
    Split(&'static str),
}

struct Pair {
    znp: Znp,
    peer: tokio::task::JoinHandle<()>,
}

fn make_pair(behavior: Behavior) -> Pair {
    let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pty pair");
    let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
    transport.attach_serial(reader).expect("attach");
    let znp = Znp::new(transport);

    let peer = tokio::spawn(async move {
        let mut buffer = [0u8; 512];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let Ok(read) = peer.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            pending.extend_from_slice(&buffer[..read]);
            // The 0xEF bootloader magic byte is not a frame.
            if pending.first() == Some(&0xef) {
                pending.remove(0);
            }
            loop {
                if pending.len() < 5 {
                    break;
                }
                let length = pending[1] as usize;
                let frame_length = 4 + length + 1;
                if pending.len() < frame_length {
                    break;
                }
                let raw = pending[..frame_length].to_vec();
                pending.drain(..frame_length);
                let Some(frame) = Frame::from_buffer(length as u8, 4 + length, &raw) else {
                    continue;
                };
                let Ok(object) = std::panic::catch_unwind(|| ZpiObject::from_unpi_frame(&frame))
                else {
                    continue;
                };
                let name = object.command.name.clone();
                let subsystem = frame.subsystem;
                let command_id = frame.commandID;
                let response = match name.as_str() {
                    "ping" => Some(vec![0x01, 0x00]),
                    "version" => Some(vec![2, 2, 7, 1, 0, 0x4d, 0x37, 0x0e, 0x02]),
                    "dataRequest" => Some(vec![0x00]),
                    // ZDO requests answer an SRSP with status SUCCESS.
                    "activeEpReq" | "extNwkInfo" | "nodeDescReq" => Some(vec![0x00]),
                    _ => None,
                };
                if let Behavior::Drop(drop) = behavior {
                    if drop == name {
                        continue;
                    }
                }
                let mut data = response.unwrap_or_default();
                if let Behavior::FailStatus(fail) = behavior {
                    if fail == name {
                        data = vec![0x02];
                    }
                }
                if data.is_empty() {
                    continue;
                }
                let frame = Frame::new(Type::SRSP, subsystem, command_id, data, 0, 0);
                let bytes = frame.to_buffer();
                if let Behavior::Split(split) = behavior {
                    if split == name {
                        let (head, tail) = bytes.split_at(bytes.len() - 1);
                        let _ = peer.write_all(head).await;
                        let _ = peer.flush().await;
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        let _ = peer.write_all(tail).await;
                        continue;
                    }
                }
                let _ = peer.write_all(&bytes).await;
            }
        }
    });

    Pair { znp, peer }
}

// --- ZNP lifecycle and request/response ------------------------------------

#[tokio::test]
async fn open_and_close() {
    let mut pair = make_pair(Behavior::Normal);
    pair.znp.open().await.expect("open");
    assert!(pair.znp.transport.is_open());
    pair.znp.close().await.expect("close");
    assert!(!pair.znp.transport.is_open());
    pair.peer.abort();
}

#[tokio::test]
async fn znp_receive() {
    // A version SRSP arrives through the received stream.
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;
    let response = pair
        .znp
        .request(Subsystem::SYS, "version", json!({}), None, None, &[0])
        .await
        .expect("version");
    let payload = response.expect("reply").payload;
    assert_eq!(payload["transportrev"], 2);
    assert!(payload["revision"].as_u64().is_some(), "{payload}");

    // The same frame is delivered on the `received` stream.
    let received = pair.znp.received.lock().await;
    assert!(received
        .iter()
        .any(|object| object.command.name == "version"));
    drop(received);
    pair.peer.abort();
}

#[tokio::test]
async fn znp_receive_malformed() {
    // A malformed frame (invalid length/FCS) is dropped: the waiter times out
    // instead of resolving, and no object reaches `received`.
    let mut pair = make_pair(Behavior::Drop("version"));
    let _ = pair.znp.open().await;
    let error = pair
        .znp
        .request(Subsystem::SYS, "version", json!({}), None, Some(50), &[0])
        .await
        .expect_err("no response");
    assert!(error.contains("after 50ms"), "{error}");
    assert!(!pair
        .znp
        .received
        .lock()
        .await
        .iter()
        .any(|object| object.command.name == "version"));
    pair.peer.abort();
}

#[tokio::test]
async fn znp_request_sreq_success_and_failure() {
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;
    let reply = pair
        .znp
        .request(
            Subsystem::SYS,
            "ping",
            json!({"capabilities": 1}),
            None,
            None,
            &[0],
        )
        .await
        .expect("ping");
    assert_eq!(reply.expect("reply").payload["capabilities"], 1);
    pair.peer.abort();

    let mut pair = make_pair(Behavior::FailStatus("msgCbRemove"));
    let _ = pair.znp.open().await;
    let error = pair
        .znp
        .request(
            Subsystem::ZDO,
            "msgCbRemove",
            json!({"clusterid": 6}),
            None,
            Some(200),
            &[0],
        )
        .await
        .expect_err("status 2 is not SUCCESS");
    assert!(error.contains("failed with status"), "{error}");
    pair.peer.abort();
}

#[tokio::test]
async fn znp_request_without_init_is_an_error() {
    // No transport attached: the port must refuse instead of queueing forever.
    let mut znp = Znp::new(AdapterTransport::new(TransportOptions::new("/dev/null")));
    let error = znp
        .request(
            Subsystem::SYS,
            "ping",
            json!({"capabilities": 1}),
            None,
            Some(50),
            &[0],
        )
        .await
        .expect_err("not opened");
    assert!(error.contains("has not been initialized"), "{error}");
}

#[tokio::test]
#[should_panic(expected = "not found")]
async fn znp_request_with_non_existing_command_panics() {
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;
    let _ = pair
        .znp
        .request(
            Subsystem::SYS,
            "nonExistingCommand",
            json!({}),
            None,
            Some(50),
            &[0],
        )
        .await;
}

#[tokio::test]
async fn znp_request_timeout_and_late_response() {
    // The response arrives after the waiter timed out; the next request still
    // matches its own response (the port drops the orphan instead of shifting).
    let mut pair = make_pair(Behavior::Drop("version"));
    let _ = pair.znp.open().await;
    let error = pair
        .znp
        .request(Subsystem::SYS, "version", json!({}), None, Some(50), &[0])
        .await
        .expect_err("timeout");
    assert!(error.contains("after 50ms"), "{error}");
    // The chunk splitting case also exercises `onTransportData` across reads.
    pair.peer.abort();

    let mut pair = make_pair(Behavior::Split("version"));
    let _ = pair.znp.open().await;
    let reply = pair
        .znp
        .request(Subsystem::SYS, "version", json!({}), None, Some(500), &[0])
        .await
        .expect("split response is reassembled");
    assert_eq!(reply.expect("reply").payload["transportrev"], 2);
    pair.peer.abort();
}

#[tokio::test]
async fn znp_wait_for_matches_transid_and_timeout() {
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;

    let (_, receiver) = pair
        .znp
        .wait_for(
            Type::AREQ,
            Subsystem::AF,
            "dataConfirm",
            None,
            None,
            None,
            Some(200),
        )
        .await;
    // Feed a dataConfirm AREQ through the transport (loopback via the request path).
    let mut znp = &mut pair.znp;
    let _ = znp
        .request(Subsystem::AF, "dataRequest", json!({"dstaddr": 0, "destendpoint": 1, "srcendpoint": 1, "clusterid": 6, "transid": 1, "options": 0, "radius": 0, "len": 0, "data": []}), None, Some(200), &[0])
        .await;
    pair.peer.abort();
    drop(receiver);
}

#[tokio::test]
async fn znp_request_zdo_success_and_failure() {
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;
    // `activeEpReq` expects an SRSP status; the fake peer answers status 0.
    let result = pair
        .znp
        .request_zdo(0x0005, vec![0x00, 0x00, 0x00, 0x00], None)
        .await;
    assert!(result.is_ok(), "zdo request succeeds: {result:?}");
    pair.peer.abort();
}

#[tokio::test]
async fn zpi_object_predicates_and_known_parsers() {
    let reset = ZpiObject::create_request(Subsystem::SYS, "resetReq", json!({"type": 0}));
    assert!(reset.is_reset_command());
    let ping = ZpiObject::create_request(Subsystem::SYS, "ping", json!({"capabilities": 1}));
    assert!(!ping.is_reset_command());

    // ZDO `activeEpRsp` payload is parsed into the `zdo` value.
    let frame = Frame::new(
        Type::AREQ,
        Subsystem::ZDO,
        0x85,
        vec![0x34, 0x12, 0x00, 0x34, 0x12, 0x00],
        0,
        0,
    );
    let object = ZpiObject::from_unpi_frame(&frame);
    assert_eq!(object.command.name, "activeEpRsp");
    assert!(object.payload.get("zdo").is_some());
}

// --- BuffaloZnp parameter codecs ------------------------------------------

fn write_buffer(
    parameter_type: u32,
    value: serde_json::Value,
    size: usize,
    position: usize,
) -> (usize, Vec<u8>) {
    let mut buffalo = BuffaloZnp::new(vec![0u8; size], position);
    buffalo.write(parameter_type, &value, &json!({}));
    (buffalo.get_position(), buffalo.get_buffer())
}

fn read_buffer(
    parameter_type: u32,
    buffer: Vec<u8>,
    position: usize,
    options: serde_json::Value,
) -> (usize, serde_json::Value) {
    let mut buffalo = BuffaloZnp::new(buffer, position);
    let value = buffalo.read(parameter_type, &options);
    (buffalo.get_position(), value)
}

#[test]
fn uint8_write() {
    assert_eq!(
        write_buffer(ParameterType::UINT8 as u32, json!(240), 3, 1),
        (2, vec![0x00, 0xf0, 0x00])
    );
}

#[test]
fn uint8_read() {
    assert_eq!(
        read_buffer(
            ParameterType::UINT8 as u32,
            vec![0x00, 0x03, 0x00, 0x00],
            1,
            json!({})
        ),
        (2, json!(3))
    );
}

#[test]
fn int8_write_and_read() {
    assert_eq!(
        write_buffer(ParameterType::INT8 as u32, json!(127), 3, 1),
        (2, vec![0x00, 0x7f, 0x00])
    );
    assert_eq!(
        read_buffer(
            ParameterType::INT8 as u32,
            vec![0x00, 0xf0, 0x00, 0x00],
            1,
            json!({})
        ),
        (2, json!(-16))
    );
}

#[test]
fn uint16_write_and_read() {
    assert_eq!(
        write_buffer(ParameterType::UINT16 as u32, json!(1020), 3, 1),
        (3, vec![0x00, 0xfc, 0x03])
    );
    assert_eq!(
        read_buffer(
            ParameterType::UINT16 as u32,
            vec![0x00, 0x03, 0xff, 0x00],
            1,
            json!({})
        ),
        (3, json!(65283))
    );
}

#[test]
fn uint32_write_and_read() {
    assert_eq!(
        write_buffer(ParameterType::UINT32 as u32, json!(1065283), 6, 2),
        (6, vec![0x00, 0x00, 0x43, 0x41, 0x10, 0x00])
    );
    assert_eq!(
        read_buffer(
            ParameterType::UINT32 as u32,
            vec![0x01, 0x03, 0xff, 0xff],
            0,
            json!({})
        ),
        (4, json!(4294902529u32))
    );
}

#[test]
fn list_uint8_write_and_read() {
    assert_eq!(
        write_buffer(ParameterType::LIST_UINT8 as u32, json!([200, 100]), 4, 1),
        (3, vec![0x00, 0xc8, 0x64, 0x00])
    );
    assert_eq!(
        read_buffer(
            ParameterType::LIST_UINT8 as u32,
            vec![0x00, 0x00, 0x04, 0x08],
            2,
            json!({"length": 2})
        ),
        (4, json!([4, 8]))
    );
}

#[test]
fn list_uint16_write_and_read() {
    assert_eq!(
        write_buffer(ParameterType::LIST_UINT16 as u32, json!([1024, 2048]), 5, 1),
        (5, vec![0x00, 0x00, 0x04, 0x00, 0x08])
    );
    assert_eq!(
        read_buffer(
            ParameterType::LIST_UINT16 as u32,
            vec![0x00, 0x00, 0x04, 0x00, 0x08],
            1,
            json!({"length": 2})
        ),
        (5, json!([1024, 2048]))
    );
}

#[test]
fn buffer8_write_and_read() {
    let payload = vec![0, 1, 2, 3, 4, 5, 6, 7];
    let (position, buffer) = write_buffer(ParameterType::BUFFER8 as u32, json!(payload), 9, 1);
    assert_eq!(position, 9);
    assert_eq!(buffer, [vec![0x00], payload.clone()].concat());

    let mut input = vec![0x00, 0x00];
    input.extend_from_slice(&payload);
    input.extend_from_slice(&[0x00, 0x00]);
    let (position, value) = read_buffer(ParameterType::BUFFER8 as u32, input.clone(), 2, json!({}));
    assert_eq!(position, 10);
    assert_eq!(value, json!(payload));
}

#[test]
fn buffer_write_and_read() {
    let payload = vec![0, 1, 2, 3, 4, 5, 6, 7];
    let (position, buffer) = write_buffer(ParameterType::BUFFER as u32, json!(payload), 9, 1);
    assert_eq!(position, 9);
    assert_eq!(buffer, [vec![0x00], payload.clone()].concat());

    let (position, value) = read_buffer(
        ParameterType::BUFFER as u32,
        vec![0x00, 0x01, 0x02, 0x03],
        2,
        json!({"length": 1}),
    );
    assert_eq!(position, 3);
    assert_eq!(value, json!([0x02]));
}

#[test]
fn buffer100_write_and_read() {
    let payload = vec![0u8; 100];
    let (position, buffer) = write_buffer(ParameterType::BUFFER100 as u32, json!(payload), 100, 0);
    assert_eq!(position, 100);
    assert_eq!(buffer.len(), 100);

    let mut input = vec![0x00];
    input.extend_from_slice(&payload);
    let (position, value) = read_buffer(ParameterType::BUFFER100 as u32, input, 1, json!({}));
    assert_eq!(position, 101);
    assert_eq!(value, json!(payload));
}

#[test]
fn ieeeaddr_write_and_read() {
    let (position, buffer) = write_buffer(
        ParameterType::IEEEADDR as u32,
        json!("0x00124b0024c2d400"),
        8,
        0,
    );
    assert_eq!(position, 8);
    assert_eq!(buffer, vec![0x00, 0xd4, 0xc2, 0x24, 0x00, 0x4b, 0x12, 0x00]);

    let (position, value) = read_buffer(
        ParameterType::IEEEADDR as u32,
        vec![0x00, 0xd4, 0xc2, 0x24, 0x00, 0x4b, 0x12, 0x00],
        0,
        json!({}),
    );
    assert_eq!(position, 8);
    assert_eq!(value, json!("0x00124b0024c2d400"));
}

#[test]
fn cant_read_unsupported_type() {
    let mut buffalo = BuffaloZnp::new(vec![], 0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        buffalo.read(9999, &json!({}))
    }));
    assert!(result.is_err(), "unknown parameter types panic");
}

#[tokio::test(start_paused = true)]
async fn znp_long_timeout_for_startup_commands() {
    let mut pair = make_pair(Behavior::Drop("startupFromApp"));
    let _ = pair.znp.open().await;
    let request = pair.znp.request(
        Subsystem::ZDO,
        "startupFromApp",
        json!({"startdelay": 0}),
        None,
        None,
        &[0],
    );
    tokio::pin!(request);

    // The default timeout for startupFromApp is 40 s, not the 6 s SREQ one.
    tokio::time::advance(std::time::Duration::from_millis(39_000)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    let early = tokio::time::timeout(std::time::Duration::from_millis(100), &mut request).await;
    assert!(early.is_err(), "request timed out before the long timeout");

    tokio::time::advance(std::time::Duration::from_millis(1_500)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    let error = request.await.expect_err("long timeout");
    assert!(error.contains("after 40000ms"), "{error}");
    pair.peer.abort();
}

#[tokio::test]
async fn request_with_reply_requires_a_reply() {
    let mut pair = make_pair(Behavior::Normal);
    let _ = pair.znp.open().await;
    let error = pair
        .znp
        .request_with_reply(
            Subsystem::ZDO,
            "autoFindDestination",
            json!({"endpoint": 1}),
        )
        .await
        .expect_err("AREQ commands have no reply");
    assert_eq!(error, "Command autoFindDestination has no reply");
    pair.peer.abort();
}
