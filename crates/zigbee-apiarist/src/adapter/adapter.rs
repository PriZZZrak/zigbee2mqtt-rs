// Original: .upstream/zigbee-herdsman/src/adapter/adapter.ts

use serde_json::{json, Map, Value};

use crate::adapter::adapter_discovery::{
    adapter_kind_name, detect_kind, is_skyconnect, is_slzb07, is_zbt2, kind_from_name,
    DiscoveryContext,
};
use crate::adapter::serial_port::SerialPortInfo;
use crate::adapter::utils::try_parse_tcp_path;

/// Upstream adapter classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Deconz,
    Ember,
    Ezsp,
    ZStack,
    ZBoss,
    ZiGate,
    ZoH,
}

/// Upstream created adapter instance (class + resolved serial options).
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterInstance {
    pub kind: AdapterKind,
    /// Upstream protected `serialPortOptions`.
    pub serial_port_options: Value,
}

/// Upstream `Adapter` factory.
pub struct Adapter;

impl Adapter {
    /// Test/dev variant with an injected `SerialPort.list()` / mDNS context.
    pub async fn create_with_context(
        context: &DiscoveryContext,
        _network_options: Value,
        serial_port_options: Value,
        _backup_path: &str,
        _options: Value,
    ) -> Result<AdapterInstance, String> {
        let adapter_name = serial_port_options
            .get("adapter")
            .and_then(Value::as_str)
            .map(str::to_string);
        let path = serial_port_options
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string);

        if let Some(path) = path.as_deref() {
            if path.starts_with("mdns://") {
                return discover_mdns(context, &serial_port_options, path);
            }
            if path.starts_with("tcp://") || path.starts_with("socket://") {
                return discover_tcp(&serial_port_options, path, adapter_name.as_deref());
            }
        }

        let requested_kind = match adapter_name.as_deref() {
            Some(name) => Some(kind_from_name(name).ok_or_else(|| {
                "Adapter 'invalid' does not exists, possible options: zstack, ember, deconz, zigate, zboss, zoh, ezsp"
                    .replace("invalid", name)
            })?),
            None => None,
        };

        let ports = match &context.serial_ports {
            Ok(ports) => ports.clone(),
            Err(error) => {
                if let (Some(kind), Some(path)) = (requested_kind, path.clone()) {
                    return Ok(instance(
                        kind,
                        &serial_port_options,
                        &path,
                        adapter_name.as_deref(),
                        None,
                        context.platform == "win32" && serial_port_options.get("path").is_some(),
                    ));
                }
                return Err(format!(
                    "USB adapter discovery error ({error}). Specify valid 'adapter' and 'port' in your configuration."
                ));
            }
        };
        let mut sorted = ports;
        sorted.sort_by(|left, right| left.path.cmp(&right.path));

        let found = path.as_ref().and_then(|path| {
            sorted
                .iter()
                .find(|port| &port.path == path || port.pnp_id.as_deref() == Some(path.as_str()))
                .cloned()
        });

        let (kind, port) = if let Some(port) = found.clone() {
            let kind = match requested_kind {
                Some(kind) => kind,
                None => detect_kind(&port).ok_or_else(no_valid_usb)?,
            };
            (kind, Some(port))
        } else if let Some(kind) = requested_kind {
            if path.is_none() {
                let port = sorted
                    .iter()
                    .find(|port| {
                        detect_kind(port).is_some_and(|detected| compatible(kind, detected))
                    })
                    .cloned()
                    .ok_or_else(no_valid_usb)?;
                (kind, Some(port))
            } else {
                (kind, None)
            }
        } else if path.is_some() {
            return Err(no_valid_usb());
        } else {
            let port = sorted
                .iter()
                .find(|port| detect_kind(port).is_some())
                .cloned()
                .ok_or_else(no_valid_usb)?;
            let kind = detect_kind(&port).ok_or_else(no_valid_usb)?;
            (kind, Some(port))
        };

        let resolved_path = port
            .as_ref()
            .map(|port| port.path.clone())
            .or_else(|| path.clone())
            .ok_or_else(no_valid_usb)?;
        let skip_defaults = port.is_none() || (context.platform == "win32" && path.is_some());
        Ok(instance(
            kind,
            &serial_port_options,
            &resolved_path,
            adapter_name.as_deref(),
            port.as_ref(),
            skip_defaults,
        ))
    }
}

fn no_valid_usb() -> String {
    "USB adapter discovery error (No valid USB adapter found). Specify valid 'adapter' and 'port' in your configuration.".to_string()
}

fn compatible(requested: AdapterKind, detected: AdapterKind) -> bool {
    requested == detected
        || (requested == AdapterKind::Ezsp && detected == AdapterKind::Ember)
        || (requested == AdapterKind::Ember && detected == AdapterKind::Ember)
}

