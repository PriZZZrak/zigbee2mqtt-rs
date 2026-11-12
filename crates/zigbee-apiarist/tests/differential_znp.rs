// PTY transcript tests for the ported ZNP layer (`znp.ts`).
//
// A fake coordinator answers the same UNPI frames as the real stick. This
// covers SREQ/SRSP status handling, AREQ matching by transaction id and by ZDO
// `eui64`, the `received` stream and waiter timeouts without any hardware.

use serde_json::json;
use tokio::io::AsyncWriteExt;
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;
use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

const EUI64: &str = "0x00124b0024c2d400";

fn spawn_fake_coordinator() -> (AdapterTransport, tokio::task::JoinHandle<()>) {
    let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pair");
    let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
    transport.attach_serial(reader).expect("attach");
    let server = tokio::spawn(async move {
        let mut buffer = [0u8; 512];
        let mut pending = Vec::new();
        loop {
            let Ok(read) = peer.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            pending.extend_from_slice(&buffer[..read]);
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
                let frame = Frame::from_buffer(length as u8, 4 + length, &raw).expect("frame");
                let object = ZpiObject::from_unpi_frame(&frame);
                let command_id = frame.commandID;
                let subsystem = frame.subsystem;
                let name = object.command.name.as_str();

                let srsp = |data: Vec<u8>| {
                    Frame::new(Type::SRSP, subsystem, command_id, data, 0, 0).to_buffer()
                };

                match name {
                    "ping" => {
                        peer.write_all(&srsp(vec![0x01, 0x00]))
                            .await
                            .expect("write");
                    }
                    "version" => {
                        peer.write_all(&srsp(vec![2, 2, 7, 1, 0, 0x4d, 0x37, 0x0e, 0x02]))
                            .await
                            .expect("write");
                    }
                    "osalNvWriteExt" => {
                        // INVALID_PARAM
                        peer.write_all(&srsp(vec![0x02])).await.expect("write");
                    }
                    "dataRequest" => {
                        peer.write_all(&srsp(vec![0x00])).await.expect("write");
                        let confirm = Frame::new(
                            Type::AREQ,
                            Subsystem::AF,
                            128,
                            vec![0x00, 0x01, 0x07],
                            0,
                            0,
                        );
                        peer.write_all(&confirm.to_buffer()).await.expect("write");
                    }
                    "nwkAddrReq" => {
                        peer.write_all(&srsp(vec![0x00])).await.expect("write");
                        let mut data = vec![0x00];
                        data.extend_from_slice(&[0x00, 0xd4, 0xc2, 0x24, 0x00, 0x4b, 0x12, 0x00]);
                        data.extend_from_slice(&0x1234u16.to_le_bytes());
                        data.push(0x00); // startIndex
                        data.push(0x00); // assoc device count
                        let response = Frame::new(Type::AREQ, Subsystem::ZDO, 0x80, data, 0, 0);
                        peer.write_all(&response.to_buffer()).await.expect("write");
                    }
                    other => panic!("unexpected command '{other}'"),
                }
            }
        }
    });
    (transport, server)
}

async fn open() -> (Znp, tokio::task::JoinHandle<()>) {
    let (transport, server) = spawn_fake_coordinator();
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open");
    (znp, server)
}

#[tokio::test]
async fn resolves_areq_by_transaction_id() {
    let (mut znp, server) = open().await;
    let (waiter_id, receiver) = znp
        .wait_for(
            Type::AREQ,
            Subsystem::AF,
            "dataConfirm",
            None,
            Some(7),
            None,
            Some(1000),
        )
        .await;
    znp.request(
        Subsystem::AF,
        "dataRequest",
        json!({
            "dstaddr": 0x1234,
            "destendpoint": 1,
            "srcendpoint": 1,
            "clusterid": 6,
            "transid": 7,
            "options": 0,
            "radius": 30,
            "len": 0,
            "data": [],
        }),
        Some(waiter_id),
        None,
        &[0],
    )
    .await
    .expect("request");
    let confirm = receiver.await.expect("waiter").expect("confirm");
    assert_eq!(confirm.payload.get("transid"), Some(&json!(7)));
    assert_eq!(confirm.payload.get("status"), Some(&json!(0)));

    let received = znp.drain_received().await;
    assert!(received
        .iter()
        .any(|object| object.command.name == "dataConfirm"));
    znp.close().await.expect("close");
    server.abort();
}

