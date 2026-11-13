// Original: .upstream/zigbee2mqtt/lib/extension/health.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Map, Value};
use zigbee_apiarist::utils::datetime::now_ms;

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::mqtt::MqttPublishOptions;
use crate::util::stringify::stringify;
use crate::util::system;

/// Upstream `round2` (`health.ts:9`).
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Upstream `round4` (`health.ts:11`).
fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

pub struct Health {
    context: ExtensionContext,
    check_timer: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Health {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        Arc::new(Self {
            context,
            check_timer: std::sync::Mutex::new(None),
        })
    }

    /// Upstream `clearStats()` (`health.ts:27-31`).
    pub fn clear_stats(&self) {
        let mut event_bus = self.context.event_bus.lock().unwrap();
        event_bus.stats.devices.clear();
        event_bus.stats.mqtt.published = 0;
        event_bus.stats.mqtt.received = 0;
    }

    /// Upstream `#checkHealth()` (`health.ts:33-77`).
    pub async fn check_health(&self) {
        let (total_kb, free_kb) = system::memory_kb();
        let used_kb = total_kb - free_kb;
        let process_kb = system::process_rss_kb();

        let (mqtt_stats, devices, reset_on_check) = {
            let event_bus = self.context.event_bus.lock().unwrap();
            let mut devices = Map::new();
            for (ieee, device) in &event_bus.stats.devices {
                let mut messages = 0u64;
                let mut mps = 0.0;
                if let Some(changes) = &device.last_seen_changes {
                    let time_diff = now_ms().saturating_sub(changes.first);
                    messages = changes.messages;
                    mps = if time_diff > 0 {
                        round4(messages as f64 / (time_diff as f64 / 1000.0))
                    } else {
                        0.0
                    };
                }
                devices.insert(
                    ieee.clone(),
                    json!({
                        "messages": messages,
                        "messages_per_sec": mps,
                        "leave_count": device.leave_counts,
                        "network_address_changes": device.network_address_changes,
                    }),
                );
            }
            (
                json!({
                    "published": event_bus.stats.mqtt.published,
                    "received": event_bus.stats.mqtt.received,
                }),
                Value::Object(devices),
                self.context
                    .settings_value()
                    .get("health")
                    .and_then(|health| health.get("reset_on_check"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )
        };

        let mqtt = {
            let mqtt = self.context.mqtt.lock().await;
            mqtt.stats()
        };
        let mut mqtt = mqtt.as_object().cloned().unwrap_or_default();
        if let Value::Object(stats) = mqtt_stats {
            for (key, value) in stats {
                mqtt.insert(key, value);
            }
        }

        let healthcheck = json!({
            "response_time": now_ms(),
            "os": {
                "load_average": system::load_average(),
                "memory_used_mb": round2(used_kb / 1024.0),
                "memory_percent": round4((used_kb / total_kb) * 100.0),
            },
            "process": {
                "uptime_sec": system::process_uptime_secs(),
                "memory_used_mb": round2(process_kb / 1024.0),
                "memory_percent": round4((process_kb / total_kb) * 100.0),
            },
            "mqtt": Value::Object(mqtt),
            "devices": devices,
        });

        if reset_on_check {
            self.clear_stats();
        }

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/health",
                stringify(&healthcheck),
                MqttPublishOptions {
                    retain: true,
                    qos: 1,
                    ..Default::default()
                },
            )
            .await;
    }
}

impl Extension for Health {
    fn name(&self) -> &'static str {
        "Health"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let interval = self
                .context
                .settings_value()
                .get("health")
                .and_then(|health| health.get("interval"))
                .and_then(Value::as_u64)
                .unwrap_or(10);
            let this = self.clone();
            let handle = tokio::spawn(async move {
                let mut ticker =
                    tokio::time::interval(Duration::from_secs(interval.saturating_mul(60)));
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    this.check_health().await;
                }
            });
            *self.check_timer.lock().unwrap() = Some(handle);
            Ok(())
        })
    }

    fn stop(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            if let Some(timer) = self.check_timer.lock().unwrap().take() {
                timer.abort();
            }
            self.context
                .event_bus
                .lock()
                .unwrap()
                .remove_listeners(self.context.key);
            Ok(())
        })
    }
}
