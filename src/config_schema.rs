//! Hand-written `config_schema` for the frontend settings pages.
//!
//! Upstream generates this document from its settings schema. This is an
//! independently authored compatibility schema (not a copy of upstream's GPL
//! document): it describes the settings this bridge supports with the same
//! JSON-schema shape the Windfront renderer expects (`properties`, `required`,
//! `definitions.device/group`).
//!
//! The renderer only understands `boolean`, `integer`/`number`, `string` and
//! `array` leaves; nested objects must carry `properties`.

use serde_json::{json, Value};

fn string(title: &str, description: &str) -> Value {
    json!({"type": "string", "title": title, "description": description})
}

fn boolean(title: &str, description: &str) -> Value {
    json!({"type": "boolean", "title": title, "description": description})
}

fn integer(title: &str, description: &str) -> Value {
    json!({"type": "integer", "title": title, "description": description})
}

fn enumeration(title: &str, description: &str, values: &[&str]) -> Value {
    json!({"type": "string", "title": title, "description": description, "enum": values})
}

fn section(title: &str, properties: Value) -> Value {
    json!({"type": "object", "title": title, "properties": properties})
}

/// Build the `config_schema` payload published in `bridge/info`.
#[must_use]
pub fn config_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "permit_join": boolean("Permit join", "Allow new devices to join the network"),
            "mqtt": section("MQTT", json!({
                "base_topic": string("Base topic", "MQTT base topic"),
                "server": string("Server", "MQTT broker URL or host"),
                "username": string("Username", "MQTT username"),
                "password": string("Password", "MQTT password"),
                "keepalive": integer("Keepalive", "MQTT keepalive in seconds"),
                "version": integer("Protocol version", "MQTT protocol version (4 = 3.1.1)"),
            })),
            "serial": section("Serial", json!({
                "port": string("Port", "Serial device, e.g. /dev/ttyUSB0"),
                "adapter": enumeration("Adapter", "Zigbee adapter type", &["znp", "zstack", "ezsp", "auto"]),
                "baudrate": integer("Baudrate", "Serial baudrate"),
                "rtscts": boolean("RTS/CTS", "Enable hardware flow control"),
                "disable_led": boolean("Disable LED", "Turn off the coordinator LED"),
            })),
            "frontend": section("Frontend", json!({
                "enabled": boolean("Enabled", "Serve the web frontend"),
                "host": string("Host", "Bind host; an absolute path selects a unix socket"),
                "port": integer("Port", "HTTP/WebSocket port"),
                "base_url": string("Base URL", "URL path prefix"),
                "auth_token": string("Auth token", "Optional WebSocket token"),
                "disable_ui_serving": boolean("Disable UI serving", "Only serve the WebSocket API"),
                "package": string("Package", "Frontend package name"),
            })),
            "homeassistant": section("Home Assistant", json!({
                "enabled": boolean("Enabled", "Publish MQTT discovery"),
                "discovery_topic": string("Discovery topic", "Home Assistant discovery prefix"),
                "status_topic": string("Status topic", "Home Assistant status topic"),
                "legacy_action_sensor": boolean("Legacy action sensor", "Publish legacy action sensors"),
                "experimental_event_entities": boolean("Event entities", "Publish experimental event entities"),
            })),
            "availability": section("Availability", json!({
                "enabled": boolean("Enabled", "Publish device availability"),
                "active": section("Active", json!({
                    "timeout": integer("Timeout", "Active ping timeout in seconds"),
                    "max_jitter": integer("Max jitter", "Maximum jitter in milliseconds"),
                })),
                "passive": section("Passive", json!({
                    "timeout": integer("Timeout", "Passive timeout in seconds"),
                })),
            })),
            "health": section("Health", json!({
                "enabled": boolean("Enabled", "Publish bridge health"),
                "interval": integer("Interval", "Health check interval in seconds"),
                "reset_on_check": boolean("Reset on check", "Reset counters on each check"),
            })),
            "ota": section("OTA", json!({
                "disable_automatic_update_check": boolean("Disable automatic check", "Do not check for updates automatically"),
                "update_check_interval": integer("Check interval", "Update check interval in minutes"),
            })),
            "advanced": section("Advanced", json!({
                "pan_id": integer("PAN ID", "Zigbee PAN ID"),
                "ext_pan_id": json!({"type": "array", "title": "Extended PAN ID", "description": "8 bytes", "items": {"type": "integer"}}),
                "channel": integer("Channel", "Zigbee channel (11-26)"),
                "log_level": enumeration("Log level", "Logging verbosity", &["error", "warn", "info", "debug"]),
                "last_seen": enumeration("Last seen", "Format of the last_seen field", &["disable", "ISO_8601", "ISO_8601_local", "epoch"]),
                "output": enumeration("Output", "State payload format", &["json", "attribute", "attribute_and_json"]),
                "cache_state": boolean("Cache state", "Cache device state"),
                "cache_state_persistent": boolean("Persistent cache", "Store cached state in state.json"),
                "cache_state_send_on_startup": boolean("Send cached state on startup", "Publish cached state after restart"),
                "timestamp_format": enumeration("Timestamp format", "Timestamp format", &["ISO_8601", "ISO_8601_local", "epoch"]),
            })),
            "blocklist": json!({"type": "array", "title": "Blocklist", "description": "Devices that may not join", "items": {"type": "string"}}),
            "passlist": json!({"type": "array", "title": "Passlist", "description": "Devices allowed to join", "items": {"type": "string"}}),
            "device_options": json!({"type": "object", "title": "Device options", "readOnly": true}),
            "devices": json!({"type": "object", "title": "Devices", "readOnly": true}),
            "groups": json!({"type": "object", "title": "Groups", "readOnly": true}),
            "map_options": json!({"type": "object", "title": "Map options", "readOnly": true}),
        },
        "required": [],
        "definitions": {
            "device": section("Device", json!({
                "friendly_name": string("Friendly name", "Name used in MQTT topics"),
                "retain": boolean("Retain", "Retain state messages"),
                "qos": integer("QoS", "MQTT QoS"),
                "disabled": boolean("Disabled", "Ignore this device"),
                "description": string("Description", "Free-text description"),
            })),
            "group": section("Group", json!({
                "friendly_name": string("Friendly name", "Name used in MQTT topics"),
                "retain": boolean("Retain", "Retain state messages"),
                "qos": integer("QoS", "MQTT QoS"),
                "transition": integer("Transition", "Default transition in seconds"),
            })),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_the_shapes_the_frontend_dereferences() {
        let schema = config_schema();
        assert!(schema["properties"].is_object());
        assert!(schema["definitions"]["device"]["properties"].is_object());
        assert!(schema["definitions"]["group"]["properties"].is_object());
        // Sections used by the settings tabs must be objects with properties.
        for section_name in [
            "mqtt",
            "serial",
            "frontend",
            "homeassistant",
            "availability",
            "health",
            "ota",
            "advanced",
        ] {
            assert!(
                schema["properties"][section_name]["properties"].is_object(),
                "missing section {section_name}"
            );
        }
        assert_eq!(schema["properties"]["permit_join"]["type"], "boolean");
        assert_eq!(
            schema["properties"]["mqtt"]["properties"]["password"]["type"],
            "string"
        );
    }
}
