// Original: .upstream/zigbee-herdsman/test/adapter/adapter.test.ts

use serde_json::{json, Value};
use zigbee_apiarist::adapter::adapter_discovery::{
    find_all_devices, DiscoveredAdapter, DiscoveryContext, MdnsService,
};
use zigbee_apiarist::adapter::serial_port::SerialPortInfo;
use zigbee_apiarist::adapter::utils::parse_tcp_path;
use zigbee_apiarist::adapter::{Adapter, AdapterInstance, AdapterKind};

// ------------------------- upstream `mockAdapters` -------------------------

fn deconz_conbee_ii() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("1cf1".to_string()),
        product_id: Some("0030".to_string()),
        manufacturer: Some("dresden elektronik ingenieurtechnik GmbH".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-dresden_elektronik_ingenieurtechnik_GmbH_ConBee_II_DE2132111-if00")
    }
}

fn ember_zbdongle_e() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("1A86".to_string()),
        product_id: Some("55d4".to_string()),
        manufacturer: Some("ITEAD".to_string()),
        ..SerialPortInfo::new(
            "/dev/serial/by-id/usb-ITEAD_SONOFF_Zigbee_3.0_USB_Dongle_Plus_V2_20240122184111-if00",
        )
    }
}

fn ember_zbdongle_e_cp() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("ITEAD".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-Itead_Sonoff_Zigbee_3.0_USB_Dongle_Plus_V2_a6ee897e4d1fef11aa004ad0639e525b-if00-port0")
    }
}

fn ember_zbdongle_e_win() -> SerialPortInfo {
    SerialPortInfo {
        path: "COM6".to_string(),
        vendor_id: Some("10C4".to_string()),
        product_id: Some("EA60".to_string()),
        manufacturer: Some("Silicon Labs".to_string()),
        pnp_id: Some("USB\\VID_10C4&PID_EA60\\F881C5D59D38E123456C317AF12345E5".to_string()),
        friendly_name: Some("Silicon Labs CP210x USB to UART Bridge (COM6)".to_string()),
        ..SerialPortInfo::default()
    }
}

fn ember_skyconnect() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("Nabu Casa".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-Nabu_Casa_SkyConnect_v1.0_3abe54797c91ed118fc3cad13b20a111-if00-port0")
    }
}

fn zstack_cc2538() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("0451".to_string()),
        product_id: Some("16C8".to_string()),
        manufacturer: Some("Texas Instruments".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-Texas_Instruments_CC2538_USB_CDC-if00")
    }
}

fn zstack_zbdongle_p() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("ITEAD".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-ITead_Sonoff_Zigbee_3.0_USB_Dongle_Plus_b8b49abd27a6ed11a280eba32981d111-if00-port0")
    }
}

fn zstack_smlight_slzb_06p10() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("SMLIGHT".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-SMLIGHT_SMLIGHT_SLZB-06p10_40df2f3e3977ed11b142f6fafdf7b791-if00-port0")
    }
}

fn zstack_smlight_slzb_07() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("SMLIGHT".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-SMLIGHT_SMLIGHT_SLZB-07_be9faa0786e1ea11bd68dc2d9a583111-if00-port0")
    }
}

fn zboss_nordic() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("2fe3".to_string()),
        product_id: Some("0100".to_string()),
        manufacturer: Some("ZEPHYR".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-ZEPHYR_Zigbee_NCP_54ACCFAFA6DADC49-if00")
    }
}

fn zigate_plusv2() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("0403".to_string()),
        product_id: Some("6015".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-FTDI_ZiGate_ZIGATE+-if00-port0")
    }
}

fn zbt_2() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("303a".to_string()),
        product_id: Some("4001".to_string()),
        manufacturer: Some("Nabu Casa".to_string()),
        ..SerialPortInfo::new("/dev/serial/by-id/usb-Nabu_Casa_ZBT-2_10B41DE58D6C-if00")
    }
}

fn zbt_1_pnpid() -> SerialPortInfo {
    SerialPortInfo {
        vendor_id: Some("10c4".to_string()),
        product_id: Some("ea60".to_string()),
        manufacturer: Some("Nabu Casa".to_string()),
        pnp_id: Some(
            "usb-Nabu_Casa_SkyConnect_v1.0_92390c41b6d8ed11a3436b6142c613ac-if00-port0".to_string(),
        ),
        ..SerialPortInfo::new("/dev/ttyUSB0")
    }
}

