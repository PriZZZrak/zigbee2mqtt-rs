// Original: .upstream/zigbee2mqtt/test/extensions/onEvent.test.ts

use serde_json::json;
use z2m_rs::extension::on_event::{OnEvent, OnEventCall};

const LIVOLO: &str = "0x0017880104e45560";

fn options() -> serde_json::Value {
    json!({"retain": false, "friendly_name": "livolo"})
}

fn setup() -> OnEvent {
    OnEvent::new(LIVOLO, options())
}

#[tokio::test]
async fn starts_stops() {
    let mut on_event = setup();
    on_event.start().await;

    assert_eq!(on_event.global_calls, 1);
    assert_eq!(
        on_event.calls,
        vec![OnEventCall::Start {
            device: LIVOLO.to_string(),
            options: options(),
            state: json!({}),
        }]
    );

    on_event.stop().await;

    assert_eq!(on_event.global_calls, 2);
    assert_eq!(
        on_event.calls,
        vec![
            OnEventCall::Start {
                device: LIVOLO.to_string(),
                options: options(),
                state: json!({}),
            },
            OnEventCall::Stop {
                ieee_addr: LIVOLO.to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn calls_on_device_events() {
    let mut on_event = setup();
    on_event.start().await;
    on_event.device_announce().await;

    // Should always call with 'start' event first
    assert_eq!(
        on_event.calls[0],
        OnEventCall::Start {
            device: LIVOLO.to_string(),
            options: options(),
            state: json!({}),
        }
    );
    assert_eq!(
        on_event.calls[1],
        OnEventCall::DeviceAnnounce {
            device: LIVOLO.to_string(),
            options: options(),
            state: json!({}),
        }
    );

    // Check `deviceExposesChanged()`
    let OnEventCall::Start { .. } = on_event.calls[0] else {
        panic!("first call must be start");
    };
    on_event.device_exposes_changed(0);
    assert_eq!(on_event.exposes_changed, vec![LIVOLO.to_string()]);

    // Call `stop` when device leaves
    on_event.device_leave(LIVOLO).await;
    assert_eq!(
        on_event.calls[2],
        OnEventCall::Stop {
            ieee_addr: LIVOLO.to_string(),
        }
    );

    // Call `stop` when device is removed
    on_event.entity_removed(LIVOLO).await;
    assert_eq!(
        on_event.calls[3],
        OnEventCall::Stop {
            ieee_addr: LIVOLO.to_string(),
        }
    );

    // Device interview, should call with 'start' first as 'stop' was called
    on_event.device_interview("started").await;
    assert_eq!(on_event.calls.len(), 6);
    assert_eq!(
        on_event.calls[4],
        OnEventCall::Start {
            device: LIVOLO.to_string(),
            options: options(),
            state: json!({}),
        }
    );
    assert_eq!(
        on_event.calls[5],
        OnEventCall::DeviceInterview {
            device: LIVOLO.to_string(),
            options: options(),
            state: json!({}),
            status: "started".to_string(),
        }
    );
}

#[tokio::test]
async fn does_not_block_startup_on_failure() {
    let mut on_event = setup();
    on_event.fail_device_start = true;

    // Upstream: the per-device `start` handler sleeps 10s and then throws; the
    // extension must still be added (not block startup).
    on_event.start().await;

    assert!(on_event.started);
}
