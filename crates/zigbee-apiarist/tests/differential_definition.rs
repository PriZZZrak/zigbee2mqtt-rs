// Differential fixture generated from zigbee-herdsman v11.2.0:
// tools/oracle/gen-definition.mjs -> fixtures/definition.json.

use serde::Deserialize;
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::znp::definition::{
    find_by_id, find_by_name, for_subsystem, MtParameter,
};
use zigbee_apiarist::adapter::z_stack::znp::parameter_type::ParameterType;

#[derive(Debug, Deserialize)]
struct Parameter {
    name: String,
    #[serde(rename = "parameterType")]
    parameter_type: String,
}

#[derive(Debug, Deserialize)]
struct Command {
    subsystem: String,
    name: String,
    id: u8,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "zdoClusterId")]
    zdo_cluster_id: Option<u16>,
    request: Vec<Parameter>,
    response: Vec<Parameter>,
}

fn subsystem(name: &str) -> Subsystem {
    match name {
        "RESERVED" => Subsystem::RESERVED,
        "SYS" => Subsystem::SYS,
        "MAC" => Subsystem::MAC,
        "NWK" => Subsystem::NWK,
        "AF" => Subsystem::AF,
        "ZDO" => Subsystem::ZDO,
        "SAPI" => Subsystem::SAPI,
        "UTIL" => Subsystem::UTIL,
        "DEBUG" => Subsystem::DEBUG,
        "APP" => Subsystem::APP,
        "APP_CNF" => Subsystem::APP_CNF,
        "GREENPOWER" => Subsystem::GREENPOWER,
        other => panic!("unknown subsystem '{other}'"),
    }
}

fn kind(name: &str) -> Type {
    match name {
        "POLL" => Type::POLL,
        "SREQ" => Type::SREQ,
        "AREQ" => Type::AREQ,
        "SRSP" => Type::SRSP,
        other => panic!("unknown type '{other}'"),
    }
}

fn parameter_type(name: &str) -> u32 {
    match name {
        "UINT8" => ParameterType::UINT8 as u32,
        "UINT16" => ParameterType::UINT16 as u32,
        "UINT32" => ParameterType::UINT32 as u32,
        "IEEEADDR" => ParameterType::IEEEADDR as u32,
        "BUFFER" => ParameterType::BUFFER as u32,
        "BUFFER8" => ParameterType::BUFFER8 as u32,
        "BUFFER16" => ParameterType::BUFFER16 as u32,
        "BUFFER18" => ParameterType::BUFFER18 as u32,
        "BUFFER32" => ParameterType::BUFFER32 as u32,
        "BUFFER42" => ParameterType::BUFFER42 as u32,
        "BUFFER100" => ParameterType::BUFFER100 as u32,
        "LIST_UINT8" => ParameterType::LIST_UINT8 as u32,
        "LIST_UINT16" => ParameterType::LIST_UINT16 as u32,
        "LIST_NETWORK" => ParameterType::LIST_NETWORK as u32,
        "INT8" => ParameterType::INT8 as u32,
        other => panic!("unknown parameter type '{other}'"),
    }
}

fn expected_parameters(parameters: &[Parameter]) -> Vec<MtParameter> {
    parameters
        .iter()
        .map(|parameter| MtParameter {
            name: Box::leak(parameter.name.clone().into_boxed_str()),
            parameter_type: parameter_type(&parameter.parameter_type),
        })
        .collect()
}

#[test]
fn definition_matches_upstream_fixtures() {
    let commands: Vec<Command> = serde_json::from_str(include_str!("fixtures/definition.json"))
        .expect("definition fixtures");

    for command in &commands {
        let subsystem = subsystem(&command.subsystem);
        let by_name = find_by_name(subsystem, &command.name).expect("command not found by name");
        assert_eq!(by_name.id, command.id, "{}: id", command.name);
        assert_eq!(
            by_name.r#type,
            kind(&command.kind),
            "{}: type",
            command.name
        );
        assert_eq!(
            by_name.zdo_cluster_id, command.zdo_cluster_id,
            "{}: zdoClusterId",
            command.name
        );
        assert_eq!(
            by_name.request,
            expected_parameters(&command.request).as_slice(),
            "{}: request",
            command.name
        );
        assert_eq!(
            by_name.response,
            expected_parameters(&command.response).as_slice(),
            "{}: response",
            command.name
        );

        let by_id = find_by_id(subsystem, command.id).expect("command not found by id");
        assert_eq!(by_id.name, command.name, "{}: id lookup name", command.name);
    }

    for expected_subsystem in [
        Subsystem::RESERVED,
        Subsystem::SYS,
        Subsystem::MAC,
        Subsystem::NWK,
        Subsystem::AF,
        Subsystem::ZDO,
        Subsystem::SAPI,
        Subsystem::UTIL,
        Subsystem::DEBUG,
        Subsystem::APP,
        Subsystem::APP_CNF,
        Subsystem::GREENPOWER,
    ] {
        let expected = commands
            .iter()
            .filter(|command| subsystem(&command.subsystem) == expected_subsystem)
            .count();
        assert_eq!(
            for_subsystem(expected_subsystem).len(),
            expected,
            "{}: command count",
            expected_subsystem.name()
        );
    }
}
