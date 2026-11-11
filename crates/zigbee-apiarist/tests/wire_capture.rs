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

async fn recv_subsystem(
    receiver: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    subsystem: Subsystem,
) -> UnpiFrame {
    loop {
        let raw = receiver.recv().await.expect("captured frame");
        let length = raw[1] as usize;
        let frame = UnpiFrame::from_buffer(length as u8, 4 + length, &raw).expect("unpi");
        if frame.subsystem == subsystem {
            return frame;
        }
    }
}

async fn run_coordinator(
    listener: TcpListener,
    captured: mpsc::UnboundedSender<Vec<u8>>,
    fail_first_confirm: bool,
    forced_confirm_status: Option<u8>,
) {
    let mut first_confirm = true;
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
            let _ = captured.send(raw);

            if object.subsystem == Subsystem::AF
                || object.subsystem == Subsystem::SYS
                || object.subsystem == Subsystem::ZDO
                || object.subsystem == Subsystem::UTIL
            {
                let response =
                    UnpiFrame::new(Type::SRSP, object.subsystem, frame.commandID, vec![0], 0, 0);
                socket
                    .write_all(&response.to_buffer())
                    .await
                    .expect("write srsp");

                if object.command.name == "dataRequest" || object.command.name == "dataRequestExt" {
                    let transid = object
                        .payload
                        .get("transid")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0) as u8;
                    let status = if let Some(status) = forced_confirm_status {
                        status
                    } else if fail_first_confirm && first_confirm {
                        first_confirm = false;
                        225u8
                    } else {
                        0u8
                    };
                    let confirm = UnpiFrame::new(
                        Type::AREQ,
                        Subsystem::AF,
                        128,
                        vec![status, 1, transid],
                        0,
                        0,
                    );
                    socket
                        .write_all(&confirm.to_buffer())
                        .await
                        .expect("write dataConfirm");
                }

                if object.command.name == "resetReq" {
                    let reset_ind = UnpiFrame::new(
                        Type::AREQ,
                        Subsystem::SYS,
                        128,
                        vec![0, 0, 2, 7, 1, 0, 0],
                        0,
                        0,
                    );
                    socket
                        .write_all(&reset_ind.to_buffer())
                        .await
                        .expect("write resetInd");
                }

                if object.command.name == "mgmtPermitJoinReq" {
                    // `mgmtPermitJoinRsp`: srcaddr (2 bytes) + status.
                    let src_lo = frame.data.get(1).copied().unwrap_or(0);
                    let src_hi = frame.data.get(2).copied().unwrap_or(0);
                    let response = UnpiFrame::new(
                        Type::AREQ,
                        Subsystem::ZDO,
                        182,
                        vec![src_lo, src_hi, 0],
                        0,
                        0,
                    );
                    socket
                        .write_all(&response.to_buffer())
                        .await
                        .expect("write permit join response");
                }
            }
        }
    }
}

async fn test_adapter(
    forced_confirm_status: Option<u8>,
) -> (
    Arc<Mutex<ZStackAdapter>>,
    mpsc::UnboundedReceiver<Vec<u8>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(
        listener,
        captured_tx,
        false,
        forced_confirm_status,
    ));

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
    (adapter, captured_rx, server)
}

#[tokio::test]
async fn zcl_broadcast_is_captured_to_file_and_matches_upstream() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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

    let captured = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let captured_hex: String = captured
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
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
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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

    let captured = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let captured_hex: String = captured
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(captured_hex, GOLDEN_AF_DATA_REQUEST_ENDPOINT);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
    let _ = std::fs::remove_file(db_path);
}

/// Generated with:
/// `zpi_object create_request AF dataRequestExt
///  {dstaddrmode:1,dstaddr:"0x0000000000000019",destendpoint:255,dstpanid:0,
///   srcendpoint:1,clusterid:0,transid:1,options:0,radius:30,len:5,
///   data:[16,100,0,0,0]}`
const GOLDEN_AF_GROUP: &str = "fe192402011900000000000000ff000001000001001e05001064000000b7";

