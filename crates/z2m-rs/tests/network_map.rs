// Original: .upstream/zigbee2mqtt/test/extensions/networkMap.test.ts
//
// Topology (upstream `mock()`):
//             | -> external_device
//             | -> bulb_color -> unsupported_router (offline)
// coordinator |      ^     ^
//             |      |     | (not valid)
//             | -> bulb    |
//                    |  -> CC2530_ROUTER -> WXKG02LM_rev1

use serde_json::{json, Value};
use z2m_rs::extension::network_map::{
    NetworkMap, NetworkMapDevice, NetworkMapLqi, NetworkMapRoute,
};

const COORDINATOR: &str = "0x00124b00120144ae";
const BULB: &str = "0x000b57fffec6a5b2";
const BULB_COLOR: &str = "0x000b57fffec6a5b3";
const BUTTON: &str = "0x0017880104e45521";
const UNSUPPORTED_ROUTER: &str = "0x0017880104e45525";
const CC2530_ROUTER: &str = "0x0017880104e45559";
const EXTERNAL_DEVICE: &str = "0x0017880104e45511";

fn setup() -> NetworkMap {
    let mut map = NetworkMap::new();
    map.add_device(NetworkMapDevice::coordinator(COORDINATOR, 0));
    map.add_device(NetworkMapDevice {
        ieee_addr: BULB.to_string(),
        network_address: 40369,
        device_type: "Router".to_string(),
        friendly_name: "bulb".to_string(),
        definition: Some(json!({
            "description": "TRADFRI bulb E26/E27, white spectrum, globe, opal, 980 lm",
            "model": "LED1545G12",
            "supports": "light (state, brightness, color_temp, color_temp_startup, level_config), effect, power_on_behavior, color_options, identify, linkquality",
            "vendor": "IKEA"
        })),
        model_id: Some("TRADFRI bulb E27 WS opal 980lm".to_string()),
        manufacturer_name: None,
        last_seen: Some(1000),
        failed: Vec::new(),
    });
    map.add_device(NetworkMapDevice {
        ieee_addr: BULB_COLOR.to_string(),
        network_address: 40399,
        device_type: "Router".to_string(),
        friendly_name: "bulb_color".to_string(),
        definition: Some(json!({
            "description": "Hue Go",
            "model": "7146060PH",
            "supports": "light (state, brightness, color_temp, color_temp_startup, color_xy, color_hs), power_on_behavior, effect, effect_speed, effect_color, identify, linkquality",
            "vendor": "Philips"
        })),
        model_id: Some("LLC020".to_string()),
        manufacturer_name: None,
        last_seen: Some(1000),
        failed: Vec::new(),
    });
    map.add_device(NetworkMapDevice {
        ieee_addr: BUTTON.to_string(),
        network_address: 6538,
        device_type: "EndDevice".to_string(),
        friendly_name: "button_double_key".to_string(),
        definition: Some(json!({
            "description": "Wireless remote switch (double rocker), 2016 model",
            "model": "WXKG02LM_rev1",
            "supports": "battery, voltage, power_outage_count, action, linkquality",
            "vendor": "Aqara"
        })),
        model_id: Some("lumi.sensor_86sw2.es1".to_string()),
        manufacturer_name: None,
        last_seen: Some(1000),
        failed: Vec::new(),
    });
    map.add_device(NetworkMapDevice {
        ieee_addr: UNSUPPORTED_ROUTER.to_string(),
        network_address: 6536,
        device_type: "Router".to_string(),
        friendly_name: UNSUPPORTED_ROUTER.to_string(),
        definition: Some(json!({
            "description": "Automatically generated definition",
            "model": "notSupportedModelID",
            "supports": "action, linkquality",
            "vendor": "Boef"
        })),
        model_id: Some("notSupportedModelID".to_string()),
        manufacturer_name: Some("Boef".to_string()),
        last_seen: Some(1000),
        failed: vec!["lqi".to_string(), "routingTable".to_string()],
    });
    map.add_device(NetworkMapDevice {
        ieee_addr: CC2530_ROUTER.to_string(),
        network_address: 6540,
        device_type: "Router".to_string(),
        friendly_name: "cc2530_router".to_string(),
        definition: Some(json!({
            "description": "CC2530 router",
            "model": "CC2530.ROUTER",
            "supports": "led, linkquality",
            "vendor": "Custom devices (DiY)"
        })),
        model_id: Some("lumi.router".to_string()),
        manufacturer_name: None,
        last_seen: Some(1000),
        failed: Vec::new(),
    });
    map.add_device(NetworkMapDevice {
        ieee_addr: EXTERNAL_DEVICE.to_string(),
        network_address: 1114,
        device_type: "EndDevice".to_string(),
        friendly_name: EXTERNAL_DEVICE.to_string(),
        definition: Some(json!({
            "description": "external/converter",
            "model": "external_converter_device",
            "supports": "linkquality",
            "vendor": "external"
        })),
        model_id: Some("external_converter_device".to_string()),
        manufacturer_name: None,
        last_seen: Some(1000),
        failed: Vec::new(),
    });
    map
}

