// End-to-end wire capture without hardware.
//
// A scripted TCP coordinator speaks UNPI like a ser2net device. The Z-Stack
// adapter is driven through its normal ZCL broadcast path; every byte written
// to the "serial port" is captured to a file and compared with a golden
// transcript generated from the pinned upstream `ZpiObject.createRequest`.

use std::sync::Arc;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame as UnpiFrame;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;
use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};
use zigbee_apiarist::utils::queue::Queue;
use zigbee_apiarist::zspec::consts::HA_PROFILE_ID;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::Direction;

/// Generated with:
/// `zpi_object create_request AF dataRequestExt
///  {dstaddrmode:2,dstaddr:"0x000000000000fffd",destendpoint:1,dstpanid:0,
///   srcendpoint:1,clusterid:6,transid:1,options:0,radius:30,len:3,
///   data:[17,7,1]}`
const GOLDEN_AF_DATA_REQUEST: &str = "fe17240202fdff00000000000001000001060001001e03001107013c";

async fn run_coordinator(listener: TcpListener, captured: mpsc::UnboundedSender<Vec<u8>>) {
    let (mut socket, _) = listener.accept().await.expect("accept");
    let mut buffer = [0u8; 1024];
    let mut pending = Vec::new();

    loop {
        let read = match socket.read(&mut buffer).await {
            Ok(0) => return,
            Ok(read) => read,
            Err(_) => return,
        };
        pending.extend_from_slice(&buffer[..read]);

        if !pending.is_empty() && pending[0] != 0xfe {
            // `Znp.open` writes the raw CC2530 skip-bootloader payload before
            // the first UNPI frame; resynchronise on SOF.
            if let Some(start) = pending.iter().position(|byte| *byte == 0xfe) {
                pending.drain(..start);
            } else {
                pending.clear();
            }
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

            let frame = UnpiFrame::from_buffer(length as u8, 4 + length, &raw).expect("unpi");
            let object = ZpiObject::from_unpi_frame(&frame);
            if object.subsystem == Subsystem::AF {
                let _ = captured.send(raw);
            }

            if object.subsystem == Subsystem::AF || object.subsystem == Subsystem::SYS {
                let response =
                    UnpiFrame::new(Type::SRSP, object.subsystem, frame.commandID, vec![0], 0, 0);
                socket
                    .write_all(&response.to_buffer())
                    .await
                    .expect("write srsp");

                if object.command.name == "dataRequest" {
                    let transid = object
                        .payload
                        .get("transid")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0) as u8;
                    let confirm =
                        UnpiFrame::new(Type::AREQ, Subsystem::AF, 128, vec![0, 1, transid], 0, 0);
                    socket
                        .write_all(&confirm.to_buffer())
                        .await
                        .expect("write dataConfirm");
                }
            }
        }
    }
}

#[tokio::test]
async fn zcl_broadcast_is_captured_to_file_and_matches_upstream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx));

    let transport = AdapterTransport::new(TransportOptions {
        path: format!("tcp://127.0.0.1:{port}"),
        baud_rate: None,
        rtscts: Some(false),
    });
    let adapter = Arc::new(Mutex::new(ZStackAdapter::new(
        transport,
        ConfigNetworkOptions {
            pan_id: 0x1a62,
            extended_pan_id: Some(vec![0xdd; 8]),
            network_key: Some(vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13]),
            channel_list: vec![11],
            network_key_distribute: Some(false),
        },
        String::new(),
        AdapterOptions::default(),
    )));

    {
        let mut adapter = adapter.lock().await;
        adapter.znp.lock().await.open().await.expect("open znp");
        adapter.queue = Some(Queue::new(1));
    }

    let frame = Frame::create(
        1,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        7,
        "on",
        "genOnOff",
        json!({}),
        json!({}),
        0,
    );

    adapter
        .lock()
        .await
        .send_zcl_frame_to_all(1, &frame, 1, 0xfffd, Some(HA_PROFILE_ID))
        .await
        .expect("broadcast");

    let captured = captured_rx.recv().await.expect("captured frame");
    let captured_hex: String = captured.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(captured_hex, GOLDEN_AF_DATA_REQUEST);

    // The same bytes are persisted instead of written to a serial device.
    let path = std::env::temp_dir().join(format!(
        "z2m-wire-{}.hex",
        zigbee_apiarist::utils::datetime::now_ms()
    ));
    std::fs::write(&path, format!("{captured_hex}\n")).expect("write capture");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read capture"),
        format!("{GOLDEN_AF_DATA_REQUEST}\n")
    );
    let _ = std::fs::remove_file(path);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

/// Generated with:
/// `zpi_object create_request AF dataRequest
///  {dstaddr:4660,destendpoint:1,srcendpoint:1,clusterid:6,transid:1,
///   options:0,radius:30,len:3,data:[17,7,1]}`
const GOLDEN_AF_DATA_REQUEST_ENDPOINT: &str = "fe0d240134120101060001001e0311070103";

#[tokio::test]
async fn endpoint_command_is_captured_to_file_and_matches_upstream() {
    use serde_json::Value;
    use std::sync::Mutex as StdMutex;
    use zigbee_apiarist::adapter::z_stack::runtime_adapter::ZStackRuntimeAdapter;
    use zigbee_apiarist::controller::database::Database;
    use zigbee_apiarist::controller::database::DeviceType;
    use zigbee_apiarist::controller::model::device::{Device, InterviewState};
    use zigbee_apiarist::controller::model::endpoint::Options;
    use zigbee_apiarist::controller::model::entity::Entity;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx));

    let transport = AdapterTransport::new(TransportOptions {
        path: format!("tcp://127.0.0.1:{port}"),
        baud_rate: None,
        rtscts: Some(false),
    });
    let adapter = Arc::new(Mutex::new(ZStackAdapter::new(
        transport,
        ConfigNetworkOptions {
            pan_id: 0x1a62,
            extended_pan_id: Some(vec![0xdd; 8]),
            network_key: Some(vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13]),
            channel_list: vec![11],
            network_key_distribute: Some(false),
        },
        String::new(),
        AdapterOptions::default(),
    )));

    {
        let mut adapter = adapter.lock().await;
        adapter.znp.lock().await.open().await.expect("open znp");
        adapter.queue = Some(Queue::new(1));
    }

    let runtime = Arc::new(ZStackRuntimeAdapter::new(adapter.clone()).await);
    let db_path = std::env::temp_dir().join(format!(
        "z2m-wire-endpoint-{}.db",
        zigbee_apiarist::utils::datetime::now_ms()
    ));
    Entity::inject_database(Arc::new(StdMutex::new(Database::open(&db_path))));
    Entity::inject_adapter(runtime);
    Device::reset_cache();
    zigbee_apiarist::controller::model::group::Group::reset_cache();

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
    let endpoint = device.create_endpoint(1).expect("create endpoint");

    endpoint
        .command(
            Value::from("genOnOff"),
            Value::from("on"),
            json!({}),
            Some(Options {
                transaction_sequence_number: Some(7),
                disable_response: Some(true),
                disable_default_response: Some(true),
                timeout: Some(1000),
                ..Default::default()
            }),
        )
        .await
        .expect("endpoint command");

    let captured = captured_rx.recv().await.expect("captured frame");
    let captured_hex: String = captured.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(captured_hex, GOLDEN_AF_DATA_REQUEST_ENDPOINT);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
    let _ = std::fs::remove_file(db_path);
}
