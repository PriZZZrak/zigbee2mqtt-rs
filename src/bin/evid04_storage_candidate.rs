//! EVID-04 storage candidate driver (harness only).
//!
//! Loads an upstream-style `database.db` with the same importer the bridge uses
//! and prints the normalized device view used for the storage differential.
//!
//! Usage:
//!   evid04_storage_candidate --db <database.db> --config <configuration.yaml>
//!   evid04_storage_candidate --db <db> --config <cfg> --rewrite <out.db>
use std::path::{Path, PathBuf};

use serde_json::json;
use zigbee2mqtt_rs::config::Config;
use zigbee2mqtt_rs::database::{load_database, PersistentDatabase};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-storage-candidate: {message}");
    std::process::exit(2);
}

fn normalize(database: &Path, config: &Path) -> serde_json::Value {
    let config = Config::load(config)
        .unwrap_or_else(|error| fail(format!("cannot load config {}: {error}", config.display())));
    let (mut devices, _) = load_database(database, &config.devices);
    devices.sort_by_key(|device| device.ieee_addr.as_hex());
    let devices: Vec<serde_json::Value> = devices
        .iter()
        .map(|device| {
            let mut endpoints: Vec<_> = device.endpoints.iter().collect();
            endpoints.sort_by_key(|endpoint| endpoint.endpoint);
            json!({
                "ieeeAddr": device.ieee_addr.as_hex(),
                "nwkAddr": device.nwk_addr,
                "friendlyName": device.friendly_name,
                "disabled": device.disabled,
                "manufacturerName": device.manufacturer,
                "manufacturerId": device.manufacturer_id,
                "powerSource": device.power_source,
                "modelId": device.model,
                "swBuildId": device.sw_build_id,
                "interviewComplete": device.interview_complete,
                "endpoints": endpoints
                    .iter()
                    .map(|endpoint| json!({
                        "id": endpoint.endpoint,
                        "profileId": endpoint.profile_id,
                        "deviceId": endpoint.device_id,
                        "inputClusters": endpoint.input_clusters,
                        "outputClusters": endpoint.output_clusters,
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    json!(devices)
}

fn main() {
    let mut database: Option<PathBuf> = None;
    let mut config: Option<PathBuf> = None;
    let mut rewrite: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--db" => {
                database = Some(PathBuf::from(
                    args.get(index + 1)
                        .unwrap_or_else(|| fail("--db needs a path")),
                ));
                index += 2;
            }
            "--config" => {
                config = Some(PathBuf::from(
                    args.get(index + 1)
                        .unwrap_or_else(|| fail("--config needs a path")),
                ));
                index += 2;
            }
            "--rewrite" => {
                rewrite = Some(PathBuf::from(
                    args.get(index + 1)
                        .unwrap_or_else(|| fail("--rewrite needs a path")),
                ));
                index += 2;
            }
            other => fail(format!("unknown argument '{other}'")),
        }
    }
    let database = database.unwrap_or_else(|| fail("--db is required"));
    let config = config.unwrap_or_else(|| fail("--config is required"));

    if let Some(output) = rewrite {
        let config_data = Config::load(&config)
            .unwrap_or_else(|error| fail(format!("cannot load config: {error}")));
        let (devices, _) = load_database(&database, &config_data.devices);
        std::fs::copy(&database, &output)
            .unwrap_or_else(|error| fail(format!("cannot copy database: {error}")));
        let mut persistent = PersistentDatabase::open(&output);
        persistent.sync_devices(&devices, None);
        persistent
            .write()
            .unwrap_or_else(|error| fail(format!("cannot write database: {error}")));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&normalize(&database, &config))
            .unwrap_or_else(|_| "[]".into())
    );
}
