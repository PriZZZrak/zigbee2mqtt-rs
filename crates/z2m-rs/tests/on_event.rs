// Original: .upstream/zigbee2mqtt/lib/extension/onEvent.ts (upstream has no
// dedicated test; controller.test.ts exercises OnEvent through startup).
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::extension::Extension;
use z2m_rs::extension::on_event::OnEvent;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{Definition, OnEventEvent};

const IEEE: &str = "0x0017880104e45519";

#[tokio::test]
async fn dispatches_definition_on_event_and_stops() {
    let _guard = serial();
    let database_path = std::env::temp_dir().join(format!(
        "z2m-onevent-db-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    inject_database(Arc::new(Mutex::new(Database::open(&database_path))));
    ZhDevice::reset_cache();
    ZhGroup::reset_cache();

    ZhDevice::create(
        DeviceType::Coordinator,
        "0x00124b0024c16b95",
        0,
        None,
        None,
        None,
        None,
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let device = ZhDevice::create(
        DeviceType::Router,
        IEEE,
        1234,
        None,
        Some("Test".to_string()),
        None,
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();

    let controller = Arc::new(
        Controller::new(ControllerOptions {
            network: NetworkOptions {
                pan_id: 0x1a62,
                extended_pan_id: vec![0xdd; 8],
                channel_list: vec![11],
                network_key: vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
                network_key_distribute: true,
                has_default_extended_pan_id: false,
            },
            transport: json!({}),
            database_path: database_path.display().to_string(),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        })
        .unwrap(),
    );

    let settings = Arc::new(Mutex::new(Settings::from_config(
        json!({"mqtt": {"server": "localhost"}, "devices": {IEEE: {"friendly_name": "lamp"}}}),
    )));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    let _ = zigbee.resolve_device(IEEE);
    let state_path = std::env::temp_dir().join(format!("z2m-onevent-state-{}", std::process::id()));
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_entity, _payload, _reason| Box::pin(async {}));

    let context = z2m_rs::extension::extension::ExtensionContext {
        zigbee: zigbee.clone(),
        mqtt,
        state,
        publish_entity_state: publish,
        event_bus: event_bus.clone(),
        logger,
        enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
        restart: Arc::new(|| Box::pin(async {})),
        add_extension: Arc::new(|_| Box::pin(async {})),
        key: 7,
    };

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition
        .on_event
        .push(Arc::new(move |event: OnEventEvent| {
            let sink = sink.clone();
            Box::pin(async move {
                sink.lock().unwrap().push(event.event_type().to_string());
                Ok(())
            })
        }));
    zigbee.converters().lock().unwrap().register(definition);

    let extension = OnEvent::new(context);
    extension.clone().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(events.lock().unwrap().contains(&"start".to_string()));

    event_bus
        .lock()
        .unwrap()
        .emit_device_joined(json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}}));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(events.lock().unwrap().contains(&"deviceJoined".to_string()));

    extension.clone().stop().await.unwrap();
    assert!(events.lock().unwrap().contains(&"stop".to_string()));

    let _ = std::fs::remove_file(&database_path);
    let _ = std::fs::remove_file(&state_path);
}
