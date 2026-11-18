//! Minimal in-process MQTT 3.1.1 broker for integration tests.
//!
//! Shared by `tests/mqtt_reconnect.rs` (CP-02) and
//! `tests/controller_startup.rs` (CP-05). It accepts
//! CONNECT/SUBSCRIBE/PUBLISH/PINGREQ, stores retained messages, records every
//! publish (topic, payload, retain, qos), tracks active client connections and
//! can drop the client connections to force a reconnect.
//!
//! Upstream anchors: `lib/mqtt.ts` `onConnect` (state publish, subscribe, 2 s
//! retained republish and the `bridge/info` cancel) and `publish`
//! (`force_disable_retain`, retained tracking).

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Published {
    pub topic: String,
    pub payload: Vec<u8>,
    pub retain: bool,
    pub qos: u8,
}

pub struct Broker {
    port: u16,
    retained: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    publishes: Arc<Mutex<Vec<Published>>>,
    subscriptions: Arc<Mutex<Vec<String>>>,
    connections: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
    active: Arc<AtomicUsize>,
}

impl Broker {
    pub async fn start() -> Arc<Broker> {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().unwrap().port();
        let retained: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let publishes: Arc<Mutex<Vec<Published>>> = Arc::new(Mutex::new(Vec::new()));
        let subscriptions: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let connections: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));

        let broker = Arc::new(Broker {
            port,
            retained: retained.clone(),
            publishes: publishes.clone(),
            subscriptions: subscriptions.clone(),
            connections: connections.clone(),
            active: active.clone(),
        });

        let state = (retained, publishes, subscriptions, connections);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let (retained, publishes, subscriptions, connections) = state.clone();
                let active = active.clone();
                let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
                connections.lock().unwrap().push(stop_tx);
                active.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let _guard = ActiveGuard(active);
                    let mut buffer: Vec<u8> = Vec::new();
                    loop {
                        let mut chunk = [0u8; 4096];
                        let read = tokio::select! {
                            _ = &mut stop_rx => return,
                            read = socket.read(&mut chunk) => read,
                        };
                        let Ok(read) = read else { return };
                        if read == 0 {
                            return;
                        }
                        buffer.extend_from_slice(&chunk[..read]);
                        loop {
                            let Some((packet_type, flags, header_len, body_len)) = frame(&buffer)
                            else {
                                break;
                            };
                            if buffer.len() < header_len + body_len {
                                break;
                            }
                            let body = buffer[header_len..header_len + body_len].to_vec();
                            buffer.drain(..header_len + body_len);
                            if !handle(
                                &mut socket,
                                packet_type,
                                flags,
                                &body,
                                &retained,
                                &publishes,
                                &subscriptions,
                            )
                            .await
                            {
                                return;
                            }
                        }
                    }
                });
            }
        });

        broker
    }

    pub fn server(&self) -> String {
        format!("mqtt://127.0.0.1:{}", self.port)
    }

    pub fn seen(&self, topic: &str) -> Vec<Published> {
        self.publishes
            .lock()
            .unwrap()
            .iter()
            .filter(|publish| publish.topic == topic)
            .cloned()
            .collect()
    }

    pub fn preload_retained(&self, topic: &str, payload: &str) {
        self.retained
            .lock()
            .unwrap()
            .insert(topic.to_string(), payload.as_bytes().to_vec());
    }

    pub fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.lock().unwrap().clone()
    }

    pub fn active_connections(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    pub fn drop_connections(&self) {
        let mut connections = self.connections.lock().unwrap();
        for sender in connections.drain(..) {
            let _ = sender.send(());
        }
    }
}

struct ActiveGuard(Arc<AtomicUsize>);

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn encode_length(mut length: usize) -> Vec<u8> {
    let mut encoded = Vec::new();
    loop {
        let mut byte = (length % 128) as u8;
        length /= 128;
        if length > 0 {
            byte |= 0x80;
        }
        encoded.push(byte);
        if length == 0 {
            return encoded;
        }
    }
}