fn instance(
    kind: AdapterKind,
    provided: &Value,
    path: &str,
    adapter_name: Option<&str>,
    port: Option<&SerialPortInfo>,
    skip_defaults: bool,
) -> AdapterInstance {
    let mut options = Map::new();
    options.insert("path".to_string(), json!(path));
    options.insert(
        "adapter".to_string(),
        json!(adapter_name.unwrap_or_else(|| adapter_kind_name(kind))),
    );

    if !skip_defaults {
        if let Some(port) = port {
            if matches!(kind, AdapterKind::Ember | AdapterKind::Ezsp) {
                if is_zbt2(port) {
                    options.insert("baudRate".to_string(), json!(460800));
                    options.insert("rtscts".to_string(), json!(true));
                } else {
                    options.insert(
                        "rtscts".to_string(),
                        json!(is_skyconnect(port) || is_slzb07(port)),
                    );
                }
            }
        }
    }
    if let Some(baud) = provided.get("baudRate") {
        options.insert("baudRate".to_string(), baud.clone());
    }
    if let Some(rtscts) = provided.get("rtscts") {
        options.insert("rtscts".to_string(), rtscts.clone());
    }

    AdapterInstance {
        kind,
        serial_port_options: Value::Object(options),
    }
}

fn discover_tcp(
    provided: &Value,
    path: &str,
    adapter_name: Option<&str>,
) -> Result<AdapterInstance, String> {
    let Some(adapter_name) = adapter_name else {
        return Err(
            "Cannot discover TCP adapters at this time. Specify valid 'adapter' and 'port' in your configuration."
                .to_string(),
        );
    };
    try_parse_tcp_path(path)
        .map_err(|_| "Invalid TCP path, expected format: tcp://<host>:<port>".to_string())?;
    let kind = kind_from_name(adapter_name).ok_or_else(|| {
        format!(
            "Adapter '{adapter_name}' does not exists, possible options: zstack, ember, deconz, zigate, zboss, zoh, ezsp"
        )
    })?;
    let normalized = if let Some(rest) = path.strip_prefix("socket://") {
        format!("tcp://{rest}")
    } else {
        path.to_string()
    };
    let mut options = Map::new();
    options.insert("path".to_string(), json!(normalized));
    options.insert("adapter".to_string(), json!(adapter_name));
    if let Some(baud) = provided.get("baudRate") {
        options.insert("baudRate".to_string(), baud.clone());
    }
    if let Some(rtscts) = provided.get("rtscts") {
        options.insert("rtscts".to_string(), rtscts.clone());
    }
    Ok(AdapterInstance {
        kind,
        serial_port_options: Value::Object(options),
    })
}

fn discover_mdns(
    context: &DiscoveryContext,
    provided: &Value,
    path: &str,
) -> Result<AdapterInstance, String> {
    let requested = path.trim_start_matches("mdns://");
    if requested.is_empty() {
        return Err(
            "No mdns device specified. You must specify the coordinator mdns service type after mdns://, e.g. mdns://my-adapter"
                .to_string(),
        );
    }
    if let Some(error) = &context.mdns_start_error {
        return Err(format!("Failed to start mdns discovery: Error: {error}"));
    }
    if context.mdns_services.is_empty() {
        return Err(format!(
            "Coordinator [{requested}] not found after timeout of 2000ms!"
        ));
    }
    let service = context
        .mdns_services
        .iter()
        .find(|service| service.radio_type.as_deref() == Some(requested))
        .unwrap_or(&context.mdns_services[0]);
    if service.radio_type.is_none() {
        return Err(format!(
            "Coordinator returned wrong Zeroconf format! The following values are expected:\ntxt.radio_type, got: undefined\nport, got: {}",
            service.port
        ));
    }
    let host = service
        .addresses
        .first()
        .cloned()
        .unwrap_or_else(|| service.host.clone());
    let tcp_path = format!("tcp://{host}:{}", service.port);
    let (kind, adapter_name) = match requested {
        "znp" => (AdapterKind::ZStack, "zstack"),
        "ezsp" => (AdapterKind::Ember, "ember"),
        other => (
            kind_from_name(other).ok_or_else(|| {
                format!(
                    "Adapter '{other}' does not exists, possible options: zstack, ember, deconz, zigate, zboss, zoh, ezsp"
                )
            })?,
            other,
        ),
    };
    let mut options = Map::new();
    options.insert("path".to_string(), json!(tcp_path));
    options.insert("adapter".to_string(), json!(adapter_name));
    if let Some(baud) = provided.get("baudRate") {
        options.insert("baudRate".to_string(), baud.clone());
    }
    if let Some(rtscts) = provided.get("rtscts") {
        options.insert("rtscts".to_string(), rtscts.clone());
    }
    Ok(AdapterInstance {
        kind,
        serial_port_options: Value::Object(options),
    })
}