fn zwa_2_conflict() -> SerialPortInfo {
    SerialPortInfo {
        path: "/dev/ttyACM0".to_string(),
        vendor_id: Some("303a".to_string()),
        product_id: Some("4001".to_string()),
        manufacturer: Some("Nabu Casa".to_string()),
        pnp_id: Some("usb-Nabu_Casa_ZWA-2_81B53EF0C8EC-if00".to_string()),
        ..SerialPortInfo::default()
    }
}

// ------------------------------- helpers ----------------------------------

fn context(ports: Result<Vec<SerialPortInfo>, &str>) -> DiscoveryContext {
    DiscoveryContext {
        serial_ports: ports.map_err(str::to_string),
        ..DiscoveryContext::default()
    }
}

fn context_with(ports: Vec<SerialPortInfo>) -> DiscoveryContext {
    context(Ok(ports))
}

fn discovered(
    name: &str,
    path: &str,
    adapter: Option<AdapterKind>,
    baud_rate: Option<u32>,
    rtscts: Option<bool>,
) -> DiscoveredAdapter {
    DiscoveredAdapter {
        name: name.to_string(),
        path: path.to_string(),
        adapter,
        baud_rate,
        rtscts,
    }
}

async fn create_with(context: &DiscoveryContext, serial: Value) -> Result<AdapterInstance, String> {
    Adapter::create_with_context(
        context,
        json!({"panID": 0x1a62, "channelList": [11]}),
        serial,
        "test.db.backup",
        json!({"disableLED": false}),
    )
    .await
}

async fn assert_detect(
    context: &DiscoveryContext,
    serial: Value,
    kind: AdapterKind,
    options: Value,
) {
    let adapter = create_with(context, serial).await.expect("adapter");
    assert_eq!(adapter.kind, kind);
    assert_eq!(adapter.serial_port_options, options);
}

// -------------------------------------------------------------------------

#[tokio::test]
async fn adapter() {
    let cases = [
        ("deconz", AdapterKind::Deconz),
        ("ember", AdapterKind::Ember),
        ("ezsp", AdapterKind::Ezsp),
        ("zstack", AdapterKind::ZStack),
        ("zboss", AdapterKind::ZBoss),
        ("zigate", AdapterKind::ZiGate),
        ("zoh", AdapterKind::ZoH),
    ];

    for (name, kind) in cases {
        let adapter = create_with(
            &DiscoveryContext::default(),
            json!({"path": "/dev/ttyUSB0", "adapter": name}),
        )
        .await
        .expect("adapter");

        assert_eq!(adapter.kind, kind);
    }
}

#[tokio::test]
async fn tcp_discovery() {
    for path in ["tcp://192168.1.321", "tcp://192168.1.321:INVALID"] {
        let result = create_with(
            &DiscoveryContext::default(),
            json!({"path": path, "adapter": "zstack"}),
        )
        .await;

        assert_eq!(
            result.unwrap_err(),
            "Invalid TCP path, expected format: tcp://<host>:<port>"
        );
    }
}

#[tokio::test]
async fn usb_discovery() {
    let mut discovery = context_with(vec![
        deconz_conbee_ii(),
        zstack_cc2538(),
        ember_zbdongle_e(),
    ]);

    let adapter = create_with(&discovery, json!({"path": zstack_cc2538().path}))
        .await
        .expect("adapter");

    assert_eq!(adapter.kind, AdapterKind::ZStack);
    assert_eq!(
        adapter.serial_port_options,
        json!({"path": zstack_cc2538().path, "adapter": "zstack"})
    );
    discovery.serial_ports = Ok(vec![]);
}

#[tokio::test]
async fn detects_sky_connect_zbt_1_when_zwa_2_present() {
    let context = context_with(vec![zwa_2_conflict(), zbt_1_pnpid()]);

    assert_detect(
        &context,
        json!({"baudRate": 115200}),
        AdapterKind::Ember,
        json!({
            "path": zbt_1_pnpid().path,
            "adapter": "ember",
            "baudRate": 115200,
            "rtscts": true,
        }),
    )
    .await;
}

