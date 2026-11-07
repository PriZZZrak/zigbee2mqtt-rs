// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/cluster.ts
// TODO: port the full cluster/command/parameter catalog.
//
// Only the clusters/commands/parameters referenced by the ported frame tests
// are modelled here. Public names mirror upstream (`genAlarms`, `commands`,
// `commandsResponse`, `ID`, `name`, `type`, `conditions`, `max`).

use crate::zspec::zcl::definition::enums::DataType;

/// Upstream `ParameterCondition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterCondition {
    /// `{type: ParameterCondition.MINIMUM_REMAINING_BUFFER_BYTES, value}`
    MINIMUM_REMAINING_BUFFER_BYTES(u32),
    /// `{type: ParameterCondition.BITMASK_SET, field, value}`
    BITMASK_SET { field: &'static str, value: u32 },
    /// `{type: ParameterCondition.BITFIELD_ENUM, field, offset, size, value}`
    BITFIELD_ENUM {
        field: &'static str,
        offset: u8,
        size: u8,
        value: u32,
    },
    /// `{type: ParameterCondition.FIELD_EQUAL, field, value, reversed}`
    FIELD_EQUAL {
        field: &'static str,
        value: u32,
        reversed: bool,
    },
}

/// Upstream `ParameterDefinition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameter {
    pub name: &'static str,
    /// Upstream camelCase: `type`.
    pub r#type: DataType,
    pub conditions: &'static [ParameterCondition],
    pub max: Option<u32>,
}

impl Parameter {
    pub const fn new(name: &'static str, r#type: DataType) -> Self {
        Self {
            name,
            r#type,
            conditions: &[],
            max: None,
        }
    }
}

/// Upstream `AttributeDefinition` (subset used by the ported tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute {
    pub name: &'static str,
    /// Upstream camelCase: `ID`.
    pub ID: u16,
    /// Upstream camelCase: `type`.
    pub r#type: DataType,
}

impl Attribute {
    pub const fn new(name: &'static str, ID: u16, r#type: DataType) -> Self {
        Self { name, ID, r#type }
    }
}

/// Upstream `CommandDefinition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub name: &'static str,
    /// Upstream camelCase: `ID`.
    pub ID: u8,
    pub response: Option<u8>,
    pub parameters: &'static [Parameter],
    pub required: bool,
}

impl Command {
    pub const fn new(name: &'static str, ID: u8) -> Self {
        Self {
            name,
            ID,
            response: None,
            parameters: &[],
            required: true,
        }
    }

    pub const fn with_response(self, response: u8) -> Self {
        Self {
            response: Some(response),
            ..self
        }
    }

    pub const fn with_parameters(self, parameters: &'static [Parameter]) -> Self {
        Self { parameters, ..self }
    }
}

/// Upstream `ClusterDefinition` (only the subset used by tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cluster {
    pub name: &'static str,
    /// Upstream camelCase: `ID`.
    pub ID: u16,
}

#[allow(non_snake_case)]
pub mod genBasic {
    use super::{Attribute, Command, DataType};

    pub const ID: u16 = 0x0000;
    pub const name: &str = "genBasic";

    pub mod attributes {
        use super::*;

        pub const zclVersion: Attribute = Attribute::new("zclVersion", 0x0000, DataType::UINT8);
        pub const manufacturerName: Attribute =
            Attribute::new("manufacturerName", 0x0004, DataType::CHAR_STR);
        pub const modelId: Attribute = Attribute::new("modelId", 0x0005, DataType::CHAR_STR);
        pub const powerSource: Attribute = Attribute::new("powerSource", 0x0007, DataType::ENUM8);
        pub const physicalEnv: Attribute = Attribute::new("physicalEnv", 0x0011, DataType::ENUM8);
        pub const alarmMask: Attribute = Attribute::new("alarmMask", 0x0013, DataType::BITMAP8);
    }

    pub mod commands {
        use super::*;

        pub const resetFactDefault: Command = Command::new("resetFactDefault", 0x00);
    }
}

#[allow(non_snake_case)]
pub mod genPowerCfg {
    pub const ID: u16 = 0x0001;
    pub const name: &str = "genPowerCfg";
}

#[allow(non_snake_case)]
pub mod genOnOff {
    use super::{Attribute, Command, DataType};

    pub const ID: u16 = 0x0006;
    pub const name: &str = "genOnOff";

    pub mod attributes {
        use super::*;

