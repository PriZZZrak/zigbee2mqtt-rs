//! EVID-04 frontend/API candidate driver (harness only).
//!
//! Loads a database/config directory, starts an in-process MQTT broker and a
//! fake ZNP coordinator, then publishes the bridge MQTT API responses used by
//! the frontend: `bridge/state`, `bridge/info`, `bridge/devices` and the
//! `permit_join` response.
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::SinkExt;
use serde_json::{json, Value};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

use zigbee2mqtt_rs::bridge::Bridge;
use zigbee2mqtt_rs::config::Config;
use zigbee2mqtt_rs::coordinator::znp::frame::{FrameType, Subsystem, ZnpCodec, ZnpFrame};
use zigbee2mqtt_rs::coordinator::znp::test_handle_with_pump;
use zigbee2mqtt_rs::coordinator::znp::transport::spawn_test_transport;
use zigbee2mqtt_rs::coordinator::{CoordinatorHandle, CoordinatorInfo};
use zigbee2mqtt_rs::mqtt::fake_broker::FakeBroker;
use zigbee2mqtt_rs::mqtt::MqttBridge;
use zigbee2mqtt_rs::zigbee::IeeeAddr;

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-api-candidate: {message}");
    std::process::exit(2);
}

fn fake_coordinator() -> CoordinatorHandle {
    let (client, server) = tokio::io::duplex(8192);
    tokio::spawn(async move {
        let mut wire = Framed::new(client, ZnpCodec);
        while let Some(Ok(frame)) = wire.next().await {
            let response = ZnpFrame::new(FrameType::SRsp, frame.subsystem, frame.cmd1, vec![0x00]);
            if wire.send(response).await.is_err() {
                break;
            }
        }
    });
    let (transport, events) = spawn_test_transport(Framed::new(server, ZnpCodec));
    let (handle, event_rx) = test_handle_with_pump(transport, events);
    CoordinatorHandle {
        inner: Arc::new(tokio::sync::Mutex::new(handle)),
        events: event_rx,
        info: CoordinatorInfo {
            ieee_addr: IeeeAddr::from_hex("0x00124b00120144ae"),
            version: "1".into(),
            transport_rev: 2,
        },
    }
}

async fn wait_topic(broker: &FakeBroker, topic: &str) -> Value {
    for _ in 0..250 {
        if let Some(item) = broker
            .publishes()
            .into_iter()
            .find(|item| item.topic == topic)
        {
            return serde_json::from_slice(&item.payload)
                .unwrap_or_else(|error| fail(format!("{topic} payload is not JSON: {error}")));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    fail(format!(
        "no publish on {topic}; saw {:?}",
        broker.publishes()
    ))
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let mut data_dir: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--data" => {
                data_dir = Some(PathBuf::from(
                    args.get(index + 1)
                        .unwrap_or_else(|| fail("--data needs a directory")),
                ));
                index += 2;
            }
            other => fail(format!("unknown argument '{other}'")),
        }
    }
    let data_dir = data_dir.unwrap_or_else(|| fail("--data is required"));
    let config_path = data_dir.join("configuration.yaml");
    let mut config =
        Config::load(&config_path).unwrap_or_else(|error| fail(format!("config: {error}")));

    let broker = FakeBroker::start().await;
    config.mqtt.server = "127.0.0.1".into();
    config.mqtt.port = broker.port;
    config.mqtt.client_id = format!("z2m-api-diff-{}", std::process::id());
    let base_topic = config.mqtt.base_topic.clone();
    let bridge = Bridge::new(config.clone(), config_path);
    bridge.import_for_diff();

    let (mqtt, _commands) = MqttBridge::connect(&config.mqtt)
        .await
        .unwrap_or_else(|error| fail(format!("mqtt connect: {error}")));
    let coord = fake_coordinator();

    let _ = mqtt.publish_bridge_state(true).await;
    bridge.publish_info_for_diff(&mqtt, &coord).await;
    bridge.publish_devices_for_diff(&mqtt).await;
    let permit_join_response = bridge.permit_join_for_diff(&mqtt, &coord, 120).await;
    let state = wait_topic(&broker, &format!("{base_topic}/bridge/state")).await;
    let info = wait_topic(&broker, &format!("{base_topic}/bridge/info")).await;
    let devices = wait_topic(&broker, &format!("{base_topic}/bridge/devices")).await;
    println!(
        "{}",
        serde_json::to_string(&json!({
            "bridge_state": state,
            "bridge_info": info,
            "bridge_devices": devices,
            "permit_join_response": permit_join_response,
        }))
        .unwrap_or_else(|_| "{}".into())
    );
}
