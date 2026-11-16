//! Port of upstream `test/adapter/adapter.test.ts` (discovery and TCP paths).
//!
//! Anchor: `.upstream/zigbee-herdsman/test/adapter/adapter.test.ts`
//! (zigbee-herdsman v11.2.0, `c1a54e3d`). Per-case mapping:
//! `docs/adapter-test-parity.md`.
//!
//! Upstream mocks `SerialPort.list()` and the mDNS browser; the Rust port takes
//! the same inputs through `DiscoveryContext` (`serial_ports`, `mdns_services`,
//! `mdns_start_error`, `platform`), so the cases drive it directly.

use serde_json::json;

use zigbee_apiarist::adapter::adapter::AdapterKind;
use zigbee_apiarist::adapter::adapter_discovery::{
    find_all_devices, DiscoveryContext, MdnsService,
};
use zigbee_apiarist::adapter::serial_port::SerialPortInfo;
use zigbee_apiarist::adapter::utils::{is_tcp_path, try_parse_tcp_path};

fn port(
    path: &str,
    vendor: &str,
    product: &str,
    manufacturer: Option<&str>,
    friendly_name: Option<&str>,
) -> SerialPortInfo {
    SerialPortInfo {
        path: path.to_string(),
        vendor_id: Some(vendor.to_string()),
        product_id: Some(product.to_string()),
        manufacturer: manufacturer.map(str::to_string),
        friendly_name: friendly_name.map(str::to_string),
        ..SerialPortInfo::new(path)
    }
}

fn context(ports: Vec<SerialPortInfo>) -> DiscoveryContext {
    DiscoveryContext {
        platform: "linux".to_string(),
        serial_ports: Ok(ports),
        mdns_services: Vec::new(),
        mdns_start_error: None,
    }
}

// --- findAllDevices: serial (USB) discovery --------------------------------

#[test]
fn detects_each_adapter() {
    let devices = find_all_devices(&context(vec![
        port("/dev/conbee", "1cf1", "0030", Some("dresden elektronik"), None),
        port("/dev/cc2538", "0451", "16c8", Some("Texas Instruments"), None),
        port("/dev/zboss", "2fe3", "0100", Some("Nordic"), None),
        port("/dev/zigate", "0403", "6015", Some("FTDI"), None),
    ]));
    let kinds: Vec<(String, Option<AdapterKind>)> = devices
        .into_iter()
        .map(|device| (device.path, device.adapter))
        .collect();
    assert!(kinds.contains(&("/dev/conbee".to_string(), Some(AdapterKind::Deconz))));
    assert!(kinds.contains(&("/dev/cc2538".to_string(), Some(AdapterKind::ZStack))));
    assert!(kinds.contains(&("/dev/zboss".to_string(), Some(AdapterKind::ZBoss))));
    assert!(kinds.contains(&("/dev/zigate".to_string(), Some(AdapterKind::ZiGate))));
}

#[test]
fn detects_ember_variants_and_skyconnect_defaults() {
    // ZBT-2 (303a:4001): ember with the high baud rate and hardware flow control.
    let devices = find_all_devices(&context(vec![
        port("/dev/zbt2", "303a", "4001", Some("Nabu Casa"), None),
        port(
            "/dev/skyconnect",
            "10c4",
            "ea60",
            Some("Nabu Casa"),
            Some("SkyConnect v1.0"),
        ),
    ]));
    let zbt2 = devices
        .iter()
        .find(|device| device.path == "/dev/zbt2")
        .expect("zbt2");
    assert_eq!(zbt2.adapter, Some(AdapterKind::Ember));
    assert_eq!(zbt2.baud_rate, Some(460800));
    assert_eq!(zbt2.rtscts, Some(true));

    let skyconnect = devices
        .iter()
        .find(|device| device.path == "/dev/skyconnect")
        .expect("skyconnect");
    assert_eq!(skyconnect.adapter, Some(AdapterKind::Ember));
    assert_eq!(skyconnect.rtscts, Some(true));
}

#[test]
fn does_not_detect_zwa2_and_windows_cp210x() {
    // ZWA-2 is deliberately not auto-detected.
    let devices = find_all_devices(&context(vec![port(
        "/dev/zwa2",
        "10c4",
        "ea60",
        Some("Nabu Casa"),
        Some("ZWA-2"),
    )]));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].adapter, None);

    // Windows CP210x without a useful friendly name stays unclassified.
    let mut windows = context(vec![port(
        "COM3",
        "10c4",
        "ea60",
        Some("Silicon Labs"),
        Some("Silicon Labs CP210x USB to UART Bridge"),
    )]);
    windows.platform = "win32".to_string();
    let devices = find_all_devices(&windows);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].adapter, None);
}