        pub const onOff: Attribute = Attribute::new("onOff", 0x0000, DataType::BOOLEAN);
        pub const globalSceneCtrl: Attribute =
            Attribute::new("globalSceneCtrl", 0x4000, DataType::BOOLEAN);
        pub const onTime: Attribute = Attribute::new("onTime", 0x4001, DataType::UINT16);
        pub const offWaitTime: Attribute = Attribute::new("offWaitTime", 0x4002, DataType::UINT16);
        pub const startUpOnOff: Attribute = Attribute::new("startUpOnOff", 0x4003, DataType::ENUM8);
    }

    pub mod commands {
        use super::*;

        pub const off: Command = Command::new("off", 0x00);
        pub const on: Command = Command::new("on", 0x01);
    }
}

#[allow(non_snake_case)]
pub mod haApplianceIdentification {
    pub const ID: u16 = 0x0b00;
    pub const name: &str = "haApplianceIdentification";
}

#[allow(non_snake_case)]
pub mod genGroups {
    use super::{Command, DataType, Parameter};

    pub const ID: u16 = 0x0004;
    pub const name: &str = "genGroups";

    pub mod commands {
        use super::*;

        pub const PARAMS: &[Parameter] = &[
            Parameter::new("groupcount", DataType::UINT8),
            // upstream: BuffaloZclDataType.LIST_UINT16 (1002)
            Parameter::new("grouplist", DataType::UNKNOWN),
        ];

