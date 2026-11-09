// HIL probe for a real Z-Stack coordinator (Dongle-P / CC2652P).
//
// Run only with a passed-through serial device:
//   Z2M_SERIAL_PORT=/dev/ttyACM0 cargo test -p zigbee-apiarist \
//       --test hil_znp -- --ignored --nocapture
//
// Without `Z2M_SERIAL_PORT` the test is skipped by `#[ignore]`.

use serde_json::json;
use std::sync::Mutex;

/// HIL tests share one physical coordinator; serialize them.
static HIL_LOCK: Mutex<()> = Mutex::new(());
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT and a real Z-Stack coordinator"]
async fn hil_version_request() {
    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT")
        .expect("set Z2M_SERIAL_PORT to the coordinator serial device");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);

    znp.open().await.expect("open coordinator");
    let reply = znp
        .request_with_reply(Subsystem::SYS, "version", json!({}))
        .await
        .expect("SYS version request");
    eprintln!("version payload: {}", reply.payload);
    znp.close().await.expect("close coordinator");

    assert!(
        reply.payload.get("product").is_some(),
        "unexpected version payload: {}",
        reply.payload
    );
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT and a real Z-Stack coordinator"]
async fn hil_nv_memory_init() {
    let _guard = HIL_LOCK.lock().unwrap();
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;

    let path = std::env::var("Z2M_SERIAL_PORT")
        .expect("set Z2M_SERIAL_PORT to the coordinator serial device");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");

    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");
    eprintln!("NV memory alignment: {:?}", nv.memory_alignment);
    assert!(nv.memory_alignment.is_some());

    nv.znp.close().await.expect("close coordinator");
}
