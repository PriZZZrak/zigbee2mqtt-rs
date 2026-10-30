use serde_json::{json, Value};
use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct BinaryInputCluster;

const PRESENT_VALUE: u16 = 0x0055;

impl ClusterHandler for BinaryInputCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            if r.attr_id == PRESENT_VALUE {
                if let Some(v) = r.value.as_f64() {
                    let state = if v != 0.0 { "closed" } else { "open" };
                    out.push(("contact".into(), json!(state)));
                }
            }
        }
        out
    }
}