#[tokio::test]
async fn detects_sky_connect_zbt_2_when_zwa_2_present() {
    let context = context_with(vec![zwa_2_conflict(), zbt_2()]);

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({
            "path": zbt_2().path,
            "adapter": "ember",
            "baudRate": 460800,
            "rtscts": true,
        }),
    )
    .await;
}

#[tokio::test]
async fn detects_each_adapter() {
    let cases = [
        (
            vec![deconz_conbee_ii()],
            json!({"baudRate": 57600}),
            AdapterKind::Deconz,
            json!({"path": deconz_conbee_ii().path, "adapter": "deconz", "baudRate": 57600}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"baudRate": 115200}),
            AdapterKind::Ember,
            json!({"path": ember_zbdongle_e().path, "adapter": "ember", "baudRate": 115200, "rtscts": false}),
        ),
        (
            vec![zstack_cc2538()],
            json!({}),
            AdapterKind::ZStack,
            json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
        ),
        (
            vec![zboss_nordic()],
            json!({}),
            AdapterKind::ZBoss,
            json!({"path": zboss_nordic().path, "adapter": "zboss"}),
        ),
        (
            vec![zigate_plusv2()],
            json!({}),
            AdapterKind::ZiGate,
            json!({"path": zigate_plusv2().path, "adapter": "zigate"}),
        ),
    ];

    for (ports, serial, kind, options) in cases {
        assert_detect(&context_with(ports), serial, kind, options).await;
    }
}

#[tokio::test]
async fn detects_on_windows_with_manufacturer_present() {
    let mut context = context_with(vec![SerialPortInfo {
        path: "COM3".to_string(),
        vendor_id: Some("1A86".to_string()),
        product_id: Some("55D4".to_string()),
        manufacturer: Some("ITEAD".to_string()),
        pnp_id: Some("USB\\VID_1A86&PID_55D4\\54DD002111".to_string()),
        friendly_name: Some("USB-Enhanced-SERIAL CH9102 (COM3)".to_string()),
        ..SerialPortInfo::default()
    }]);
    context.platform = "win32".to_string();

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": "COM3", "adapter": "ember", "rtscts": false}),
    )
    .await;

    context.serial_ports = Ok(vec![ember_zbdongle_e_win()]);
    assert_detect(
        &context,
        json!({"adapter": "ember", "path": "COM6"}),
        AdapterKind::Ember,
        json!({"path": ember_zbdongle_e_win().path, "adapter": "ember"}),
    )
    .await;

    context.serial_ports = Ok(vec![ember_zbdongle_e_win()]);
    assert_detect(
        &context,
        json!({"adapter": "ezsp", "path": "COM6"}),
        AdapterKind::Ezsp,
        json!({"path": ember_zbdongle_e_win().path, "adapter": "ezsp"}),
    )
    .await;
}

#[tokio::test]
async fn detects_on_windows_without_manufacturer_present() {
    let mut context = context_with(vec![SerialPortInfo {
        path: "COM3".to_string(),
        vendor_id: Some("1A86".to_string()),
        product_id: Some("55D4".to_string()),
        manufacturer: Some("wch.cn".to_string()),
        pnp_id: Some("USB\\VID_1A86&PID_55D4\\54DD002111".to_string()),
        friendly_name: Some("USB-Enhanced-SERIAL CH9102 (COM3)".to_string()),
        ..SerialPortInfo::default()
    }]);
    context.platform = "win32".to_string();

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": "COM3", "adapter": "ember", "rtscts": false}),
    )
    .await;
}

