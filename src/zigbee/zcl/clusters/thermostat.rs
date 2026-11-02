use super::super::attribute::AttributeReport;
use super::ClusterHandler;
use serde_json::{json, Value};

pub struct ThermostatCluster;

const LOCAL_TEMP: u16 = 0x0000;
const OUTDOOR_TEMP: u16 = 0x0001;
const OCCUPANCY: u16 = 0x0002;
const ABS_MIN_HEAT_SETPOINT: u16 = 0x0003;
const OCCUPIED_HEATING_SETPOINT: u16 = 0x0004;
const ABS_MAX_HEAT_SETPOINT: u16 = 0x0005;
const ABS_MIN_COOL_SETPOINT: u16 = 0x0006;
const ABS_MAX_COOL_SETPOINT: u16 = 0x0007;
const PI_HEATING_DEMAND: u16 = 0x0008;
const PI_COOLING_DEMAND: u16 = 0x0009;
const LOCAL_TEMP_CALIBRATION: u16 = 0x0010;
const SYSTEM_MODE: u16 = 0x0011;
const RUNNING_STATE: u16 = 0x0012;
const RUNNING_MODE: u16 = 0x0013;
const TEMP_SETPOINT_HOLD: u16 = 0x0015;
const TEMP_SETPOINT_HOLD_DURATION: u16 = 0x0016;
const MIN_HEAT_SETPOINT_LIMIT: u16 = 0x0017;
const MAX_HEAT_SETPOINT_LIMIT: u16 = 0x0018;
const MIN_COOL_SETPOINT_LIMIT: u16 = 0x0019;
const MAX_COOL_SETPOINT_LIMIT: u16 = 0x001A;
const MIN_SETPOINT_DEAD_BAND: u16 = 0x001B;
const UNOCCUPIED_HEATING_SETPOINT: u16 = 0x0023;
const UNOCCUPIED_COOLING_SETPOINT: u16 = 0x0024;
const OCCUPIED_COOLING_SETPOINT: u16 = 0x0025;
const SETPOINT_CHANGE_SOURCE: u16 = 0x0030;
const SETPOINT_CHANGE_AMOUNT: u16 = 0x0031;
const SETPOINT_CHANGE_SOURCE_TS: u16 = 0x0032;
const AC_LOUVER_POSITION: u16 = 0x0040;
const REMOTE_SENSING: u16 = 0x0041;
const CTRL_SEQ_OF_OPER: u16 = 0x0042;
const PROGRAMING_OPER_MODE: u16 = 0x0043;

const SENSOR_TYPE: u16 = 0xFF00;
const HYSTERESIS: u16 = 0xFF01;
const CORRECTION: u16 = 0xFF02;
const FAULT_CODE: u16 = 0xFF05;

const SENSOR_TYPES: [&str; 10] = [
    "3.3 кОм",
    "5 кОм",
    "6.8 кОм",
    "10 кОм",
    "12 кОм",
    "14.8 кОм",
    "15 кОм",
    "20 кОм",
    "33 кОм",
    "47 кОм",
];

const FAULT_CODES: [&str; 8] = [
    "Нет аварий",
    "Нагрев при выключенном реле",
    "Перегруз",
    "Превышение тока",
    "Обрыв нагревателя",
    "Ошибка датчика температуры",
    "Ток невалиден",
    "Деградация нагревателя",
];

const SYSTEM_MODES: [&str; 10] = [
    "off",
    "auto",
    "",
    "cool",
    "heat",
    "emergency_heating",
    "precooling",
    "fan_only",
    "dry",
    "sleep",
];

const RUNNING_MODES: [&str; 5] = ["off", "cool", "heat", "", "fan_only"];

const RUNNING_STATES: [&str; 7] = [
    "idle",
    "heat",
    "cool",
    "fan_only",
    "pending_heat",
    "pending_cool",
    "vent",
];

const CTRL_SEQS: [&str; 8] = [
    "cooling_only",
    "cooling_with_reheat",
    "heating_only",
    "heating_with_reheat",
    "cooling_and_heating_4_pipes",
    "cooling_and_heating_4_pipes_with_reheat",
    "",
    "",
];

const PROG_MODES: [&str; 4] = ["simple", "schedule", "economy", "away"];

const AC_LOUVER: [&str; 7] = [
    "fully_open",
    "quarter",
    "half",
    "three_quarters",
    "fully_closed",
    "",
    "",
];

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

