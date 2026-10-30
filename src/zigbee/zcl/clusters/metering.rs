use serde_json::{json, Value};
use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct MeteringCluster;

const CURRENT_SUMM_DELIVERED: u16 = 0x0000;

impl ClusterHandler for MeteringCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            if r.attr_id == CURRENT_SUMM_DELIVERED {
                if let Some(v) = r.value.as_f64() {
                    out.push(("energy".into(), json!(v / 1000.0)));
                }
            }
        }
        out
    }
}