#[tokio::test]
async fn detects_with_conflict_vendor_product_ids() {
    let mut context = context_with(vec![SerialPortInfo {
        manufacturer: None,
        ..ember_skyconnect()
    }]);

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": ember_skyconnect().path, "adapter": "ember", "rtscts": true}),
    )
    .await;

    context.serial_ports = Ok(vec![SerialPortInfo {
        path: "/dev/ttyACM0".to_string(),
        ..zstack_zbdongle_p()
    }]);
    assert_eq!(
        create_with(&context, json!({})).await.unwrap_err(),
        "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration."
    );

    context.serial_ports = Ok(vec![SerialPortInfo {
        path: "/dev/ttyACM0".to_string(),
        pnp_id: Some(zstack_zbdongle_p().path.replace("/dev/serial/by-id/", "")),
        ..zstack_zbdongle_p()
    }]);
    assert_detect(
        &context,
        json!({}),
        AdapterKind::ZStack,
        json!({"path": "/dev/ttyACM0", "adapter": "zstack"}),
    )
    .await;

    context.serial_ports = Ok(vec![zstack_zbdongle_p()]);
    assert_detect(
        &context,
        json!({}),
        AdapterKind::ZStack,
        json!({"path": zstack_zbdongle_p().path, "adapter": "zstack"}),
    )
    .await;

    context.serial_ports = Ok(vec![ember_zbdongle_e_cp()]);
    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": ember_zbdongle_e_cp().path, "adapter": "ember", "rtscts": false}),
    )
    .await;

    context.serial_ports = Ok(vec![zstack_smlight_slzb_06p10()]);
    assert_detect(
        &context,
        json!({}),
        AdapterKind::ZStack,
        json!({"path": zstack_smlight_slzb_06p10().path, "adapter": "zstack"}),
    )
    .await;

    context.serial_ports = Ok(vec![zstack_smlight_slzb_07()]);
    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": zstack_smlight_slzb_07().path, "adapter": "ember", "rtscts": true}),
    )
    .await;
}

