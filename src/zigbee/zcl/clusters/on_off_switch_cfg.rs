use serde_json::{json, Value};
use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct OnOffSwitchCfgCluster;

const SWITCH_TYPE: u16 = 0x0000;
const DECOUPLED: u16 = 0xFF10;

const CONTACT_TYPES: [&str; 3] = ["Кнопка", "Выключатель", "Переключатель"];

impl ClusterHandler for OnOffSwitchCfgCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            match r.attr_id {
                SWITCH_TYPE => {
                    if let Some(v) = r.value.as_f64() {
                        let name = CONTACT_TYPES.get(v as usize).copied().unwrap_or("");
                        out.push(("switch_type".into(), json!(if name.is_empty() { v.to_string() } else { name.to_string() })));
                    }
                }
                DECOUPLED => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("decoupled".into(), json!(if v != 0.0 { "ON" } else { "OFF" })));
                    }
                }
                _ => {}
            }
        }
        out
    }
}