// Apply a byte-exact NV snapshot captured by `hil_dump` to a real coordinator.
//
// This is a recovery tool, NOT commissioning: it never deletes the NIB and
// never forms a network. It only writes the NV items and extended table
// entries recorded in a known-good dump, then resets the adapter.
//
// Usage:
//   Z2M_SERIAL_PORT=/dev/ttyUSB0 \
//   Z2M_SNAPSHOT=/path/to/adapter-dump.json \
//   Z2M_SNAPSHOT_APPLY=1 \
//     cargo run -p zigbee-apiarist --example hil_apply_snapshot
//
// Contains network keys / link keys. Never commit the snapshot.

use serde_json::{json, Value};
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use zigbee_apiarist::adapter::z_stack::constants::common::{NvItemsIds, NvSystemIds};
use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;

fn hex_to_bytes(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).expect("hex"))
        .collect()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if std::env::var("Z2M_SNAPSHOT_APPLY").ok().as_deref() != Some("1") {
        eprintln!("refusing to write: set Z2M_SNAPSHOT_APPLY=1 to apply the snapshot");
        std::process::exit(1);
    }
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let snapshot_path = std::env::var("Z2M_SNAPSHOT").expect("set Z2M_SNAPSHOT");
    let snapshot: Value =
        serde_json::from_str(&std::fs::read_to_string(&snapshot_path).expect("read snapshot"))
            .expect("snapshot json");

    let mut znp = Znp::new(AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(115200),
        rtscts: Some(false),
    }));
    znp.open().await.expect("open coordinator");
    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");

    let items = snapshot
        .get("nvItems")
        .and_then(Value::as_object)
        .expect("nvItems");
    for (name, entry) in items {
        let Some(hex) = entry.get("hex").and_then(Value::as_str) else {
            continue;
        };
        let id = entry.get("id").and_then(Value::as_u64).expect("id") as u16;
        let data = hex_to_bytes(hex);
        nv.write_item(id, &data, 0, true)
            .await
            .unwrap_or_else(|error| panic!("write item {name} (id={id}) failed: {error}"));
        eprintln!("wrote item {name} (id={id}, len={})", data.len());
    }

    let table_ids = [
        ("APS_KEY_DATA_TABLE", 6u16),
        ("NWK_SEC_MATERIAL_TABLE", 7u16),
        ("TCLK_TABLE", 4u16),
    ];
    for (name, id) in table_ids {
        let Some(entries) = snapshot
            .get("extendedTables")
            .and_then(|tables| tables.get(name))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for entry in entries {
            let Some(hex) = entry.get("hex").and_then(Value::as_str) else {
                continue;
            };
            let sub_id = entry.get("subId").and_then(Value::as_u64).expect("subId") as u16;
            let data = hex_to_bytes(hex);
            nv.write_extended_table_entry(NvSystemIds::ZSTACK, id, sub_id, &data, 0, true)
                .await
                .unwrap_or_else(|error| {
                    panic!("write table {name} subId={sub_id} failed: {error}")
                });
        }
        eprintln!("wrote table {name} ({id}) entries={}", entries.len());
    }

    nv.znp
        .lock()
        .await
        .request(
            Subsystem::SYS,
            "resetReq",
            json!({"type": 0}),
            None,
            None,
            &[0],
        )
        .await
        .expect("reset adapter");
    eprintln!("snapshot applied; adapter reset");
    nv.znp
        .lock()
        .await
        .close()
        .await
        .expect("close coordinator");
}