#[tokio::test]
async fn resolves_zdo_areq_by_eui64() {
    let (mut znp, server) = open().await;
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NETWORK_ADDRESS_REQUEST,
        &[json!(EUI64), json!(false), json!(0)],
    );
    let (waiter_id, receiver) = znp
        .wait_for(
            Type::AREQ,
            Subsystem::ZDO,
            "nwkAddrRsp",
            Some(json!(EUI64)),
            None,
            None,
            Some(1000),
        )
        .await;
    znp.request_zdo(ClusterId::NETWORK_ADDRESS_REQUEST, payload, Some(waiter_id))
        .await
        .expect("request zdo");
    let response = receiver.await.expect("waiter").expect("response");
    let zdo = response.payload.get("zdo").expect("zdo");
    assert_eq!(zdo[0], json!(0));
    assert_eq!(zdo[1]["eui64"], json!(EUI64));
    assert_eq!(zdo[1]["nwkAddress"], json!(0x1234));
    znp.close().await.expect("close");
    server.abort();
}

#[tokio::test]
async fn reports_non_success_srsp_status() {
    let (mut znp, server) = open().await;
    let error = znp
        .request(
            Subsystem::SYS,
            "osalNvWriteExt",
            json!({"id": 58, "offset": 0, "len": 17, "value": vec![0u8; 17]}),
            None,
            None,
            &[0],
        )
        .await
        .expect_err("must fail");
    assert!(error.contains("INVALID_PARAM"), "{error}");
    znp.close().await.expect("close");
    server.abort();
}

#[tokio::test]
async fn times_out_when_areq_never_arrives() {
    let (mut znp, server) = open().await;
    let (_waiter_id, receiver) = znp
        .wait_for(
            Type::AREQ,
            Subsystem::ZDO,
            "leaveInd",
            None,
            None,
            None,
            Some(50),
        )
        .await;
    let result = receiver.await.expect("waiter");
    assert!(result.is_err());
    znp.close().await.expect("close");
    server.abort();
}

#[tokio::test]
async fn skips_cc2530_bootloader_with_magic_byte() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pair");
    let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
    transport.attach_serial(reader).expect("attach");

    let magic_seen = Arc::new(AtomicBool::new(false));
    let seen = magic_seen.clone();
    let server = tokio::spawn(async move {
        let mut buffer = [0u8; 256];
        let mut pending = Vec::new();
        let mut first_ping = true;
        loop {
            let Ok(read) = peer.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            pending.extend_from_slice(&buffer[..read]);
            // The magic byte is sent raw, outside of UNPI framing.
            while let Some(position) = pending.iter().position(|byte| *byte == 0xef) {
                pending.drain(..=position);
                seen.store(true, Ordering::SeqCst);
            }
            loop {
                if pending.len() < 5 || pending[0] != 0xfe {
                    break;
                }
                let length = pending[1] as usize;
                let frame_length = 4 + length + 1;
                if pending.len() < frame_length {
                    break;
                }
                let raw = pending[..frame_length].to_vec();
                pending.drain(..frame_length);
                let frame = Frame::from_buffer(length as u8, 4 + length, &raw).expect("frame");
                let object = ZpiObject::from_unpi_frame(&frame);
                assert_eq!(object.command.name, "ping");
                if first_ping {
                    // Simulate a coordinator stuck in the bootloader: no SRSP.
                    first_ping = false;
                    continue;
                }
                let response = Frame::new(
                    Type::SRSP,
                    Subsystem::SYS,
                    frame.commandID,
                    vec![0x01, 0x00],
                    0,
                    0,
                );
                peer.write_all(&response.to_buffer()).await.expect("write");
            }
        }
    });

    let mut znp = Znp::new(transport);
    znp.open().await.expect("open after bootloader skip");
    assert!(magic_seen.load(Ordering::SeqCst), "0xef was not written");
    znp.close().await.expect("close");
    server.abort();
}
