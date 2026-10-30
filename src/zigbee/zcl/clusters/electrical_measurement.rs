use serde_json::{json, Value};
use super::super::attribute::AttributeReport;
use super::ClusterHandler;

pub struct ElectricalMeasurementCluster;

const RMS_CURRENT: u16 = 0x0508;
const APPARENT_POWER: u16 = 0x050F;
const ALARMS_MASK: u16 = 0x0800;

const FAULT_BITS: [(&str, u64); 7] = [
    ("fault_heating_while_off", 0x01),
    ("fault_heater_open", 0x02),
    ("fault_overcurrent", 0x04),
    ("fault_element_degraded", 0x08),
    ("fault_sensor", 0x10),
    ("fault_current_invalid", 0x20),
    ("fault_overload_trip", 0x40),
];

impl ClusterHandler for ElectricalMeasurementCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            match r.attr_id {
                RMS_CURRENT => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("current".into(), json!(v / 1000.0)));
                    }
                }
                APPARENT_POWER => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("power".into(), json!(v)));
                    }
                }
                ALARMS_MASK => {
                    if let Some(v) = r.value.as_f64() {
                        let mask = v as u64;
                        out.push(("fault_mask".into(), json!(mask)));
                        for (name, bit) in FAULT_BITS.iter() {
                            out.push((name.to_string(), json!(if mask & bit != 0 { "ON" } else { "OFF" })));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zigbee::zcl::attribute::{AttributeReport, AttributeValue};

    #[test]
    fn em_current() {
        let reports = vec![AttributeReport {
            attr_id: 0x0508,
            value: AttributeValue::U16(1500),
        }];
        let result = ElectricalMeasurementCluster.process_reports(&reports);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "current");
    }

    #[test]
    fn em_alarms() {
        let reports = vec![AttributeReport {
            attr_id: 0x0800,
            value: AttributeValue::U16(0x05),
        }];
        let result = ElectricalMeasurementCluster.process_reports(&reports);
        assert!(result.iter().any(|(k, _)| k == "fault_mask"));
        assert!(result.iter().any(|(k, _)| k == "fault_heating_while_off"));
    }
}