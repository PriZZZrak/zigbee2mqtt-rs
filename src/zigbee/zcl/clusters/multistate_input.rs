use serde_json::{json, Value};
use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct MultistateInputCluster;

const PRESENT_VALUE: u16 = 0x0055;

const ACTION_CODES: [&str; 6] = ["", "single", "double", "hold", "press", "release"];

impl ClusterHandler for MultistateInputCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            if r.attr_id == PRESENT_VALUE {
                if let Some(v) = r.value.as_f64() {
                    let code = v as usize;
                    if code > 0 && code < ACTION_CODES.len() {
                        out.push(("action".into(), json!(ACTION_CODES[code])));
                    }
                }
            }
        }
        out
    }
}