        pub const getMembership: Command = Command::new("getMembership", 2).with_parameters(PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod genAlarms {
    use super::{Command, DataType, Parameter};

    pub const ID: u16 = 0x0009;
    pub const name: &str = "genAlarms";

    pub mod commands {
        use super::*;

        pub const getAlarm: Command = Command::new("getAlarm", 2);
        pub const reset: Command = Command::new("reset", 0);
    }

    pub mod commandsResponse {
        use super::*;

        pub const PARAMS: &[Parameter] = &[
            Parameter::new("alarmcode", DataType::ENUM8),
            Parameter::new("clusterid", DataType::UINT16),
        ];

        pub const alarm: Command = Command::new("alarm", 0).with_parameters(PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod genLevelCtrl {
    use super::{Command, DataType, Parameter, ParameterCondition};

    pub const ID: u16 = 0x0008;
    pub const name: &str = "genLevelCtrl";

    pub mod commands {
        use super::*;

        const OPTIONS_CONDITIONS: &[ParameterCondition] =
            &[ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(2)];

        const OPTIONS_MASK: Parameter = Parameter {
            name: "optionsMask",
            r#type: DataType::BITMAP8,
            conditions: OPTIONS_CONDITIONS,
            max: None,
        };
        const OPTIONS_OVERRIDE: Parameter = Parameter {
            name: "optionsOverride",
            r#type: DataType::BITMAP8,
            conditions: OPTIONS_CONDITIONS,
            max: None,
        };

        pub const PARAMS: &[Parameter] = &[
            Parameter::new("level", DataType::UINT8),
            Parameter::new("transtime", DataType::UINT16),
            OPTIONS_MASK,
            OPTIONS_OVERRIDE,
        ];

        pub const moveToLevelWithOnOff: Command =
            Command::new("moveToLevelWithOnOff", 4).with_parameters(PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod genOta {
    use super::{Command, DataType, Parameter, ParameterCondition};

    pub const ID: u16 = 0x0019;
    pub const name: &str = "genOta";

    pub mod commands {
        use super::*;

        pub const PARAMS: &[Parameter] = &[
            Parameter::new("fieldControl", DataType::BITMAP8),
            Parameter::new("manufacturerCode", DataType::UINT16),
            Parameter::new("imageType", DataType::UINT16),
            Parameter::new("fileVersion", DataType::UINT32),
            Parameter::new("fileOffset", DataType::UINT32),
            Parameter::new("maximumDataSize", DataType::UINT8),
            Parameter::new("minimumBlockPeriod", DataType::UINT16),
        ];

        pub const imageBlockRequest: Command =
            Command::new("imageBlockRequest", 3).with_parameters(PARAMS);
    }

    pub mod commandsResponse {
        use super::*;

        const STATUS_SUCCESS: ParameterCondition = ParameterCondition::FIELD_EQUAL {
            field: "status",
            value: 0,
            reversed: false,
        };
        const STATUS_SUCCESS_CONDITIONS: &[ParameterCondition] = &[STATUS_SUCCESS];

        const STATUS_PARAM: Parameter = Parameter::new("status", DataType::ENUM8);
        const MANUFACTURER_CODE: Parameter = Parameter {
            name: "manufacturerCode",
            r#type: DataType::UINT16,
            conditions: STATUS_SUCCESS_CONDITIONS,
            max: None,
        };
        const IMAGE_TYPE: Parameter = Parameter {
            name: "imageType",
            r#type: DataType::UINT16,
            conditions: STATUS_SUCCESS_CONDITIONS,
            max: Some(65471),
        };
        const FILE_VERSION: Parameter = Parameter {
            name: "fileVersion",
            r#type: DataType::UINT32,
            conditions: STATUS_SUCCESS_CONDITIONS,
            max: None,
        };
        const IMAGE_SIZE: Parameter = Parameter {
            name: "imageSize",
            r#type: DataType::UINT32,
            conditions: STATUS_SUCCESS_CONDITIONS,
            max: None,
        };

        pub const QUERY_NEXT_IMAGE_RSP_PARAMS: &[Parameter] = &[
            STATUS_PARAM,
            MANUFACTURER_CODE,
            IMAGE_TYPE,
            FILE_VERSION,
            IMAGE_SIZE,
        ];

        pub const queryNextImageResponse: Command =
            Command::new("queryNextImageResponse", 2).with_parameters(QUERY_NEXT_IMAGE_RSP_PARAMS);

        const PAYLOAD_TYPE_1: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
            field: "payloadType",
            value: 1,
            reversed: false,
        }];
        const PAYLOAD_TYPE_2: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
            field: "payloadType",
            value: 2,
            reversed: false,
        }];
        const PAYLOAD_TYPE_3: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
            field: "payloadType",
            value: 3,
            reversed: false,
        }];

        const PAYLOAD_TYPE: Parameter = Parameter::new("payloadType", DataType::ENUM8);
        const QUERY_JITTER: Parameter = Parameter::new("queryJitter", DataType::UINT8);
        const IMAGE_NOTIFY_MANUFACTURER_CODE: Parameter = Parameter {
            name: "manufacturerCode",
            r#type: DataType::UINT16,
            conditions: PAYLOAD_TYPE_1,
            max: None,
        };
        const IMAGE_NOTIFY_IMAGE_TYPE: Parameter = Parameter {
            name: "imageType",
            r#type: DataType::UINT16,
            conditions: PAYLOAD_TYPE_2,
            max: None,
        };
        const IMAGE_NOTIFY_FILE_VERSION: Parameter = Parameter {
            name: "newFileVersion",
            r#type: DataType::UINT32,
            conditions: PAYLOAD_TYPE_3,
            max: None,
        };

        pub const IMAGE_NOTIFY_PARAMS: &[Parameter] = &[
            PAYLOAD_TYPE,
            QUERY_JITTER,
            IMAGE_NOTIFY_MANUFACTURER_CODE,
            IMAGE_NOTIFY_IMAGE_TYPE,
            IMAGE_NOTIFY_FILE_VERSION,
        ];

        pub const imageNotify: Command =
            Command::new("imageNotify", 0).with_parameters(IMAGE_NOTIFY_PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod touchlink {
    use super::{Command, DataType, Parameter, ParameterCondition};

    pub const ID: u16 = 0x1000;
    pub const name: &str = "touchlink";

    pub mod commandsResponse {
        use super::*;

        const NO_SUB_DEVICE: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
            field: "numberOfSubDevices",
            value: 1,
            reversed: false,
        }];

        const TRANSACTION_ID: Parameter = Parameter::new("transactionID", DataType::UINT32);
        const RSSI_CORRECTION: Parameter = Parameter::new("rssiCorrection", DataType::UINT8);
        const ZIGBEE_INFORMATION: Parameter =
            Parameter::new("zigbeeInformation", DataType::BITMAP8);
        const TOUCHLINK_INFORMATION: Parameter =
            Parameter::new("touchlinkInformation", DataType::BITMAP8);
        const KEY_BITMASK: Parameter = Parameter::new("keyBitmask", DataType::BITMAP16);
        const RESPONSE_ID: Parameter = Parameter::new("responseID", DataType::UINT32);
        const EXTENDED_PAN_ID: Parameter = Parameter::new("extendedPanID", DataType::IEEE_ADDR);
        const NETWORK_UPDATE_ID: Parameter = Parameter::new("networkUpdateID", DataType::UINT8);
        const LOGICAL_CHANNEL: Parameter = Parameter::new("logicalChannel", DataType::UINT8);
        const PAN_ID: Parameter = Parameter::new("panID", DataType::UINT16);
        const NETWORK_ADDRESS: Parameter = Parameter::new("networkAddress", DataType::UINT16);
        const NUMBER_OF_SUB_DEVICES: Parameter =
            Parameter::new("numberOfSubDevices", DataType::UINT8);
        const TOTAL_GROUP_IDENTIFIERS: Parameter =
            Parameter::new("totalGroupIdentifiers", DataType::UINT8);
        const ENDPOINT_ID: Parameter = Parameter {
            name: "endpointID",
            r#type: DataType::UINT8,
            conditions: NO_SUB_DEVICE,
            max: None,
        };
        const PROFILE_ID: Parameter = Parameter {
            name: "profileID",
            r#type: DataType::UINT16,
            conditions: NO_SUB_DEVICE,
            max: None,
        };
        const DEVICE_ID: Parameter = Parameter {
            name: "deviceID",
            r#type: DataType::UINT16,
            conditions: NO_SUB_DEVICE,
            max: None,
        };
        const VERSION: Parameter = Parameter {
            name: "version",
            r#type: DataType::UINT8,
            conditions: NO_SUB_DEVICE,
            max: None,
        };
        const GROUP_ID_COUNT: Parameter = Parameter {
            name: "groupIDCount",
            r#type: DataType::UINT8,
            conditions: NO_SUB_DEVICE,
            max: None,
        };

        pub const SCAN_RSP_PARAMS: &[Parameter] = &[
            TRANSACTION_ID,
            RSSI_CORRECTION,
            ZIGBEE_INFORMATION,
            TOUCHLINK_INFORMATION,
            KEY_BITMASK,
            RESPONSE_ID,
            EXTENDED_PAN_ID,
            NETWORK_UPDATE_ID,
            LOGICAL_CHANNEL,
            PAN_ID,
            NETWORK_ADDRESS,
            NUMBER_OF_SUB_DEVICES,
            TOTAL_GROUP_IDENTIFIERS,
            ENDPOINT_ID,
            PROFILE_ID,
            DEVICE_ID,
            VERSION,
            GROUP_ID_COUNT,
        ];

        pub const scanResponse: Command =
            Command::new("scanResponse", 1).with_parameters(SCAN_RSP_PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod greenPower {
    use super::{Command, DataType, Parameter, ParameterCondition};

    pub const ID: u16 = 0x0021;
    pub const name: &str = "greenPower";

    pub mod commands {
        use super::*;

        const OPTIONS_BITFIELD: ParameterCondition = ParameterCondition::BITFIELD_ENUM {
            field: "options",
            offset: 0,
            size: 3,
            value: 0b000,
        };
        const OPTIONS_BITMASK: ParameterCondition = ParameterCondition::BITMASK_SET {
            field: "options",
            value: 0x4000,
        };
        const OPTIONS_BITFIELD_CONDITIONS: &[ParameterCondition] = &[OPTIONS_BITFIELD];
        const OPTIONS_BITMASK_CONDITIONS: &[ParameterCondition] = &[OPTIONS_BITMASK];

        const RP: Parameter = Parameter::new("reserved", DataType::DATA8);
        const OPTIONS_BITFIELD_PARAM: Parameter = Parameter {
            name: "options",
            r#type: DataType::BITMAP8,
            conditions: OPTIONS_BITFIELD_CONDITIONS,
            max: None,
        };
        const OPTIONS_BITMASK_PARAM: Parameter = Parameter {
            name: "options",
            r#type: DataType::BITMAP24,
            conditions: OPTIONS_BITMASK_CONDITIONS,
            max: None,
        };

        pub const NOTIFICATION_PARAMS: &[Parameter] = &[
            RP,
            OPTIONS_BITFIELD_PARAM,
            RP,
            RP,
            RP,
            RP,
            RP,
            RP,
            OPTIONS_BITMASK_PARAM,
        ];

        pub const notification: Command =
            Command::new("notification", 0).with_parameters(NOTIFICATION_PARAMS);
    }
}

#[allow(non_snake_case)]
pub mod genTime {
    pub const ID: u16 = 0x000a;
    pub const name: &str = "genTime";
}

#[allow(non_snake_case)]
pub mod ssIasZone {
    pub const ID: u16 = 0x0500;
    pub const name: &str = "ssIasZone";
}

#[allow(non_snake_case)]
pub mod genPollCtrl {
    pub const ID: u16 = 0x0020;
    pub const name: &str = "genPollCtrl";
}

#[allow(non_snake_case)]
pub mod genScenes {
    pub const ID: u16 = 0x0005;
    pub const name: &str = "genScenes";
}