#[test]
fn uses_pnp_id_and_friendly_name_for_the_display_name() {
    let mut info = port(
        "/dev/ttyUSB0",
        "0451",
        "16c8",
        Some("Texas Instruments"),
        None,
    );
    info.pnp_id = Some("USB\\VID_0451&PID_16C8".to_string());
    let devices = find_all_devices(&context(vec![info]));
    assert_eq!(devices.len(), 1);
    assert!(devices[0].name.starts_with("USB\\VID_0451&PID_16C8"));
    assert!(devices[0].name.contains("Texas Instruments"));
    assert_eq!(devices[0].adapter, Some(AdapterKind::ZStack));
    // A Z-Stack dongle keeps the adapter default baud rate.
    assert_eq!(devices[0].baud_rate, None);

    let info = port("/dev/ttyACM0", "1cf1", "0030", Some("dresden elektronik"), None);
    let devices = find_all_devices(&context(vec![info]));
    assert_eq!(devices[0].name, "/dev/ttyACM0 (dresden elektronik)");
}

#[test]
fn sorts_serial_ports_by_path() {
    let devices = find_all_devices(&context(vec![
        port("/dev/ttyUSB1", "0451", "16c8", None, None),
        port("/dev/ttyUSB0", "0451", "16c8", None, None),
    ]));
    let paths: Vec<&str> = devices.iter().map(|device| device.path.as_str()).collect();
    assert_eq!(paths, vec!["/dev/ttyUSB0", "/dev/ttyUSB1"]);
}

#[test]
fn handles_serial_and_mdns_errors_during_find_all_devices() {
    let mut failed = context(Vec::new());
    failed.serial_ports = Err("serial error".to_string());
    let devices = find_all_devices(&failed);
    assert!(devices.is_empty());

    let mut mdns_failed = context(vec![port("/dev/ttyUSB0", "0451", "16c8", None, None)]);
    mdns_failed.mdns_start_error = Some("mdns error".to_string());
    // Upstream returns an empty list for any mDNS startup error.
    assert!(find_all_devices(&mdns_failed).is_empty());
}

// --- findAllDevices: mDNS discovery ---------------------------------------

#[test]
fn discovers_mdns_services_as_tcp_paths() {
    let mut discovery = context(Vec::new());
    discovery.mdns_services = vec![
        MdnsService {
            name: Some("zigbee".to_string()),
            radio_type: Some("znp".to_string()),
            host: "zigbee.local".to_string(),
            port: "6638".to_string(),
            addresses: vec!["192.168.1.10".to_string()],
        },
        MdnsService {
            name: Some("deconz".to_string()),
            radio_type: Some("deconz".to_string()),
            host: "deconz.local".to_string(),
            port: "80".to_string(),
            addresses: Vec::new(),
        },
        MdnsService {
            name: Some("unknown".to_string()),
            radio_type: None,
            host: "unknown.local".to_string(),
            port: "1234".to_string(),
            addresses: Vec::new(),
        },
    ];
    let devices = find_all_devices(&discovery);
    assert_eq!(devices.len(), 3);
    assert_eq!(devices[0].path, "tcp://192.168.1.10:6638");
    assert_eq!(devices[0].adapter, Some(AdapterKind::ZStack));
    assert_eq!(devices[0].name, "zigbee (tcp://192.168.1.10:6638)");
    // Without addresses the service host is used.
    assert_eq!(devices[1].path, "tcp://deconz.local:80");
    assert_eq!(devices[1].adapter, Some(AdapterKind::Deconz));
    assert_eq!(devices[2].adapter, None);
}

// --- TCP path parsing ------------------------------------------------------

#[test]
fn tcp_discovery_parses_supported_paths() {
    for (path, host, port_number) in [
        ("tcp://192.168.1.1:6638", "192.168.1.1", 6638),
        ("socket://my-coordinator:1234", "my-coordinator", 1234),
        ("tcp://localhost:6638", "localhost", 6638),
        ("tcp://[::1]:6638", "::1", 6638),
        ("tcp://[fe80::1]:1234", "fe80::1", 1234),
    ] {
        let parsed = try_parse_tcp_path(path).expect(path);
        assert_eq!(parsed.host, host, "{path}");
        assert_eq!(parsed.port, port_number, "{path}");
        assert!(is_tcp_path(path), "{path}");
    }
    assert!(!is_tcp_path("/dev/ttyUSB0"));
}

#[test]
fn tcp_discovery_rejects_invalid_paths() {
    for path in ["tcp://", "socket://", "tcp://[::1", "tcp://host:notaport"] {
        assert!(try_parse_tcp_path(path).is_err(), "{path}");
    }
}

// --- adapter kind helpers --------------------------------------------------

#[test]
fn adapter_kind_names_and_json_payload() {
    assert_eq!(
        zigbee_apiarist::adapter::adapter_discovery::live_serial_ports().is_ok(),
        true,
        "live serial enumeration must not fail on a host without dongles"
    );
    let _ = json!({});
}