fn lqi(eui64: &str, nwk_address: u16, relationship: u8, depth: u8, lqi: u8) -> NetworkMapLqi {
    NetworkMapLqi {
        eui64: eui64.to_string(),
        nwk_address,
        relationship,
        depth,
        lqi,
    }
}

fn mock_lqi(map: &mut NetworkMap) {
    map.set_lqi(
        COORDINATOR,
        vec![
            lqi(BULB_COLOR, 40399, 2, 1, 120),
            lqi(BULB, 40369, 2, 1, 92),
            lqi(EXTERNAL_DEVICE, 1114, 2, 1, 92),
        ],
    );
    map.set_routing_table(
        COORDINATOR,
        vec![NetworkMapRoute {
            destination_address: 6540,
            next_hop_address: 40369,
            status: "ACTIVE".to_string(),
        }],
    );
    map.set_lqi(
        BULB,
        vec![
            lqi(BULB_COLOR, 40399, 1, 2, 110),
            lqi(CC2530_ROUTER, 6540, 1, 2, 100),
        ],
    );
    map.set_lqi(
        CC2530_ROUTER,
        vec![
            lqi("0x0000000000000000", 6538, 1, 2, 130),
            lqi(BULB_COLOR, 40399, 4, 2, 130),
        ],
    );
    // upstream: unsupported_router lqi/routingTable reject twice
    map.fail_attempts(UNSUPPORTED_ROUTER, 2);
}

fn base_link() -> Value {
    json!({
        "deviceType": 0,
        "rxOnWhenIdle": 0,
        "permitJoining": 0,
    })
}

fn raw_link(
    source: (&str, u16),
    target: (&str, u16),
    depth: u8,
    lqi: u8,
    relationship: u8,
    routes: Vec<Value>,
) -> Value {
    let mut link = base_link();
    let object = link.as_object_mut().unwrap();
    object.insert("depth".to_string(), json!(depth));
    object.insert("linkquality".to_string(), json!(lqi));
    object.insert("lqi".to_string(), json!(lqi));
    object.insert("relationship".to_string(), json!(relationship));
    object.insert("routes".to_string(), json!(routes));
    object.insert(
        "source".to_string(),
        json!({"ieeeAddr": source.0, "networkAddress": source.1}),
    );
    object.insert("sourceIeeeAddr".to_string(), json!(source.0));
    object.insert("sourceNwkAddr".to_string(), json!(source.1));
    object.insert(
        "target".to_string(),
        json!({"ieeeAddr": target.0, "networkAddress": target.1}),
    );
    object.insert("targetIeeeAddr".to_string(), json!(target.0));
    link
}

