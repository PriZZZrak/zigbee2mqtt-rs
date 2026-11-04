use std::path::PathBuf;

use clap::Parser;
use tracing::error;
use tracing_subscriber::{fmt, EnvFilter};

use zigbee2mqtt_rs::bridge::Bridge;
use zigbee2mqtt_rs::config::Config;

#[derive(Debug, Parser)]
#[command(name = "zigbee2mqtt-rs", about = "Zigbee to MQTT bridge")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "configuration.yaml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long)]
    log_level: Option<String>,

    /// Inspect configuration/database offline and print a JSON migration report
    #[arg(long)]
    inspect: bool,

    /// Create a timestamped rollback snapshot under this directory (outside the data directory)
    #[arg(long, value_name = "DIR")]
    snapshot_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.inspect || args.snapshot_dir.is_some() {
        let inspection = zigbee2mqtt_rs::migration::inspect(&args.config);
        let blocked = !inspection.blockers.is_empty();
        let snapshot = match args.snapshot_dir.as_deref() {
            Some(directory) => {
                match zigbee2mqtt_rs::migration::create_snapshot(&args.config, directory) {
                    Ok(snapshot) => Some(snapshot),
                    Err(error) => {
                        eprintln!("Snapshot failed: {error}");
                        std::process::exit(2);
                    }
                }
            }
            None => None,
        };
        let report = serde_json::json!({
            "inspection": inspection,
            "snapshot": snapshot,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|error| {
                eprintln!("Cannot serialize report: {error}");
                std::process::exit(2);
            })
        );
        if blocked {
            std::process::exit(1);
        }
        return;
    }

    let cfg = match Config::load(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config {}: {e}", args.config.display());
            std::process::exit(1);
        }
    };

    let log_level = args
        .log_level
        .as_deref()
        .or(Some(cfg.advanced.log_level.as_str()))
        .unwrap_or("info");

    let filter = EnvFilter::try_new(format!("zigbee2mqtt_rs={log_level},rumqttc=warn"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    tracing::info!(
        "zigbee2mqtt-rs v{} starting (serial={}, mqtt={}:{})",
        env!("CARGO_PKG_VERSION"),
        cfg.serial.port,
        cfg.mqtt.server,
        cfg.mqtt.port
    );

    let bridge = Bridge::new(cfg, args.config.clone());

    if let Err(e) = bridge.run().await {
        error!("Bridge error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::Parser;

    #[test]
    fn parses_inspect_and_snapshot_flags() {
        let args = Args::try_parse_from([
            "zigbee2mqtt-rs",
            "-c",
            "custom.yaml",
            "--inspect",
            "--snapshot-dir",
            "/tmp/snapshots",
        ])
        .unwrap();
        assert!(args.inspect);
        assert_eq!(
            args.snapshot_dir,
            Some(std::path::PathBuf::from("/tmp/snapshots"))
        );
    }

    #[test]
    fn snapshot_flag_requires_a_directory() {
        assert!(Args::try_parse_from(["zigbee2mqtt-rs", "--snapshot-dir"]).is_err());
    }
}