#[tokio::test]
async fn detects_with_multiple_adapters_connected() {
    let context = context_with(vec![
        deconz_conbee_ii(),
        zstack_cc2538(),
        ember_zbdongle_e(),
    ]);

    assert_detect(
        &context,
        json!({"adapter": "zstack", "path": zstack_cc2538().path}),
        AdapterKind::ZStack,
        json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn detects_with_pnp_id_instead_of_path() {
    let context = context_with(vec![SerialPortInfo {
        path: "/dev/ttyUSB0".to_string(),
        pnp_id: Some("usb-ZEPHYR_Zigbee_NCP_54ACCFAFA6DADC49-if00".to_string()),
        ..zboss_nordic()
    }]);

    assert_detect(
        &context,
        json!({"adapter": "zboss", "path": "/dev/ttyUSB0"}),
        AdapterKind::ZBoss,
        json!({"path": "/dev/ttyUSB0", "adapter": "zboss"}),
    )
    .await;
}

#[tokio::test]
async fn errors_inside_mdns() {
    let context = DiscoveryContext {
        mdns_start_error: Some("failed".to_string()),
        ..DiscoveryContext::default()
    };

    let result = create_with(&context, json!({"path": "mdns://mdns_test_device"})).await;

    assert_eq!(
        result.unwrap_err(),
        "Failed to start mdns discovery: Error: failed"
    );
}

#[tokio::test]
async fn falls_back_to_host_if_no_addresses() {
    let context = DiscoveryContext {
        mdns_services: vec![MdnsService {
            name: Some("Mock Adapter".to_string()),
            radio_type: Some("zstack".to_string()),
            host: "mock_adapter.local".to_string(),
            port: "1122".to_string(),
            addresses: Vec::new(),
        }],
        ..DiscoveryContext::default()
    };

    assert_detect(
        &context,
        json!({"path": "mdns://zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://mock_adapter.local:1122", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn finds_all_devices() {
    let mut context = context_with(vec![
        SerialPortInfo {
            pnp_id: Some("deconz conbee ii".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            ..deconz_conbee_ii()
        },
        SerialPortInfo {
            pnp_id: Some("zbdongle-e".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            ..ember_zbdongle_e()
        },
        SerialPortInfo {
            pnp_id: Some("cc2538".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            ..zstack_cc2538()
        },
        SerialPortInfo {
            pnp_id: Some("nordic".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            ..zboss_nordic()
        },
        SerialPortInfo {
            pnp_id: Some("zigate-plus-v2".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            manufacturer: Some(String::new()),
            ..zigate_plusv2()
        },
        SerialPortInfo {
            pnp_id: Some("zbt-2".to_string()),
            serial_number: Some(String::new()),
            location_id: Some(String::new()),
            ..zbt_2()
        },
    ]);

    let expected = vec![
        discovered(
            "zigate-plus-v2 ()",
            &zigate_plusv2().path,
            Some(AdapterKind::ZiGate),
            None,
            None,
        ),
        discovered(
            "zbdongle-e (ITEAD)",
            &ember_zbdongle_e().path,
            Some(AdapterKind::Ember),
            None,
            Some(false),
        ),
        discovered(
            "zbt-2 (Nabu Casa)",
            &zbt_2().path,
            Some(AdapterKind::Ember),
            Some(460800),
            Some(true),
        ),
        discovered(
            "cc2538 (Texas Instruments)",
            &zstack_cc2538().path,
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
        discovered(
            "nordic (ZEPHYR)",
            &zboss_nordic().path,
            Some(AdapterKind::ZBoss),
            None,
            None,
        ),
        discovered(
            "deconz conbee ii (dresden elektronik ingenieurtechnik GmbH)",
            &deconz_conbee_ii().path,
            Some(AdapterKind::Deconz),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::Ember),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
    ];

    assert_eq!(find_all_devices(&mut context), expected);
    assert_eq!(context.bonjour_destroyed, 1);
}

#[tokio::test]
async fn finds_all_devices_with_quirks() {
    let mut context = context_with(vec![SerialPortInfo {
        friendly_name: Some("silicon labs cp210x".to_string()),
        ..ember_zbdongle_e_cp()
    }]);
    context.platform = "win32".to_string();
    context.mdns_services = vec![
        MdnsService {
            name: Some("Mock Adapter".to_string()),
            radio_type: Some("zstack".to_string()),
            host: "mock_adapter.local".to_string(),
            port: "1122".to_string(),
            addresses: Vec::new(),
        },
        MdnsService {
            name: None,
            radio_type: Some("ezsp".to_string()),
            host: "mock_adapter.local".to_string(),
            port: "1122".to_string(),
            addresses: vec!["192.168.1.123".to_string()],
        },
        MdnsService::new("znp"),
    ];

    let expected = vec![
        discovered(
            "silicon labs cp210x (ITEAD)",
            &ember_zbdongle_e_cp().path,
            None,
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://mock_adapter.local:1122)",
            "tcp://mock_adapter.local:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
        discovered(
            "Unknown (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::Ember),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
    ];

    assert_eq!(find_all_devices(&mut context), expected);
    assert_eq!(context.bonjour_destroyed, 1);
}

#[tokio::test]
async fn for_zstack_as_znp() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "mdns://znp"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://192.168.1.123:1122", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn given_invalid_path() {
    let result = create_with(&DiscoveryContext::default(), json!({"path": "mdns://"})).await;

    assert_eq!(
        result.unwrap_err(),
        "No mdns device specified. You must specify the coordinator mdns service type after mdns://, e.g. mdns://my-adapter"
    );
}

#[tokio::test]
async fn handles_ipv4() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "tcp://192.168.12.34:6638", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://192.168.12.34:6638", "adapter": "zstack"}),
    )
    .await;

    let tcp_path = parse_tcp_path("tcp://192.168.12.34:6638");
    assert_eq!(tcp_path.host, "192.168.12.34");
    assert_eq!(tcp_path.port, 6638);
}

#[tokio::test]
async fn handles_ipv6() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "tcp://[fd73:5e46:b9f0:0:80b5:4eff:dead:beef]:6638", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://[fd73:5e46:b9f0:0:80b5:4eff:dead:beef]:6638", "adapter": "zstack"}),
    )
    .await;

    let tcp_path = parse_tcp_path("tcp://[fd73:5e46:b9f0:0:80b5:4eff:dead:beef]:6638");
    assert_eq!(tcp_path.host, "fd73:5e46:b9f0:0:80b5:4eff:dead:beef");
    assert_eq!(tcp_path.port, 6638);
}

#[tokio::test]
async fn handles_domain_name() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "tcp://myadapter.local:6638", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://myadapter.local:6638", "adapter": "zstack"}),
    )
    .await;

    let tcp_path = parse_tcp_path("tcp://myadapter.local:6638");
    assert_eq!(tcp_path.host, "myadapter.local");
    assert_eq!(tcp_path.port, 6638);
}

#[tokio::test]
async fn handles_invalid_ipv6() {
    let result = create_with(
        &DiscoveryContext::default(),
        json!({"path": "tcp://fd73:5e46:b9f0:0:80b5:4eff:dead:beef:6638", "adapter": "zstack"}),
    )
    .await;

    assert_eq!(
        result.unwrap_err(),
        "Invalid TCP path, expected format: tcp://<host>:<port>"
    );
}

#[tokio::test]
async fn handles_mdns_error_during_find_all_devices() {
    let mut context = context_with(Vec::new());
    context.mdns_services = Vec::new();

    assert_eq!(find_all_devices(&mut context), Vec::new());
    assert_eq!(context.bonjour_destroyed, 1);
}

#[tokio::test]
async fn handles_nested_mdns_error_during_find_all_devices() {
    let mut context = context_with(Vec::new());
    context.mdns_start_error = Some("failed".to_string());

    assert_eq!(find_all_devices(&mut context), Vec::new());
    assert_eq!(context.bonjour_destroyed, 0);
}

#[tokio::test]
async fn handles_serial_error_during_find_all_devices() {
    let mut context = context(Err("fail"));

    let expected = vec![
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::Ember),
            None,
            None,
        ),
        discovered(
            "Mock Adapter (tcp://192.168.1.123:1122)",
            "tcp://192.168.1.123:1122",
            Some(AdapterKind::ZStack),
            None,
            None,
        ),
    ];

    assert_eq!(find_all_devices(&mut context), expected);
    assert_eq!(context.bonjour_destroyed, 1);
}

#[tokio::test]
async fn invalid_adapter() {
    let result = create_with(
        &DiscoveryContext::default(),
        json!({"path": "tcp://192.168.1.321:3456"}),
    )
    .await;

    assert_eq!(
        result.unwrap_err(),
        "Cannot discover TCP adapters at this time. Specify valid 'adapter' and 'port' in your configuration."
    );
}

#[tokio::test]
async fn m_dns_discovery() {
    let cases = [
        ("deconz", AdapterKind::Deconz),
        ("ember", AdapterKind::Ember),
        ("ezsp", AdapterKind::Ember), // `ezsp` radio_type uses `ember` Adapter
        ("zstack", AdapterKind::ZStack),
        ("zboss", AdapterKind::ZBoss),
        ("zigate", AdapterKind::ZiGate),
    ];

    for (name, kind) in cases {
        let adapter = create_with(
            &DiscoveryContext::default(),
            json!({"path": format!("mdns://{name}")}),
        )
        .await
        .expect("adapter");

        assert_eq!(adapter.kind, kind);
        assert_eq!(
            adapter.serial_port_options,
            json!({
                "path": "tcp://192.168.1.123:1122",
                "adapter": if name == "ezsp" { "ember" } else { name },
            })
        );
    }
}

#[tokio::test]
async fn returns_config_with_hostname_path() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "tcp://my-super-host:3456", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://my-super-host:3456", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn returns_config_with_socket_path() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "socket://192.168.1.321:3456", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://192.168.1.321:3456", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn returns_config_with_tcp_path() {
    assert_detect(
        &DiscoveryContext::default(),
        json!({"path": "tcp://192.168.1.321:3456", "adapter": "zstack"}),
        AdapterKind::ZStack,
        json!({"path": "tcp://192.168.1.321:3456", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn returns_first_from_list_with_multiple_adapters_nothing_to_match_against() {
    let context = context_with(vec![
        deconz_conbee_ii(),
        zstack_cc2538(),
        ember_zbdongle_e(),
    ]);

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": ember_zbdongle_e().path, "adapter": "ember", "rtscts": false}),
    )
    .await;
}

#[tokio::test]
async fn returns_instance_anyway_on_failure_to_get_serial_port_list() {
    let context = context(Err("spawn udevadm ENOENT"));

    assert_detect(
        &context,
        json!({"adapter": "zstack", "path": zstack_cc2538().path}),
        AdapterKind::ZStack,
        json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn returns_instance_anyway_on_failure_to_match() {
    let context = context_with(Vec::new());

    assert_detect(
        &context,
        json!({"adapter": "zstack", "path": "dev/ttyUSB0"}),
        AdapterKind::ZStack,
        json!({"path": "dev/ttyUSB0", "adapter": "zstack"}),
    )
    .await;
}

#[tokio::test]
async fn returns_instance_anyway_on_failure_to_match_with_different_path() {
    let context = context_with(vec![deconz_conbee_ii()]);

    assert_detect(
        &context,
        json!({"adapter": "deconz", "path": "/dev/ttyUSB0"}),
        AdapterKind::Deconz,
        json!({"path": "/dev/ttyUSB0", "adapter": "deconz"}),
    )
    .await;
}

#[tokio::test]
async fn returns_invalid_format() {
    let context = DiscoveryContext {
        mdns_services: vec![MdnsService {
            name: Some("Mock Adapter".to_string()),
            radio_type: None,
            host: "my_adapter.local".to_string(),
            port: "1122".to_string(),
            addresses: vec!["192.168.1.123".to_string()],
        }],
        ..DiscoveryContext::default()
    };

    let result = create_with(&context, json!({"path": "mdns://my_adapter"})).await;

    assert_eq!(
        result.unwrap_err(),
        "Coordinator returned wrong Zeroconf format! The following values are expected:\ntxt.radio_type, got: undefined\nport, got: 1122"
    );
}

#[tokio::test]
async fn throws_on_failure_to_detect_with_conflict_vendor_product_ids() {
    let context = context_with(vec![SerialPortInfo {
        path: "/dev/ttyACM0".to_string(),
        manufacturer: None,
        ..ember_skyconnect()
    }]);

    assert_eq!(
        create_with(&context, json!({})).await.unwrap_err(),
        "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration."
    );
}

#[tokio::test]
async fn throws_on_failure_to_get_serial_port_list() {
    let context = context(Err("spawn udevadm ENOENT"));

    assert_eq!(
        create_with(&context, json!({})).await.unwrap_err(),
        "USB adapter discovery error (spawn udevadm ENOENT). Specify valid 'adapter' and 'port' in your configuration."
    );
}

#[tokio::test]
async fn throws_on_failure_to_match_invalid_adapter() {
    let context = context_with(Vec::new());

    assert_eq!(
        create_with(&context, json!({"adapter": "invalid", "path": "dev/ttyUSB0"}))
            .await
            .unwrap_err(),
        "Adapter 'invalid' does not exists, possible options: zstack, ember, deconz, zigate, zboss, zoh, ezsp"
    );
}

#[tokio::test]
async fn throws_on_failure_to_match_when_port_info_too_limited() {
    let context = context_with(vec![SerialPortInfo {
        vendor_id: None,
        ..deconz_conbee_ii()
    }]);
    assert_eq!(
        create_with(&context, json!({})).await.unwrap_err(),
        "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration."
    );

    let context = context_with(vec![SerialPortInfo {
        product_id: None,
        ..deconz_conbee_ii()
    }]);
    assert_eq!(
        create_with(&context, json!({})).await.unwrap_err(),
        "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration."
    );
}

#[tokio::test]
async fn throws_on_failure_to_match_with_different_path() {
    let context = context_with(vec![deconz_conbee_ii()]);

    assert_eq!(
        create_with(&context, json!({"path": "/dev/ttyUSB0"}))
            .await
            .unwrap_err(),
        "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration."
    );
}

#[tokio::test]
async fn times_out() {
    let context = DiscoveryContext {
        mdns_services: Vec::new(),
        ..DiscoveryContext::default()
    };

    let result = create_with(&context, json!({"path": "mdns://mdns_test_device"})).await;

    assert_eq!(
        result.unwrap_err(),
        "Coordinator [mdns_test_device] not found after timeout of 2000ms!"
    );
}

#[tokio::test]
async fn uses_default_serial_port_options_of_adapter() {
    let context = context_with(vec![zbt_2()]);

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({
            "adapter": "ember",
            "baudRate": 460800,
            "path": zbt_2().path,
            "rtscts": true,
        }),
    )
    .await;
}

#[tokio::test]
async fn uses_provided_options_instead_of_default_serial_port_options_of_adapter() {
    let context = context_with(vec![zbt_2()]);

    assert_detect(
        &context,
        json!({"rtscts": false, "baudRate": 1}),
        AdapterKind::Ember,
        json!({
            "adapter": "ember",
            "baudRate": 1,
            "path": zbt_2().path,
            "rtscts": false,
        }),
    )
    .await;
}

#[tokio::test]
async fn with_adapter_only_config() {
    let cases = [
        (
            vec![deconz_conbee_ii()],
            json!({"adapter": "deconz"}),
            AdapterKind::Deconz,
            json!({"path": deconz_conbee_ii().path, "adapter": "deconz"}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"adapter": "ember"}),
            AdapterKind::Ember,
            json!({"path": ember_zbdongle_e().path, "adapter": "ember", "rtscts": false}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"adapter": "ezsp"}),
            AdapterKind::Ezsp,
            json!({"path": ember_zbdongle_e().path, "adapter": "ezsp", "rtscts": false}),
        ),
        (
            vec![zstack_cc2538()],
            json!({"adapter": "zstack"}),
            AdapterKind::ZStack,
            json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
        ),
        (
            vec![zboss_nordic()],
            json!({"adapter": "zboss"}),
            AdapterKind::ZBoss,
            json!({"path": zboss_nordic().path, "adapter": "zboss"}),
        ),
        (
            vec![zigate_plusv2()],
            json!({"adapter": "zigate"}),
            AdapterKind::ZiGate,
            json!({"path": zigate_plusv2().path, "adapter": "zigate"}),
        ),
    ];

    for (ports, serial, kind, options) in cases {
        assert_detect(&context_with(ports), serial, kind, options).await;
    }
}

#[tokio::test]
async fn with_adapter_path_config() {
    let cases = [
        (
            vec![deconz_conbee_ii()],
            json!({"adapter": "deconz", "path": deconz_conbee_ii().path}),
            AdapterKind::Deconz,
            json!({"path": deconz_conbee_ii().path, "adapter": "deconz"}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"adapter": "ember", "path": ember_zbdongle_e().path}),
            AdapterKind::Ember,
            json!({"path": ember_zbdongle_e().path, "adapter": "ember", "rtscts": false}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"adapter": "ezsp", "path": ember_zbdongle_e().path}),
            AdapterKind::Ezsp,
            json!({"path": ember_zbdongle_e().path, "adapter": "ezsp", "rtscts": false}),
        ),
        (
            vec![zstack_cc2538()],
            json!({"adapter": "zstack", "path": zstack_cc2538().path}),
            AdapterKind::ZStack,
            json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
        ),
        (
            vec![zboss_nordic()],
            json!({"adapter": "zboss", "path": zboss_nordic().path}),
            AdapterKind::ZBoss,
            json!({"path": zboss_nordic().path, "adapter": "zboss"}),
        ),
        (
            vec![zigate_plusv2()],
            json!({"adapter": "zigate", "path": zigate_plusv2().path}),
            AdapterKind::ZiGate,
            json!({"path": zigate_plusv2().path, "adapter": "zigate"}),
        ),
    ];

    for (ports, serial, kind, options) in cases {
        assert_detect(&context_with(ports), serial, kind, options).await;
    }
}

#[tokio::test]
async fn with_path_only_config() {
    let cases = [
        (
            vec![deconz_conbee_ii()],
            json!({"path": deconz_conbee_ii().path}),
            AdapterKind::Deconz,
            json!({"path": deconz_conbee_ii().path, "adapter": "deconz"}),
        ),
        (
            vec![ember_zbdongle_e()],
            json!({"path": ember_zbdongle_e().path}),
            AdapterKind::Ember,
            json!({"path": ember_zbdongle_e().path, "adapter": "ember", "rtscts": false}),
        ),
        (
            vec![zstack_cc2538()],
            json!({"path": zstack_cc2538().path}),
            AdapterKind::ZStack,
            json!({"path": zstack_cc2538().path, "adapter": "zstack"}),
        ),
        (
            vec![zboss_nordic()],
            json!({"path": zboss_nordic().path}),
            AdapterKind::ZBoss,
            json!({"path": zboss_nordic().path, "adapter": "zboss"}),
        ),
        (
            vec![zigate_plusv2()],
            json!({"path": zigate_plusv2().path}),
            AdapterKind::ZiGate,
            json!({"path": zigate_plusv2().path, "adapter": "zigate"}),
        ),
    ];

    for (ports, serial, kind, options) in cases {
        assert_detect(&context_with(ports), serial, kind, options).await;
    }
}

#[tokio::test]
async fn without_config() {
    // multiple adapters connected: first sorted by path is Ember
    let context = context_with(vec![
        deconz_conbee_ii(),
        zstack_cc2538(),
        ember_zbdongle_e(),
    ]);

    assert_detect(
        &context,
        json!({}),
        AdapterKind::Ember,
        json!({"path": ember_zbdongle_e().path, "adapter": "ember", "rtscts": false}),
    )
    .await;
}
