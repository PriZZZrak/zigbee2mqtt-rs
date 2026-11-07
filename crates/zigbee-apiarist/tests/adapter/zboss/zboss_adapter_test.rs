// Original: .upstream/zigbee-herdsman/test/adapter/zboss/zbossAdapter.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - ZBOSS Adapter
//   - reports the formed network after forming on a blank NCP
//   - reports the new network after a factory reset and reform

use zigbee_apiarist::adapter::zboss::adapter::zboss_adapter::{
    NetworkParameters, ZBOSSAdapter, ZBOSSAdapterConfig, ZBOSSAdapterOptions,
    ZBOSSAdapterStartupOptions,
};

const EXT_PAN_ID: [u8; 8] = [0xdd; 8];
const NETWORK_KEY: [u8; 16] = [
    0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f, 0x00, 0x02, 0x04, 0x06, 0x08, 0x0a, 0x0c, 0x0d,
];

fn create_adapter(pan_id: u16) -> ZBOSSAdapter {
    ZBOSSAdapter::new(
        ZBOSSAdapterOptions {
            pan_id,
            extended_pan_id: EXT_PAN_ID,
            channel_list: vec![15],
            network_key: NETWORK_KEY.to_vec(),
        },
        ZBOSSAdapterConfig {
            path: "/dev/serial/by-id/mock-adapter".to_string(),
            adapter: "zboss".to_string(),
        },
        "backup.json",
        ZBOSSAdapterStartupOptions { disable_led: false },
    )
}

#[tokio::test]
async fn reports_the_formed_network_after_forming_on_a_blank_ncp() {
    let mut adapter = create_adapter(0x1872);
    assert_eq!(adapter.start().await, "reset");
    assert_eq!(
        adapter.get_network_parameters().await,
        NetworkParameters {
            pan_id: 0x1872,
            extended_pan_id: "0xdddddddddddddddd".to_string(),
            channel: 15,
            nwk_update_id: 0,
        }
    );
}

#[tokio::test]
async fn reports_the_new_network_after_a_factory_reset_and_reform() {
    let mut adapter = create_adapter(0x1873);
    assert_eq!(adapter.start().await, "reset");
    assert_eq!(
        adapter.get_network_parameters().await,
        NetworkParameters {
            pan_id: 0x1873,
            extended_pan_id: "0xdddddddddddddddd".to_string(),
            channel: 15,
            nwk_update_id: 0,
        }
    );
}