impl ClusterHandler for ThermostatCluster {
    fn process_reports(&self, reports: &[AttributeReport]) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for r in reports {
            match r.attr_id {
                LOCAL_TEMP => {
                    if let Some(v) = r.value.as_f64() {
                        if v != -32768.0 {
                            let c = round2(v / 100.0);
                            if c >= -273.15 {
                                out.push(("local_temperature".into(), json!(c)));
                            }
                        }
                    }
                }
                OUTDOOR_TEMP => {
                    if let Some(v) = r.value.as_f64() {
                        if v != -32768.0 {
                            let c = round2(v / 100.0);
                            if c >= -273.15 {
                                out.push(("outdoor_temperature".into(), json!(c)));
                            }
                        }
                    }
                }
                OCCUPANCY => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("occupancy".into(), json!(v % 2.0 > 0.0)));
                    }
                }
                ABS_MIN_HEAT_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("abs_min_heat_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                OCCUPIED_HEATING_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("occupied_heating_setpoint".into(), json!(c)));
                        }
                    }
                }
                ABS_MAX_HEAT_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("abs_max_heat_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                ABS_MIN_COOL_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("abs_min_cool_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                ABS_MAX_COOL_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("abs_max_cool_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                PI_HEATING_DEMAND => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("pi_heating_demand".into(), json!(v)));
                    }
                }
                PI_COOLING_DEMAND => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("pi_cooling_demand".into(), json!(v)));
                    }
                }
                LOCAL_TEMP_CALIBRATION => {
                    if let Some(v) = r.value.as_f64() {
                        out.push((
                            "local_temperature_calibration".into(),
                            json!(round2(v / 10.0)),
                        ));
                    }
                }
                SYSTEM_MODE => {
                    if let Some(v) = r.value.as_f64() {
                        let mode = SYSTEM_MODES.get(v as usize).copied().unwrap_or("");
                        if !mode.is_empty() {
                            out.push(("system_mode".into(), json!(mode)));
                        }
                    }
                }
                RUNNING_STATE => {
                    if let Some(v) = r.value.as_f64() {
                        let state = RUNNING_STATES.get(v as usize).copied().unwrap_or("");
                        if !state.is_empty() {
                            out.push(("running_state".into(), json!(state)));
                        }
                    }
                }
                RUNNING_MODE => {
                    if let Some(v) = r.value.as_f64() {
                        let mode = RUNNING_MODES.get(v as usize).copied().unwrap_or("");
                        if !mode.is_empty() {
                            out.push(("running_mode".into(), json!(mode)));
                        }
                    }
                }
                TEMP_SETPOINT_HOLD => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("temperature_setpoint_hold".into(), json!(v != 0.0)));
                    }
                }
                TEMP_SETPOINT_HOLD_DURATION => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("temperature_setpoint_hold_duration".into(), json!(v)));
                    }
                }
                MIN_HEAT_SETPOINT_LIMIT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("min_heat_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                MAX_HEAT_SETPOINT_LIMIT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("max_heat_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                MIN_COOL_SETPOINT_LIMIT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("min_cool_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                MAX_COOL_SETPOINT_LIMIT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("max_cool_setpoint_limit".into(), json!(c)));
                        }
                    }
                }
                MIN_SETPOINT_DEAD_BAND => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("min_setpoint_dead_band".into(), json!(c)));
                        }
                    }
                }
                UNOCCUPIED_HEATING_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("unoccupied_heating_setpoint".into(), json!(c)));
                        }
                    }
                }
                UNOCCUPIED_COOLING_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("unoccupied_cooling_setpoint".into(), json!(c)));
                        }
                    }
                }
                OCCUPIED_COOLING_SETPOINT => {
                    if let Some(v) = r.value.as_f64() {
                        let c = round2(v / 100.0);
                        if c >= -273.15 {
                            out.push(("occupied_cooling_setpoint".into(), json!(c)));
                        }
                    }
                }
                SETPOINT_CHANGE_SOURCE => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("setpoint_change_source".into(), json!(v)));
                    }
                }
                SETPOINT_CHANGE_AMOUNT => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("setpoint_change_amount".into(), json!(round2(v / 100.0))));
                    }
                }
                SETPOINT_CHANGE_SOURCE_TS => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("setpoint_change_source_timestamp".into(), json!(v)));
                    }
                }
                AC_LOUVER_POSITION => {
                    if let Some(v) = r.value.as_f64() {
                        let name = AC_LOUVER.get(v as usize).copied().unwrap_or("");
                        if !name.is_empty() {
                            out.push(("ac_louver_position".into(), json!(name)));
                        }
                    }
                }
                REMOTE_SENSING => {
                    if let Some(v) = r.value.as_f64() {
                        let vv = v as u64;
                        out.push(("remote_sensing".into(), json!({
                            "local_temperature": if vv & 1 != 0 { "remotely" } else { "internally" },
                            "outdoor_temperature": if vv & 2 != 0 { "remotely" } else { "internally" },
                            "occupancy": if vv & 4 != 0 { "remotely" } else { "internally" },
                        })));
                    }
                }
                CTRL_SEQ_OF_OPER => {
                    if let Some(v) = r.value.as_f64() {
                        let name = CTRL_SEQS.get(v as usize).copied().unwrap_or("");
                        if !name.is_empty() {
                            out.push(("control_sequence_of_operation".into(), json!(name)));
                        }
                    }
                }
                PROGRAMING_OPER_MODE => {
                    if let Some(v) = r.value.as_f64() {
                        let name = PROG_MODES.get(v as usize).copied().unwrap_or("");
                        if !name.is_empty() {
                            out.push(("programming_operation_mode".into(), json!(name)));
                        }
                    }
                }
                SENSOR_TYPE => {
                    if let Some(v) = r.value.as_f64() {
                        let name = SENSOR_TYPES.get(v as usize).copied().unwrap_or("");
                        out.push((
                            "sensor_type".into(),
                            json!(if name.is_empty() {
                                v.to_string()
                            } else {
                                name.to_string()
                            }),
                        ));
                    }
                }
                HYSTERESIS => {
                    if let Some(v) = r.value.as_f64() {
                        out.push(("hysteresis".into(), json!(v / 10.0)));
                    }
                }
                CORRECTION => {
                    if let Some(v) = r.value.as_f64() {
                        let signed = if v > 127.0 { v - 256.0 } else { v };
                        out.push(("correction".into(), json!(signed / 10.0)));
                    }
                }
                FAULT_CODE => {
                    if let Some(v) = r.value.as_f64() {
                        let name = FAULT_CODES.get(v as usize).copied().unwrap_or("");
                        out.push((
                            "fault_reason".into(),
                            json!(if name.is_empty() {
                                v.to_string()
                            } else {
                                name.to_string()
                            }),
                        ));
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
    fn thermostat_local_temp() {
        let reports = vec![AttributeReport {
            attr_id: 0x0000,
            value: AttributeValue::I16(2250),
        }];
        let result = ThermostatCluster.process_reports(&reports);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "local_temperature");
    }
}
