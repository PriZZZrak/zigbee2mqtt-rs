// Restores ZCD_NV_STARTUP_OPTION to the normal `0x00` value.
//
// The HIL NV probe previously wrote/deleted this item on the real coordinator.
// Run once before returning the adapter to production:
//   Z2M_SERIAL_PORT=/dev/ttyUSB0 cargo run -p zigbee-apiarist \
//       --example hil_restore_startup

use serde_json::json;
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let mut znp = Znp::new(AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    }));
    znp.open().await.expect("open coordinator");
    let version = znp
        .request_with_reply(
            zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
            "version",
            json!({}),
        )
        .await
        .expect("version");
    eprintln!("version: {}", version.payload);

    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");
    nv.write_item(NvItemsIds::STARTUP_OPTION, &[0x00], 0, true)
        .await
        .expect("write STARTUP_OPTION");
    let restored = nv
        .read_item(NvItemsIds::STARTUP_OPTION, 0)
        .await
        .expect("read STARTUP_OPTION");
    eprintln!("STARTUP_OPTION: {restored:?}");
    nv.znp.close().await.expect("close coordinator");
}
