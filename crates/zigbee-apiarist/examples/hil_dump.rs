// HIL dump for a real Z-Stack coordinator.
//
// Usage:
//   Z2M_SERIAL_PORT=/dev/ttyUSB0 cargo run -p zigbee-apiarist \
//       --example hil_dump > /tmp/opencode/adapter-dump.json
//
// Contains network keys / link keys. Do not commit the output.

use std::sync::Arc;

use serde_json::{json, Value};
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use zigbee_apiarist::adapter::z_stack::constants::common::{NvItemsIds, NvSystemIds};
use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::utils::logger::{set_logger, Logger};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    struct StderrLogger;
    impl Logger for StderrLogger {
        fn debug(&self, message: &str, namespace: &str) {
            eprintln!("[{namespace}] {message}");
        }
        fn info(&self, message: &str, namespace: &str) {
            eprintln!("[{namespace}] {message}");
        }
        fn warning(&self, message: &str, namespace: &str) {
            eprintln!("[{namespace}] {message}");
        }
        fn error(&self, message: &str, namespace: &str) {
            eprintln!("[{namespace}] {message}");
        }
    }
    set_logger(Arc::new(StderrLogger));

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
        .request_with_reply(Subsystem::SYS, "version", json!({}))
        .await
        .expect("version");
    let ext_addr = znp
        .request_with_reply(Subsystem::SYS, "getExtAddr", json!({}))
        .await
        .expect("getExtAddr");
    let device_info = znp
        .request_with_reply(Subsystem::UTIL, "getDeviceInfo", json!({"param": 0}))
        .await
        .expect("getDeviceInfo");
    let ext_nwk_info = znp
        .request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
        .await
        .expect("extNwkInfo");

    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");

    let items = [
        ("EXTADDR", NvItemsIds::EXTADDR),
        ("STARTUP_OPTION", NvItemsIds::STARTUP_OPTION),
        ("NIB", NvItemsIds::NIB),
        ("EXTENDED_PAN_ID", NvItemsIds::EXTENDED_PAN_ID),
        ("NWK_ACTIVE_KEY_INFO", NvItemsIds::NWK_ACTIVE_KEY_INFO),
        ("NWK_ALTERN_KEY_INFO", NvItemsIds::NWK_ALTERN_KEY_INFO),
        ("APS_USE_EXT_PANID", NvItemsIds::APS_USE_EXT_PANID),
        (
            "ZNP_HAS_CONFIGURED_ZSTACK3",
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3,
        ),
        ("PRECFGKEY", NvItemsIds::PRECFGKEY),
        ("PRECFGKEYS_ENABLE", NvItemsIds::PRECFGKEYS_ENABLE),
        ("NWKKEY", NvItemsIds::NWKKEY),
        ("PANID", NvItemsIds::PANID),
        ("CHANLIST", NvItemsIds::CHANLIST),
        ("LOGICAL_TYPE", NvItemsIds::LOGICAL_TYPE),
        ("ZDO_DIRECT_CB", NvItemsIds::ZDO_DIRECT_CB),
        (
            "ZNP_HAS_CONFIGURED_ZSTACK1",
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1,
        ),
    ];

    let mut nv_items = serde_json::Map::new();
    for (name, id) in items {
        match nv.read_item(id, 0).await {
            Ok(Some(bytes)) => {
                nv_items.insert(
                    name.to_string(),
                    json!({"id": id, "len": bytes.len(), "hex": hex(&bytes)}),
                );
            }
            Ok(None) => {
                nv_items.insert(name.to_string(), json!({"id": id, "missing": true}));
            }
            Err(error) => {
                nv_items.insert(name.to_string(), json!({"id": id, "error": error}));
            }
        }
    }

    let mut extended = serde_json::Map::new();
    for (name, id) in [
        ("APS_KEY_DATA_TABLE", 6u16),
        ("NWK_SEC_MATERIAL_TABLE", 7u16),
        ("TCLK_TABLE", 4u16),
    ] {
        let mut entries = Vec::new();
        for sub_id in 0..64u16 {
            match nv
                .read_extended_table_entry(NvSystemIds::ZSTACK, id, sub_id, 0)
                .await
            {
                Ok(Some(bytes)) => entries.push(json!({
                    "subId": sub_id,
                    "len": bytes.len(),
                    "hex": hex(&bytes),
                })),
                Ok(None) => break,
                Err(error) => {
                    entries.push(json!({"subId": sub_id, "error": error}));
                    break;
                }
            }
        }
        extended.insert(name.to_string(), Value::Array(entries));
    }

    let previous_alignment = nv.memory_alignment;
    nv.znp
        .lock()
        .await
        .close()
        .await
        .expect("close coordinator");

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "version": version.payload,
            "extAddr": ext_addr.payload,
            "deviceInfo": device_info.payload,
            "extNwkInfo": ext_nwk_info.payload,
            "memoryAlignment": previous_alignment.map(|alignment| alignment.as_str()),
            "nvItems": Value::Object(nv_items),
            "extendedTables": Value::Object(extended),
        }))
        .expect("json")
    );
}