fn raw_route(destination_address: u16, next_hop_address: u16) -> Value {
    json!({
        "destinationAddress": destination_address,
        "nextHopAddress": next_hop_address,
        "status": "ACTIVE",
        "memoryConstrained": 0,
        "manyToOne": 0,
        "routeRecordRequired": 0,
        "reserved1": 0,
    })
}

fn raw_links_full() -> Vec<Value> {
    vec![
        raw_link((BULB_COLOR, 40399), (COORDINATOR, 0), 1, 120, 2, vec![]),
        raw_link(
            (BULB, 40369),
            (COORDINATOR, 0),
            1,
            92,
            2,
            vec![raw_route(6540, 40369)],
        ),
        raw_link((EXTERNAL_DEVICE, 1114), (COORDINATOR, 0), 1, 92, 2, vec![]),
        raw_link((BULB_COLOR, 40399), (BULB, 40369), 2, 110, 1, vec![]),
        raw_link((CC2530_ROUTER, 6540), (BULB, 40369), 2, 100, 1, vec![]),
        raw_link((BUTTON, 6538), (CC2530_ROUTER, 6540), 2, 130, 1, vec![]),
    ]
}

fn raw_links_without_bulb() -> Vec<Value> {
    vec![
        raw_link((BULB_COLOR, 40399), (COORDINATOR, 0), 1, 120, 2, vec![]),
        raw_link(
            (BULB, 40369),
            (COORDINATOR, 0),
            1,
            92,
            2,
            vec![raw_route(6540, 40369)],
        ),
        raw_link((EXTERNAL_DEVICE, 1114), (COORDINATOR, 0), 1, 92, 2, vec![]),
        raw_link((BUTTON, 6538), (CC2530_ROUTER, 6540), 2, 130, 1, vec![]),
    ]
}

fn node(device: &NetworkMapDevice) -> Value {
    let mut node = json!({
        "failed": device.failed,
        "friendlyName": device.friendly_name,
        "ieeeAddr": device.ieee_addr,
        "lastSeen": device.last_seen,
        "networkAddress": device.network_address,
        "type": device.device_type,
    });
    let object = node.as_object_mut().unwrap();
    if let Some(definition) = &device.definition {
        object.insert("definition".to_string(), definition.clone());
    }
    if let Some(model_id) = &device.model_id {
        object.insert("modelID".to_string(), json!(model_id));
    }
    if let Some(manufacturer_name) = &device.manufacturer_name {
        object.insert("manufacturerName".to_string(), json!(manufacturer_name));
    }
    node
}

fn expected_raw(map: &NetworkMap, include_bulb: bool) -> Value {
    let nodes: Vec<Value> = map
        .devices
        .iter()
        .filter(|device| include_bulb || device.ieee_addr != BULB)
        .map(node)
        .collect();
    let links = if include_bulb {
        raw_links_full()
    } else {
        raw_links_without_bulb()
    };
    json!({
        "data": {
            "routes": true,
            "type": "raw",
            "value": {"links": links, "nodes": nodes},
        },
        "status": "ok",
    })
}

