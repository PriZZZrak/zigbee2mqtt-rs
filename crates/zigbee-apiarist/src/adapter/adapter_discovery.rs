// Original: .upstream/zigbee-herdsman/src/adapter/adapterDiscovery.ts

use crate::adapter::adapter::AdapterKind;
use crate::adapter::serial_port::SerialPortInfo;

/// Upstream `findAllDevices()` result entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAdapter {
    pub name: String,
    pub path: String,
    pub adapter: Option<AdapterKind>,
    /// Upstream camelCase: `baudRate`.
    pub baud_rate: Option<u32>,
    pub rtscts: Option<bool>,
}

/// One mDNS service (upstream `bonjour-service` `Service`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsService {
    pub name: Option<String>,
    /// Upstream `txt.radio_type`.
    pub radio_type: Option<String>,
    pub host: String,
    pub port: String,
    pub addresses: Vec<String>,
}

/// Inputs for `findAllDevices()`: `SerialPort.list()` and the mDNS browser.
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    /// Upstream `node:os.platform()`.
    pub platform: String,
    /// Upstream `SerialPort.list()` result.
    pub serial_ports: Result<Vec<SerialPortInfo>, String>,
    /// Services returned by the mDNS browser, in discovery order.
    pub mdns_services: Vec<MdnsService>,
    /// Upstream `new Bonjour(options, errorCb)` error callback.
    pub mdns_start_error: Option<String>,
}

impl Default for DiscoveryContext {
    fn default() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
            serial_ports: Ok(Vec::new()),
            mdns_services: Vec::new(),
            mdns_start_error: None,
        }
    }
}

fn lower(value: Option<&String>) -> String {
    value.map(|value| value.to_lowercase()).unwrap_or_default()
}

fn port_text(port: &SerialPortInfo) -> String {
    format!(
        "{} {} {} {}",
        port.path,
        port.pnp_id.clone().unwrap_or_default(),
        port.friendly_name.clone().unwrap_or_default(),
        port.manufacturer.clone().unwrap_or_default()
    )
    .to_lowercase()
}

pub(crate) fn detect_kind(port: &SerialPortInfo) -> Option<AdapterKind> {
    let vendor = lower(port.vendor_id.as_ref());
    let product = lower(port.product_id.as_ref());
    let manufacturer = lower(port.manufacturer.as_ref());
    let text = port_text(port);
    if text.contains("zwa-2") {
        return None;
    }

    match (vendor.as_str(), product.as_str()) {
        ("1cf1", "0030") => return Some(AdapterKind::Deconz),
        ("0451", "16c8") => return Some(AdapterKind::ZStack),
        ("2fe3", "0100") => return Some(AdapterKind::ZBoss),
        ("0403", "6015") => return Some(AdapterKind::ZiGate),
        ("303a", "4001") => return Some(AdapterKind::Ember),
        ("1a86", "55d4") => return Some(AdapterKind::Ember),
        ("10c4", "ea60") => {
            if text.contains("slzb-06p10") {
                return Some(AdapterKind::ZStack);
            }
            if text.contains("slzb-07") {
                return Some(AdapterKind::Ember);
            }
            if text.contains("dongle_plus_v2") {
                return Some(AdapterKind::Ember);
            }
            if text.contains("dongle_plus_") {
                return Some(AdapterKind::ZStack);
            }
            if text.contains("zwa-2") {
                return None;
            }
            if text.contains("skyconnect") {
                return Some(AdapterKind::Ember);
            }
            if manufacturer.contains("nabu casa") {
                return None;
            }
            if manufacturer.contains("itead")
                && (text.contains("dongle_plus") || text.contains("sonoff"))
            {
                return Some(AdapterKind::Ember);
            }
            if manufacturer.contains("silicon labs") || text.contains("vid_10c4&pid_ea60") {
                return Some(AdapterKind::Ember);
            }
        }
        _ => {}
    }
    None
}

pub(crate) fn is_skyconnect(port: &SerialPortInfo) -> bool {
    let text = port_text(port);
    text.contains("skyconnect")
}

pub(crate) fn is_zbt2(port: &SerialPortInfo) -> bool {
    lower(port.vendor_id.as_ref()) == "303a" && lower(port.product_id.as_ref()) == "4001"
}

pub(crate) fn is_slzb07(port: &SerialPortInfo) -> bool {
    port_text(port).contains("slzb-07")
}

pub(crate) fn adapter_kind_name(kind: AdapterKind) -> &'static str {
    match kind {
        AdapterKind::Deconz => "deconz",
        AdapterKind::Ember => "ember",
        AdapterKind::Ezsp => "ezsp",
        AdapterKind::ZStack => "zstack",
        AdapterKind::ZBoss => "zboss",
        AdapterKind::ZiGate => "zigate",
        AdapterKind::ZoH => "zoh",
    }
}

pub(crate) fn kind_from_name(name: &str) -> Option<AdapterKind> {
    match name {
        "deconz" => Some(AdapterKind::Deconz),
        "ember" => Some(AdapterKind::Ember),
        "ezsp" => Some(AdapterKind::Ezsp),
        "zstack" => Some(AdapterKind::ZStack),
        "zboss" => Some(AdapterKind::ZBoss),
        "zigate" => Some(AdapterKind::ZiGate),
        "zoh" => Some(AdapterKind::ZoH),
        _ => None,
    }
}