fn frame(buffer: &[u8]) -> Option<(u8, u8, usize, usize)> {
    if buffer.is_empty() {
        return None;
    }
    let packet_type = buffer[0] >> 4;
    let flags = buffer[0] & 0x0f;
    let mut index = 1;
    let mut length = 0usize;
    let mut multiplier = 1usize;
    loop {
        let byte = *buffer.get(index)?;
        index += 1;
        length += (byte & 0x7f) as usize * multiplier;
        if byte & 0x80 == 0 {
            break;
        }
        multiplier *= 128;
        if multiplier > 128 * 128 * 128 {
            return None;
        }
    }
    Some((packet_type, flags, index, length))
}

async fn handle(
    socket: &mut tokio::net::TcpStream,
    packet_type: u8,
    flags: u8,
    body: &[u8],
    retained: &Arc<Mutex<HashMap<String, Vec<u8>>>>,
    publishes: &Arc<Mutex<Vec<Published>>>,
    subscriptions: &Arc<Mutex<Vec<String>>>,
) -> bool {
    match packet_type {
        1 => {
            // CONNECT -> CONNACK
            let _ = socket.write_all(&[0x20, 0x02, 0x00, 0x00]).await;
        }
        8 => {
            // SUBSCRIBE -> SUBACK, then retained delivery for matching filters
            let qos = 0u8;
            let packet_id = u16::from_be_bytes([body[0], body[1]]);
            let mut index = 2;
            let mut filters = Vec::new();
            while index + 2 <= body.len() {
                let len = u16::from_be_bytes([body[index], body[index + 1]]) as usize;
                index += 2;
                if index + len > body.len() {
                    break;
                }
                filters.push(String::from_utf8_lossy(&body[index..index + len]).to_string());
                index += len + 1; // requested qos byte
            }
            subscriptions.lock().unwrap().extend(filters.clone());
            let mut suback = vec![
                0x90,
                0x03,
                packet_id.to_be_bytes()[0],
                packet_id.to_be_bytes()[1],
                qos,
            ];
            let _ = socket.write_all(&suback).await;
            let retained = retained.lock().unwrap().clone();
            for (topic, payload) in retained {
                let matches = filters.iter().any(|filter| {
                    filter.ends_with("/#") && topic.starts_with(filter.trim_end_matches('#'))
                        || filter == &topic
                });
                if !matches {
                    continue;
                }
                let mut packet = vec![0x31];
                let mut remaining = Vec::new();
                remaining.extend_from_slice(&(topic.len() as u16).to_be_bytes());
                remaining.extend_from_slice(topic.as_bytes());
                remaining.extend_from_slice(&payload);
                packet.extend_from_slice(&encode_length(remaining.len()));
                packet.extend_from_slice(&remaining);
                let _ = socket.write_all(&packet).await;
            }
        }
        3 => {
            // PUBLISH
            let retain = flags & 0x01 != 0;
            let qos = (flags >> 1) & 0x03;
            let topic_len = u16::from_be_bytes([body[0], body[1]]) as usize;
            let topic = String::from_utf8_lossy(&body[2..2 + topic_len]).to_string();
            let mut index = 2 + topic_len;
            let packet_id = if qos > 0 {
                let id = u16::from_be_bytes([body[index], body[index + 1]]);
                index += 2;
                Some(id)
            } else {
                None
            };
            let payload = body[index..].to_vec();
            publishes.lock().unwrap().push(Published {
                topic: topic.clone(),
                payload: payload.clone(),
                retain,
                qos,
            });
            if retain {
                retained.lock().unwrap().insert(topic, payload);
            }
            if let Some(packet_id) = packet_id {
                let _ = socket
                    .write_all(&[
                        0x40,
                        0x02,
                        packet_id.to_be_bytes()[0],
                        packet_id.to_be_bytes()[1],
                    ])
                    .await;
            }
        }
        12 => {
            let _ = socket.write_all(&[0xd0, 0x00]).await;
        }
        14 => return false,
        _ => {}
    }
    true
}