#[tokio::test]
async fn group_send_matches_upstream_adapter_test() {
    use zigbee_apiarist::zspec::zcl::zcl_header::FrameType;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        100,
        "read",
        0,
        json!([{"attrId": 0}]),
        json!({}),
        0,
    );
    adapter
        .lock()
        .await
        .send_zcl_frame_to_group(25, &frame, Some(1), Some(HA_PROFILE_ID))
        .await
        .expect("group send");

    let captured = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let captured_hex: String = captured
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(captured_hex, GOLDEN_AF_GROUP);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn permit_join_zdo_frame_matches_upstream_adapter_test() {
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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

    assert_eq!(
        zigbee_apiarist::adapter::z_stack::znp::definition::find_zdo_response(
            Subsystem::ZDO,
            0x8036
        )
        .map(|command| command.name),
        Some("mgmtPermitJoinRsp")
    );
    let probe = UnpiFrame::new(Type::AREQ, Subsystem::ZDO, 182, vec![0xd2, 0x04, 0], 0, 0);
    let probe_object = ZpiObject::from_unpi_frame(&probe);
    assert_eq!(probe_object.payload["srcaddr"], json!(1234));
    assert_eq!(probe_object.payload["zdo"], json!([0, null]));

    adapter
        .lock()
        .await
        .permit_join(250, Some(1234))
        .await
        .expect("permit join");

    let captured = recv_subsystem(&mut captured_rx, Subsystem::ZDO).await;
    let object = ZpiObject::from_unpi_frame(&captured);
    assert_eq!(object.command.name, "mgmtPermitJoinReq");

    let zdo_payload = BuffaloZdo::build_request(
        false,
        ClusterId::PERMIT_JOINING_REQUEST,
        &[json!(250), json!(1), json!([])],
    );
    let mut expected = vec![2, 0xd2, 0x04];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(captured.data, expected);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

/// Generated with:
/// `zpi_object create_request AF dataRequestExt
///  {dstaddrmode:3,dstaddr:"0x0017880104c9cd33",destendpoint:254,
///   dstpanid:65535,srcendpoint:12,clusterid:4096,transid:1,options:0,
///   radius:30,len:9,data:[1,12,6,1,0,0,0,255,255]}`
const GOLDEN_INTERPAN_IDENTIFY: &str =
    "fe1d24020333cdc90401881700feffff0c001001001e0900010c0601000000ffff6b";

#[tokio::test]
async fn interpan_to_ieee_matches_upstream_adapter_test() {
    use zigbee_apiarist::zspec::zcl::zcl_header::FrameType;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        12,
        "identifyRequest",
        "touchlink",
        json!({"transactionID": 1, "duration": 65535}),
        json!({}),
        0,
    );
    adapter
        .lock()
        .await
        .send_zcl_frame_inter_pan_to_ieee_addr(&frame, "0x0017880104c9cd33")
        .await
        .expect("interpan identify");

    let captured = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let captured_hex: String = captured
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(captured_hex, GOLDEN_INTERPAN_IDENTIFY);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

const GOLDEN_RETRY_FIRST: &str = "fe10240102001401000001001e061064050000004a";
const GOLDEN_RETRY_SECOND: &str = "fe10240102001401000002001e0610640500000049";

#[tokio::test]
async fn mac_channel_access_failure_retries_with_new_transaction_id() {
    use zigbee_apiarist::zspec::zcl::zcl_header::FrameType;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, true, None));

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
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        100,
        "writeNoRsp",
        0,
        json!([{"attrId": 0, "dataType": 0, "attrData": null}]),
        json!({}),
        0,
    );
    adapter
        .lock()
        .await
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, true, false, None, None)
        .await
        .expect("retry send");

    let first = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let second = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let first_hex: String = first
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let second_hex: String = second
        .to_buffer()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(first_hex, GOLDEN_RETRY_FIRST);
    assert_eq!(second_hex, GOLDEN_RETRY_SECOND);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn zdo_request_payloads_match_upstream_adapter_tests() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (captured_tx, mut captured_rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(run_coordinator(listener, captured_tx, false, None));

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

    // "Sends proper ZDO request payload for NETWORK_ADDRESS_REQUEST"
    let zdo_payload = BuffaloZdo::build_request(
        false,
        ClusterId::NETWORK_ADDRESS_REQUEST,
        &[json!("0x1122334455667788"), json!(false), json!(0)],
    );
    adapter
        .lock()
        .await
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &zdo_payload,
            true,
        )
        .await
        .expect("network address request");
    let captured = recv_subsystem(&mut captured_rx, Subsystem::ZDO).await;
    assert_eq!(captured.data, zdo_payload);

    // "Sends proper ZDO request payload for NWK_UPDATE_REQUEST UNICAST"
    let zdo_payload = BuffaloZdo::build_request(
        false,
        ClusterId::NWK_UPDATE_REQUEST,
        &[
            json!([15]),
            json!(0xfe),
            json!(0),
            serde_json::Value::Null,
            json!(0),
        ],
    );
    adapter
        .lock()
        .await
        .send_zdo(
            BLANK_EUI64,
            0x123,
            ClusterId::NWK_UPDATE_REQUEST,
            &zdo_payload,
            true,
        )
        .await
        .expect("nwk update request");
    let captured = recv_subsystem(&mut captured_rx, Subsystem::ZDO).await;
    let mut expected = vec![0x23, 0x01, 2];
    expected.extend_from_slice(&zdo_payload);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(captured.data, expected);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn to_all_default_and_sleepy_match_upstream_adapter_test() {
    use zigbee_apiarist::zspec::enums::BroadcastAddress;

    let (adapter, mut captured_rx, server) = test_adapter(None).await;
    let frame = Frame::create(
        0,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        100,
        "read",
        0,
        json!([{"attrId": 0}]),
        json!({}),
        0,
    );

    adapter
        .lock()
        .await
        .send_zcl_frame_to_all(242, &frame, 250, BroadcastAddress::DEFAULT as u16, None)
        .await
        .expect("default broadcast");
    let first = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let first_hex: String = first
        .to_buffer()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        first_hex,
        "fe19240202fcff000000000000f20000fa000001001e0500106400000058"
    );

    adapter
        .lock()
        .await
        .send_zcl_frame_to_all(242, &frame, 250, BroadcastAddress::SLEEPY as u16, None)
        .await
        .expect("sleepy broadcast");
    let second = recv_subsystem(&mut captured_rx, Subsystem::AF).await;
    let second_hex: String = second
        .to_buffer()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        second_hex,
        "fe19240202ffff000000000000f20000fa000002001e0500106400000058"
    );

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn data_confirm_failure_errors_for_endpoint_and_group() {
    let (adapter, _captured_rx, server) = test_adapter(Some(201)).await;
    let frame = Frame::create(
        0,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        100,
        "read",
        0,
        json!([{"attrId": 0}]),
        json!({}),
        0,
    );

    let endpoint_error = adapter
        .lock()
        .await
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 1000, true, false, None, None)
        .await
        .expect_err("endpoint dataConfirm failure");
    assert!(
        endpoint_error.contains("201")
            || endpoint_error.contains("0xc9")
            || endpoint_error.contains("Data request failed")
    );

    let group_error = adapter
        .lock()
        .await
        .send_zcl_frame_to_group(25, &frame, Some(1), Some(HA_PROFILE_ID))
        .await
        .expect_err("group dataConfirm failure");
    assert!(
        group_error.contains("201")
            || group_error.contains("0xc9")
            || group_error.contains("Data request failed")
    );

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn permit_join_all_and_coordinator_match_upstream_adapter_test() {
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let (adapter, mut captured_rx, server) = test_adapter(None).await;

    adapter
        .lock()
        .await
        .permit_join(100, None)
        .await
        .expect("permit join all");
    let frame = recv_subsystem(&mut captured_rx, Subsystem::ZDO).await;
    let zdo_payload = BuffaloZdo::build_request(
        false,
        ClusterId::PERMIT_JOINING_REQUEST,
        &[json!(100), json!(1), json!([])],
    );
    let mut expected = vec![15, 0xfc, 0xff];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(frame.data, expected);

    adapter
        .lock()
        .await
        .permit_join(254, Some(0))
        .await
        .expect("permit join coordinator");
    let frame = recv_subsystem(&mut captured_rx, Subsystem::ZDO).await;
    let zdo_payload = BuffaloZdo::build_request(
        false,
        ClusterId::PERMIT_JOINING_REQUEST,
        &[json!(254), json!(1), json!([])],
    );
    let mut expected = vec![2, 0, 0];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(frame.data, expected);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}

#[tokio::test]
async fn reset_soft_and_hard_match_upstream_adapter_test() {
    let (adapter, mut captured_rx, server) = test_adapter(None).await;

    adapter
        .lock()
        .await
        .reset("soft")
        .await
        .expect("soft reset");
    let soft = loop {
        let frame = recv_subsystem(&mut captured_rx, Subsystem::SYS).await;
        if ZpiObject::from_unpi_frame(&frame).command.name == "resetReq" {
            break frame;
        }
    };
    assert_eq!(soft.data, vec![1]);

    adapter
        .lock()
        .await
        .reset("hard")
        .await
        .expect("hard reset");
    let hard = loop {
        let frame = recv_subsystem(&mut captured_rx, Subsystem::SYS).await;
        if ZpiObject::from_unpi_frame(&frame).command.name == "resetReq" {
            break frame;
        }
    };
    assert_eq!(hard.data, vec![0]);

    adapter.lock().await.znp.lock().await.close().await.ok();
    server.abort();
}