fn assert_published(network_map: &NetworkMap, expected: Value) {
    assert_eq!(network_map.published.len(), 1);
    assert_eq!(
        network_map.published[0].0,
        "zigbee2mqtt/bridge/response/networkmap"
    );
    assert_eq!(network_map.published[0].1, expected);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn output_raw_networkmap() {
    let mut network_map = setup();
    mock_lqi(&mut network_map);
    network_map
        .handle_request(json!({"type": "raw", "routes": true}))
        .await;

    assert_published(&network_map, expected_raw(&network_map, true));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn output_graphviz_networkmap() {
    let mut network_map = setup();
    mock_lqi(&mut network_map);

    // upstream mutates the definition description with an embedded quote
    let bulb_color = network_map
        .devices
        .iter_mut()
        .find(|device| device.ieee_addr == BULB_COLOR)
        .unwrap();
    let definition = bulb_color.definition.as_mut().unwrap();
    definition["description"] = json!("Hue Go 6\"");
    bulb_color.last_seen = None;

    network_map
        .handle_request(json!({"type": "graphviz", "routes": true}))
        .await;

    assert_eq!(network_map.published.len(), 1);
    assert_eq!(
        network_map.published[0].0,
        "zigbee2mqtt/bridge/response/networkmap"
    );

    let expected = r##"digraph G {
        node[shape=record];
          "0x00124b00120144ae" [style="bold, filled", fillcolor="#e04e5d", fontcolor="#ffffff", label="{Coordinator|0x00124b00120144ae (0x0000)|0 seconds ago}"];
          "0x000b57fffec6a5b2" [style="rounded, filled", fillcolor="#4ea3e0", fontcolor="#ffffff", label="{bulb|0x000b57fffec6a5b2 (0x9db1)|IKEA TRADFRI bulb E26/E27, white spectrum, globe, opal, 980 lm (LED1545G12)|9 seconds ago}"];
          "0x000b57fffec6a5b2" -> "0x00124b00120144ae" [penwidth=2, weight=1, color="#009900", label="92 (routes: 0x198c)"]
          "0x000b57fffec6a5b3" [style="rounded, filled", fillcolor="#4ea3e0", fontcolor="#ffffff", label="{bulb_color|0x000b57fffec6a5b3 (0x9dcf)|Philips Hue Go 6\" (7146060PH)|unknown}"];
          "0x000b57fffec6a5b3" -> "0x00124b00120144ae" [penwidth=0.5, weight=0, color="#994444", label="120"]
          "0x000b57fffec6a5b3" -> "0x000b57fffec6a5b2" [penwidth=0.5, weight=0, color="#994444", label="110"]
          "0x0017880104e45521" [style="rounded, dashed, filled", fillcolor="#fff8ce", fontcolor="#000000", label="{button_double_key|0x0017880104e45521 (0x198a)|Aqara Wireless remote switch (double rocker), 2016 model (WXKG02LM_rev1)|9 seconds ago}"];
          "0x0017880104e45521" -> "0x0017880104e45559" [penwidth=1, weight=0, color="#994444", label="130"]
          "0x0017880104e45525" [style="rounded, filled", fillcolor="#4ea3e0", fontcolor="#ffffff", label="{0x0017880104e45525|0x0017880104e45525 (0x1988)failed: lqi,routingTable|Boef Automatically generated definition (notSupportedModelID)|9 seconds ago}"];
          "0x0017880104e45559" [style="rounded, filled", fillcolor="#4ea3e0", fontcolor="#ffffff", label="{cc2530_router|0x0017880104e45559 (0x198c)|Custom devices (DiY) CC2530 router (CC2530.ROUTER)|9 seconds ago}"];
          "0x0017880104e45559" -> "0x000b57fffec6a5b2" [penwidth=0.5, weight=0, color="#994444", label="100"]
          "0x0017880104e45511" [style="rounded, dashed, filled", fillcolor="#fff8ce", fontcolor="#000000", label="{0x0017880104e45511|0x0017880104e45511 (0x045a)|external external/converter (external_converter_device)|9 seconds ago}"];
          "0x0017880104e45511" -> "0x00124b00120144ae" [penwidth=1, weight=0, color="#994444", label="92"]
        }"##;

    let expected_lines: Vec<&str> = expected.lines().map(str::trim).collect();
    let actual = network_map.published[0].1["data"]["value"]
        .as_str()
        .unwrap();
    let actual_lines: Vec<&str> = actual.lines().map(str::trim).collect();

    for (expected, actual) in expected_lines.iter().zip(actual_lines.iter()) {
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn output_plantuml_networkmap() {
    let mut network_map = setup();
    mock_lqi(&mut network_map);

    let bulb_color = network_map
        .devices
        .iter_mut()
        .find(|device| device.ieee_addr == BULB_COLOR)
        .unwrap();
    bulb_color.last_seen = None;

    network_map
        .handle_request(json!({"type": "plantuml", "routes": true}))
        .await;

    assert_eq!(network_map.published.len(), 1);
    assert_eq!(
        network_map.published[0].0,
        "zigbee2mqtt/bridge/response/networkmap"
    );

    let expected = r##"' paste into: https://www.planttext.com/

@startuml
card 0x0017880104e45511 [
0x0017880104e45511
---
0x0017880104e45511 (0x045a)
---
external external/converter (external_converter_device)
---
9 seconds ago
]

card 0x0017880104e45525 [
0x0017880104e45525
---
0x0017880104e45525 (0x1988) failed: lqi,routingTable
---
Boef Automatically generated definition (notSupportedModelID)
---
9 seconds ago
]

card 0x000b57fffec6a5b2 [
bulb
---
0x000b57fffec6a5b2 (0x9db1)
---
IKEA TRADFRI bulb E26/E27, white spectrum, globe, opal, 980 lm (LED1545G12)
---
9 seconds ago
]

card 0x000b57fffec6a5b3 [
bulb_color
---
0x000b57fffec6a5b3 (0x9dcf)
---
Philips Hue Go (7146060PH)
---
unknown
]

card 0x0017880104e45521 [
button_double_key
---
0x0017880104e45521 (0x198a)
---
Aqara Wireless remote switch (double rocker), 2016 model (WXKG02LM_rev1)
---
9 seconds ago
]

card 0x0017880104e45559 [
cc2530_router
---
0x0017880104e45559 (0x198c)
---
Custom devices (DiY) CC2530 router (CC2530.ROUTER)
---
9 seconds ago
]

card 0x00124b00120144ae [
Coordinator
---
0x00124b00120144ae (0x0000)
---
0 seconds ago
]

0x000b57fffec6a5b3 --> 0x00124b00120144ae: 120
0x000b57fffec6a5b2 --> 0x00124b00120144ae: 92
0x0017880104e45511 --> 0x00124b00120144ae: 92
0x000b57fffec6a5b3 --> 0x000b57fffec6a5b2: 110
0x0017880104e45559 --> 0x000b57fffec6a5b2: 100
0x0017880104e45521 --> 0x0017880104e45559: 130

@enduml"##;

    let expected_lines: Vec<&str> = expected.lines().map(str::trim).collect();
    let actual = network_map.published[0].1["data"]["value"]
        .as_str()
        .unwrap();
    let actual_lines: Vec<&str> = actual.lines().map(str::trim).collect();

    for (expected, actual) in expected_lines.iter().zip(actual_lines.iter()) {
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_requesting_invalid_type() {
    let mut network_map = setup();
    mock_lqi(&mut network_map);

    network_map.handle_request(json!("not_existing")).await;

    assert_eq!(network_map.published.len(), 1);
    assert_eq!(
        network_map.published[0],
        (
            "zigbee2mqtt/bridge/response/networkmap".to_string(),
            json!({
                "data": {},
                "status": "error",
                "error": "Type 'not_existing' not supported, allowed are: raw,graphviz,plantuml",
            }),
            json!({}),
        )
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_exclude_disabled_devices_from_networkmap() {
    let mut network_map = setup();
    network_map.disable_device(BULB);
    mock_lqi(&mut network_map);

    network_map
        .handle_request(json!({"type": "raw", "routes": true}))
        .await;

    assert_published(&network_map, expected_raw(&network_map, false));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handles_retrying_request_when_first_attempt_fails() {
    let mut network_map = setup();
    network_map.disable_device(BULB);
    mock_lqi(&mut network_map);
    network_map.fail_attempts(BULB, 1);

    network_map
        .handle_request(json!({"type": "raw", "routes": true}))
        .await;

    assert_published(&network_map, expected_raw(&network_map, false));
}
