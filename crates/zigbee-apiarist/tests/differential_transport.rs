// TCP transport transcript: a local TCP server speaks UNPI like a coordinator
// reached over a ser2net-style socket. Verifies `tcp://` parsing, socket open,
// reader/writer wiring and a full ZNP request round-trip without hardware.

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;

#[tokio::test]
async fn znp_request_over_tcp_transport() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buffer = [0u8; 512];
        let mut pending = Vec::new();
        loop {
            let read = match socket.read(&mut buffer).await {
                Ok(0) => return,
                Ok(read) => read,
                Err(_) => return,
            };
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
                let data = match object.command.name.as_str() {
                    "ping" => vec![0x01, 0x00],
                    "version" => vec![2, 1, 2, 7, 1, 0x51, 0x0b, 0x35, 0x01],
                    other => panic!("unexpected command '{other}'"),
                };
                let response = Frame::new(Type::SRSP, Subsystem::SYS, frame.commandID, data, 0, 0);
                socket
                    .write_all(&response.to_buffer())
                    .await
                    .expect("write");
            }
        }
    });

    let transport = AdapterTransport::new(TransportOptions {
        path: format!("tcp://127.0.0.1:{port}"),
        baud_rate: None,
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open tcp transport");
    let version = znp
        .request_with_reply(Subsystem::SYS, "version", json!({}))
        .await
        .expect("version");
    assert_eq!(version.payload.get("product"), Some(&json!(1)));
    assert_eq!(version.payload.get("majorrel"), Some(&json!(2)));
    assert_eq!(version.payload.get("minorrel"), Some(&json!(7)));
    znp.close().await.expect("close");
    server.abort();
}
