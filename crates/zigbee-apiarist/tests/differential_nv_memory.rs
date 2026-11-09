// Wire-level NV memory test: a PTY-backed ZNP server answers the same UNPI
// requests the real coordinator would. Verifies AdapterNvMemory against the
// upstream request/response sequence without hardware.

use std::collections::BTreeMap;

use serde_json::Value;
use tokio::io::AsyncWriteExt;
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds;
use zigbee_apiarist::adapter::z_stack::structs::r#struct::Alignment;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::ZpiObject;

fn value_bytes(payload: &Value, key: &str) -> Vec<u8> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect()
        })
        .unwrap_or_default()
}

async fn respond(
    peer: &mut serial2_tokio::SerialPort,
    frame: &Frame,
    items: &mut BTreeMap<u16, Vec<u8>>,
) {
    let object = ZpiObject::from_unpi_frame(frame);
    let name = object.command.name.as_str();
    let payload = &object.payload;

    let data = match name {
        "ping" => vec![0x01, 0x00],
        "osalNvLength" => {
            let id = payload["id"].as_u64().unwrap() as u16;
            let length = items.get(&id).map(Vec::len).unwrap_or(0) as u16;
            length.to_le_bytes().to_vec()
        }
        "osalNvReadExt" => {
            let id = payload["id"].as_u64().unwrap() as u16;
            let offset = payload["offset"].as_u64().unwrap() as usize;
            let value = items.get(&id).cloned().unwrap_or_default();
            let end = (offset + 240).min(value.len());
            let chunk = &value[offset..end];
            let mut data = vec![0x00, chunk.len() as u8];
            data.extend_from_slice(chunk);
            data
        }
        "osalNvWriteExt" => {
            let id = payload["id"].as_u64().unwrap() as u16;
            let offset = payload["offset"].as_u64().unwrap() as usize;
            let value = value_bytes(payload, "value");
            let item = items.entry(id).or_default();
            if item.len() < offset + value.len() {
                item.resize(offset + value.len(), 0);
            }
            item[offset..offset + value.len()].copy_from_slice(&value);
            vec![0x00]
        }
        "osalNvItemInit" => vec![0x09],
        "osalNvDelete" => {
            let id = payload["id"].as_u64().unwrap() as u16;
            items.remove(&id);
            vec![0x00]
        }
        "nvLength" => {
            let id = payload["itemid"].as_u64().unwrap() as u16;
            let length = items.get(&id).map(Vec::len).unwrap_or(0) as u8;
            vec![length]
        }
        "nvCreate" => {
            let id = payload["itemid"].as_u64().unwrap() as u16;
            let length = payload["len"].as_u64().unwrap() as usize;
            items.insert(id, vec![0; length]);
            vec![0x00]
        }
        "nvRead" => {
            let id = payload["itemid"].as_u64().unwrap() as u16;
            let offset = payload["offset"].as_u64().unwrap() as usize;
            let length = payload["len"].as_u64().unwrap() as usize;
            let value = items.get(&id).cloned().unwrap_or_default();
            let end = (offset + length).min(value.len());
            let chunk = &value[offset..end];
            let mut data = vec![0x00, chunk.len() as u8];
            data.extend_from_slice(chunk);
            data
        }
        "nvWrite" => {
            let id = payload["itemid"].as_u64().unwrap() as u16;
            let offset = payload["offset"].as_u64().unwrap() as usize;
            let value = value_bytes(payload, "value");
            let item = items.entry(id).or_default();
            if item.len() < offset + value.len() {
                item.resize(offset + value.len(), 0);
            }
            item[offset..offset + value.len()].copy_from_slice(&value);
            vec![0x00]
        }
        other => panic!("unexpected NV command '{other}'"),
    };

    let response = Frame::new(Type::SRSP, Subsystem::SYS, frame.commandID, data, 0, 0);
    peer.write_all(&response.to_buffer()).await.expect("write");
}

#[tokio::test]
async fn nv_memory_roundtrip_over_unpi() {
    let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pair");
    let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
    transport.attach_serial(reader).expect("attach");
    let mut znp = Znp::new(transport);

    let mut items = BTreeMap::new();
    items.insert(NvItemsIds::NWKKEY, vec![0x11; 21]);

    let server = tokio::spawn(async move {
        let mut buffer = [0u8; 256];
        let mut pending = Vec::new();
        loop {
            let read = match peer.read(&mut buffer).await {
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
                respond(&mut peer, &frame, &mut items).await;
            }
        }
    });

    znp.open().await.expect("open");
    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("init");
    assert_eq!(nv.memory_alignment, Some(Alignment::Unaligned));

    let data = vec![0xaa; 30];
    nv.write_item(NvItemsIds::STARTUP_OPTION, &data, 0, true)
        .await
        .expect("write");
    let read = nv
        .read_item(NvItemsIds::STARTUP_OPTION, 0)
        .await
        .expect("read")
        .expect("exists");
    assert_eq!(read, data);

    nv.delete_item(NvItemsIds::STARTUP_OPTION)
        .await
        .expect("delete");
    assert!(nv
        .read_item(NvItemsIds::STARTUP_OPTION, 0)
        .await
        .expect("read")
        .is_none());

    nv.znp.close().await.expect("close");
    server.abort();
}

#[test]
fn parses_osal_nv_read_ext_response() {
    let mut data = vec![0x00, 0x15];
    data.extend_from_slice(&[0x11; 21]);
    let frame = Frame::new(Type::SRSP, Subsystem::SYS, 28, data, 0, 0);
    let object = ZpiObject::from_unpi_frame(&frame);
}