/// Upstream `findAllDevices()`.
pub fn find_all_devices(context: &DiscoveryContext) -> Vec<DiscoveredAdapter> {
    if context.mdns_start_error.is_some() {
        return Vec::new();
    }

    let mut discovered = Vec::new();
    if let Ok(ports) = &context.serial_ports {
        let mut ports = ports.clone();
        ports.sort_by(|left, right| left.path.cmp(&right.path));
        for port in ports {
            let (adapter, baud_rate, rtscts) = if context.platform == "win32"
                && port
                    .friendly_name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains("silicon labs cp210x"))
            {
                (None, None, None)
            } else {
                let kind = detect_kind(&port);
                if kind == Some(AdapterKind::Ember) {
                    if is_zbt2(&port) {
                        (kind, Some(460800), Some(true))
                    } else {
                        (kind, None, Some(is_skyconnect(&port) || is_slzb07(&port)))
                    }
                } else {
                    (kind, None, None)
                }
            };
            let name = port
                .friendly_name
                .clone()
                .or_else(|| port.pnp_id.clone())
                .unwrap_or_else(|| port.path.clone());
            discovered.push(DiscoveredAdapter {
                name: format!(
                    "{} ({})",
                    name,
                    port.manufacturer.clone().unwrap_or_default()
                ),
                path: port.path,
                adapter,
                baud_rate,
                rtscts,
            });
        }
    }

    for service in &context.mdns_services {
        let host = service
            .addresses
            .first()
            .cloned()
            .unwrap_or_else(|| service.host.clone());
        let path = format!("tcp://{host}:{}", service.port);
        let adapter = match service.radio_type.as_deref() {
            Some("zstack") | Some("znp") => Some(AdapterKind::ZStack),
            Some("ezsp") => Some(AdapterKind::Ember),
            Some("deconz") => Some(AdapterKind::Deconz),
            Some("zboss") => Some(AdapterKind::ZBoss),
            Some("zigate") => Some(AdapterKind::ZiGate),
            _ => None,
        };
        discovered.push(DiscoveredAdapter {
            name: format!(
                "{} ({path})",
                service
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string())
            ),
            path,
            adapter,
            baud_rate: None,
            rtscts: None,
        });
    }

    discovered
}

/// Enumerate real serial ports (upstream `SerialPort.list()`).
///
/// `serial2` exposes device paths only; on Linux we enrich them from sysfs and
/// prefer stable `/dev/serial/by-id/...` paths when available.
pub fn live_serial_ports() -> Result<Vec<SerialPortInfo>, String> {
    let paths = serial2_tokio::SerialPort::available_ports().map_err(|error| error.to_string())?;
    let by_id = by_id_paths();
    let mut ports = Vec::new();
    for path in paths {
        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let chosen = by_id
            .iter()
            .find(|(target, _)| target == &canonical || target == &path)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let mut info = SerialPortInfo::new(&chosen);
        enrich_from_sysfs(&mut info, &path);
        ports.push(info);
    }
    Ok(ports)
}

/// Discover adapters from real serial ports. mDNS discovery is left to the
/// caller/context so this stays deterministic and hardware-independent.
pub fn discover_live_adapters() -> Result<Vec<DiscoveredAdapter>, String> {
    let mut context = DiscoveryContext {
        serial_ports: live_serial_ports(),
        mdns_services: Vec::new(),
        ..DiscoveryContext::default()
    };
    Ok(find_all_devices(&context))
}

#[cfg(target_os = "linux")]
fn by_id_paths() -> Vec<(std::path::PathBuf, String)> {
    let mut entries = Vec::new();
    let Ok(dir) = std::fs::read_dir("/dev/serial/by-id") else {
        return entries;
    };
    for entry in dir.flatten() {
        let path = entry.path();
        let target = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        entries.push((target, path.to_string_lossy().to_string()));
    }
    entries
}

#[cfg(not(target_os = "linux"))]
fn by_id_paths() -> Vec<(std::path::PathBuf, String)> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn enrich_from_sysfs(info: &mut SerialPortInfo, device_path: &std::path::Path) {
    let Some(name) = device_path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let link = format!("/sys/class/tty/{name}/device");
    let Ok(mut current) = std::fs::canonicalize(&link) else {
        return;
    };
    for _ in 0..4 {
        if current.join("idVendor").exists() {
            info.vendor_id = read_attr(&current, "idVendor");
            info.product_id = read_attr(&current, "idProduct");
            info.manufacturer = read_attr(&current, "manufacturer");
            info.serial_number = read_attr(&current, "serial");
            if info.friendly_name.is_none() {
                info.friendly_name = read_attr(&current, "product");
            }
            info.pnp_id = info
                .vendor_id
                .as_ref()
                .zip(info.product_id.as_ref())
                .map(|(vendor, product)| format!("USB VID_{vendor}&PID_{product}"));
            return;
        }
        let Some(parent) = current.parent() else {
            return;
        };
        current = parent.to_path_buf();
    }
}

#[cfg(target_os = "linux")]
fn read_attr(dir: &std::path::Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(not(target_os = "linux"))]
fn enrich_from_sysfs(_info: &mut SerialPortInfo, _device_path: &std::path::Path) {}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[test]
    fn live_enumerators_do_not_panic_without_hardware() {
        let _ = live_serial_ports();
        let _ = discover_live_adapters();
    }
}
