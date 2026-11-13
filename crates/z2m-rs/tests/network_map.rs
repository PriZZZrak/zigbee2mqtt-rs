// Original: .upstream/zigbee2mqtt/test/extensions/networkMap.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::extension::network_map::NetworkMap;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    state_path: std::path::PathBuf,
}

fn harness() -> Harness {
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state_path = std::env::temp_dir().join(format!(
        "z2m-netmap-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    Harness {
        context: ExtensionContext {
            zigbee,
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 12,
        },
        mqtt,
        event_bus,
        state_path,
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.state_path);
    }
}

fn topology() -> Value {
    json!({
        "nodes": [
            {
                "ieeeAddr": "0x00124b0024c16b95",
                "friendlyName": "Coordinator",
                "type": "Coordinator",
                "networkAddress": 0,
                "lastSeen": null,
                "failed": null,
            },
            {
                "ieeeAddr": "0x0017880104e45519",
                "friendlyName": "lamp",
                "type": "Router",
                "networkAddress": 4660,
                "lastSeen": 1700000000000i64,
                "failed": ["lqi"],
                "definition": {
                    "model": "TEST",
                    "vendor": "Acme",
                    "description": "Test device",
                },
            },
            {
                "ieeeAddr": "0x0017880104e45520",
                "friendlyName": "sensor",
                "type": "EndDevice",
                "networkAddress": 22136,
                "lastSeen": 1700000000000i64,
                "failed": [],
            }
        ],
        "links": [
            {
                "source": {"ieeeAddr": "0x0017880104e45520", "networkAddress": 22136},
                "target": {"ieeeAddr": "0x0017880104e45519", "networkAddress": 4660},
                "linkquality": 42,
                "lqi": 42,
                "sourceIeeeAddr": "0x0017880104e45520",
                "targetIeeeAddr": "0x0017880104e45519",
                "routes": [],
            },
            {
                "source": {"ieeeAddr": "0x0017880104e45519", "networkAddress": 4660},
                "target": {"ieeeAddr": "0x00124b0024c16b95", "networkAddress": 0},
                "linkquality": 120,
                "lqi": 120,
                "sourceIeeeAddr": "0x0017880104e45519",
                "targetIeeeAddr": "0x00124b0024c16b95",
                "routes": [{"destinationAddress": 22136}],
            }
        ]
    })
}

#[test]
fn graphviz_renders_nodes_edges_and_styles() {
    let harness = harness();
    let map = NetworkMap::new(harness.context.clone());
    let text = map.graphviz(&topology());

    assert!(text.starts_with("digraph G {\nnode[shape=record];\n"));
    assert!(text.contains("label=\"{Coordinator|0x00124b0024c16b95 (0x0000)"));
    assert!(text.contains("style=\"bold, filled\""));
    assert!(text
        .contains("label=\"{lamp|0x0017880104e45519 (0x1234)failed: lqi|Acme Test device (TEST)"));
    assert!(text.contains("style=\"rounded, filled\""));
    assert!(text.contains("label=\"{sensor|0x0017880104e45520 (0x5678)"));
    assert!(text.contains("style=\"rounded, dashed, filled\""));
    assert!(text.contains("penwidth=2, weight=1"));
    assert!(text.contains("label=\"120 (routes: 0x5678)\""));
    assert!(text.contains("penwidth=1, weight=0"));
    assert!(text.ends_with('}'));
}

#[test]
fn plantuml_sorts_cards_and_writes_links() {
    let harness = harness();
    let map = NetworkMap::new(harness.context.clone());
    let text = map.plantuml(&topology());

    let coordinator = text.find("card 0x00124b0024c16b95 [").unwrap();
    let lamp = text.find("card 0x0017880104e45519 [").unwrap();
    let sensor = text.find("card 0x0017880104e45520 [").unwrap();
    assert!(coordinator < lamp && lamp < sensor);
    assert!(text.contains("Acme Test device (TEST)"));
    assert!(text.contains("0x0017880104e45520 --> 0x0017880104e45519: 42"));
    assert!(text.contains("0x0017880104e45519 --> 0x00124b0024c16b95: 120"));
    assert!(text.ends_with("@enduml"));
}

#[test]
fn raw_returns_topology() {
    let harness = harness();
    let map = NetworkMap::new(harness.context.clone());
    assert_eq!(map.raw(&topology()), topology());
}

#[tokio::test]
async fn unsupported_type_returns_error() {
    let harness = harness();
    let map = NetworkMap::new(harness.context.clone());
    map.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/networkmap",
        "message": "json",
    }));

    for _ in 0..100 {
        if !harness.mqtt.lock().await.attempted.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mqtt = harness.mqtt.lock().await;
    let response: Value = serde_json::from_str(&mqqtt_payload(&mqtt)).unwrap();
    assert_eq!(response["status"], "error");
    assert_eq!(
        response["error"],
        "Type 'json' not supported, allowed are: raw,graphviz,plantuml"
    );
}

fn mqqtt_payload(mqtt: &Mqtt) -> String {
    mqtt.attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/response/networkmap")
        .map(|(_, payload, _)| payload.clone())
        .unwrap()
}
