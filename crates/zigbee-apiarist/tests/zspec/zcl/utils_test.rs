// Original: .upstream/zigbee-herdsman/test/zspec/zcl/utils.test.ts

use std::panic::{catch_unwind, AssertUnwindSafe};

use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
use zigbee_apiarist::zspec::zcl::definition::status::Status;
use zigbee_apiarist::zspec::zcl::utils::manufacturer_code;
use zigbee_apiarist::zspec::zcl::utils::{
    Attribute, Cluster, CustomClusters, Parameter, Utils, ZclValue,
};
use zigbee_apiarist::zspec::zcl::zcl_status_error::ZclStatusError;

/// Upstream `expect(...).toThrow(...)`.
fn assert_panics(function: impl FnOnce()) {
    assert!(catch_unwind(AssertUnwindSafe(function)).is_err());
}

/// Upstream `createAttribute()`.
fn create_attribute() -> Attribute {
    Attribute::new("testAttr", 0x0001, DataType::UINT8)
}

/// Upstream `createParameter()`.
fn create_parameter() -> Parameter {
    Parameter::new("testParam", DataType::UINT8)
}

/// Upstream `CUSTOM_CLUSTERS`.
fn custom_clusters() -> CustomClusters {
    let mut clusters = CustomClusters::empty();
    let my_custom_attr = Attribute::new("myCustomAttr", 65533, DataType::UINT8);

    clusters
        .genBasic
        .attributes
        .insert("myCustomAttr".to_string(), my_custom_attr.clone());
    clusters
        .myCustomCluster
        .attributes
        .insert("myCustomAttr".to_string(), my_custom_attr.clone());
    clusters
        .myCustomClusterManuf
        .attributes
        .insert("myCustomAttr".to_string(), my_custom_attr);
    clusters.myCustomClusterManuf.manufacturerCode = Some(65534);

    let mut danfoss = Attribute::new("danfossSystemStatusCode", 65530, DataType::UINT8);
    danfoss.manufacturerCode = Some(manufacturer_code::DANFOSS_A_S);
    clusters
        .myCustomWithManufAttr
        .attributes
        .insert("danfossSystemStatusCode".to_string(), danfoss);

    clusters
}

fn assert_cluster(cluster: &Cluster, expected: &Cluster, name: &str) {
    assert_eq!(cluster.ID, expected.ID);
    assert_eq!(cluster.name, name);
    assert_eq!(cluster.manufacturerCode, expected.manufacturerCode);

    for (key, attribute) in &expected.attributes {
        let actual = cluster
            .attributes
            .get(key)
            .unwrap_or_else(|| panic!("missing attribute {key}"));
        assert_eq!(&actual.name, key);
        assert_eq!(actual, attribute);
    }
}

#[test]
#[ignore = "not implemented yet"]
fn creates_status_error() {
    // upstream: `new Zcl.StatusError(Zcl.Status.ABORT)`
    let zcl_error = ZclStatusError::new(Status::ABORT);

    assert_eq!(zcl_error.code, Status::ABORT);
    assert_eq!(zcl_error.message(), "Status 'ABORT'");
}

#[test]
#[ignore = "not implemented yet"]
fn checks_analog_data_type() {
    // upstream: `Zcl.Utils.isAnalogDataType(type)`
    assert!(Utils::is_analog_data_type(DataType::UINT16));
    assert!(!Utils::is_analog_data_type(DataType::DATA16));
    assert!(!Utils::is_analog_data_type(DataType::NO_DATA));
}

#[test]
#[ignore = "not implemented yet"]
fn gets_cluster() {
    // upstream: `Zcl.Utils.getCluster(key, manufacturerCode, customClusters)`
    let empty = CustomClusters::empty();
    let customs = custom_clusters();

    // by ID
    let cluster = Utils::get_cluster(0x0000, None, &empty);
    assert_eq!(cluster.ID, 0x0000);
    assert_eq!(cluster.name, "genBasic");
    assert_eq!(cluster.manufacturerCode, None);

    // by name
    let cluster = Utils::get_cluster("genAlarms", None, &empty);
    assert_eq!(cluster.ID, 0x0009);
    assert_eq!(cluster.name, "genAlarms");

    // by ID with no manufacturer code
    let cluster = Utils::get_cluster(0x0009, Some(123), &empty);
    assert_eq!(cluster.name, "genAlarms");

    // by ID with non-matching manufacturer code
    let cluster = Utils::get_cluster(0xfc57, Some(123), &empty);
    assert_eq!(cluster.name, "manuSpecificAmazonWWAH");
    assert_eq!(
        cluster.manufacturerCode,
        Some(manufacturer_code::AMAZON_LAB126)
    );

    // by ID with matching manufacturer code
    let cluster = Utils::get_cluster(0xfc57, Some(manufacturer_code::AMAZON_LAB126), &empty);
    assert_eq!(cluster.name, "manuSpecificAmazonWWAH");
    assert_eq!(
        cluster.manufacturerCode,
        Some(manufacturer_code::AMAZON_LAB126)
    );

    // custom by ID
    let cluster = Utils::get_cluster(65534, None, &customs);
    assert_cluster(&cluster, &customs.myCustomCluster, "myCustomCluster");

    // custom by name
    let cluster = Utils::get_cluster("myCustomCluster", None, &customs);
    assert_cluster(&cluster, &customs.myCustomCluster, "myCustomCluster");

    // custom by ID with no manufacturer code
    let cluster = Utils::get_cluster(65534, Some(123), &customs);
    assert_cluster(&cluster, &customs.myCustomCluster, "myCustomCluster");

    // custom by ID with non-matching manufacturer code
    let cluster = Utils::get_cluster(65533, Some(123), &customs);
    assert_cluster(
        &cluster,
        &customs.myCustomClusterManuf,
        "myCustomClusterManuf",
    );

    // custom by ID with matching manufacturer code
    let cluster = Utils::get_cluster(65533, Some(65534), &customs);
    assert_cluster(
        &cluster,
        &customs.myCustomClusterManuf,
        "myCustomClusterManuf",
    );

    // custom by ID overriding same Zcl ID entirely
    let cluster = Utils::get_cluster(0x0000, None, &customs);
    assert_cluster(&cluster, &customs.genBasic, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn creates_empty_cluster_when_getting_by_invalid_id() {
    // upstream: `Zcl.Utils.getCluster(99999, undefined, {})`
    let cluster = Utils::get_cluster(99999, None, &CustomClusters::empty());

    assert_eq!(cluster.ID, 99999);
    assert_eq!(cluster.name, "99999");
    assert_eq!(cluster.manufacturerCode, None);
    assert!(cluster.attributes.is_empty());
    assert!(cluster.commands.is_empty());
    assert!(cluster.commandsResponse.is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_getting_invalid_cluster_name() {
    assert_panics(|| {
        Utils::get_cluster("invalid", None, &CustomClusters::empty());
    });
}

#[test]
#[ignore = "not implemented yet"]
fn gets_cluster_attribute() {
    let empty = CustomClusters::empty();
    let customs = custom_clusters();

    // by ID
    let cluster = Utils::get_cluster(0x0000, None, &empty);
    let attribute = Utils::get_cluster_attribute(&cluster, 0x0000, None).expect("attribute");
    assert_eq!(attribute, cluster.attributes["zclVersion"]);

    // by name
    let cluster = Utils::get_cluster("genAlarms", None, &empty);
    let attribute = Utils::get_cluster_attribute(&cluster, "alarmCount", None).expect("attribute");
    assert_eq!(attribute, cluster.attributes["alarmCount"]);

    // by ID with no manufacturer code
    let cluster = Utils::get_cluster(0x0009, Some(123), &empty);
    let attribute = Utils::get_cluster_attribute(&cluster, 0x0000, Some(123)).expect("attribute");
    assert_eq!(attribute.name, "alarmCount");

    // by ID with matching manufacturer code
    let cluster = Utils::get_cluster(0x0b05, Some(manufacturer_code::DANFOSS_A_S), &customs);
    let attribute =
        Utils::get_cluster_attribute(&cluster, 65530, Some(manufacturer_code::DANFOSS_A_S))
            .expect("attribute");
    assert_eq!(attribute.name, "danfossSystemStatusCode");

    // custom by ID
    let cluster = Utils::get_cluster(0x0000, None, &customs);
    let attribute = Utils::get_cluster_attribute(&cluster, 65533, None).expect("attribute");
    assert_eq!(attribute, cluster.attributes["myCustomAttr"]);

    // custom by name
    let cluster = Utils::get_cluster(0x0000, None, &customs);
    let attribute =
        Utils::get_cluster_attribute(&cluster, "myCustomAttr", None).expect("attribute");
    assert_eq!(attribute, cluster.attributes["myCustomAttr"]);
}

#[test]
#[ignore = "not implemented yet"]
fn returns_undefined_when_getting_invalid_attribute() {
    let cluster = Utils::get_cluster(0x0009, None, &CustomClusters::empty());

    assert_eq!(Utils::get_cluster_attribute(&cluster, "abcd", None), None);
    assert_eq!(Utils::get_cluster_attribute(&cluster, 99999, None), None);
}

#[test]
#[ignore = "not implemented yet"]
fn returns_undefined_when_getting_attribute_with_invalid_manufacturer_code() {
    let customs = custom_clusters();
    let cluster = Utils::get_cluster(0x0b05, Some(123), &customs);

    assert_eq!(
        Utils::get_cluster_attribute(&cluster, 65530, Some(123)),
        None
    );
}

#[test]
#[ignore = "not implemented yet"]
fn gets_cluster_command() {
    // by ID
    let cluster = Utils::get_cluster(0x0000, None, &CustomClusters::empty());
    let command = Utils::get_cluster_command(&cluster, 0x00);
    assert_eq!(command, cluster.commands["resetFactDefault"]);

    // by name
    let cluster = Utils::get_cluster(0x0009, None, &CustomClusters::empty());
    let command = Utils::get_cluster_command(&cluster, "resetAll");
    assert_eq!(command, cluster.commands["resetAll"]);
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_getting_invalid_command() {
    let cluster = Utils::get_cluster(0x0009, None, &CustomClusters::empty());

    assert_panics(|| {
        Utils::get_cluster_command(&cluster, "abcd");
    });
    assert_panics(|| {
        Utils::get_cluster_command(&cluster, 99999);
    });
}

#[test]
#[ignore = "not implemented yet"]
fn gets_cluster_command_response() {
    // by ID
    let cluster = Utils::get_cluster(0x0003, None, &CustomClusters::empty());
    let command = Utils::get_cluster_command_response(&cluster, 0x00);
    assert_eq!(command, cluster.commandsResponse["identifyQueryRsp"]);

    // by name
    let cluster = Utils::get_cluster(0x0009, None, &CustomClusters::empty());
    let command = Utils::get_cluster_command_response(&cluster, "getRsp");
    assert_eq!(command, cluster.commandsResponse["getRsp"]);
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_getting_invalid_command_response() {
    let cluster = Utils::get_cluster(0x0009, None, &CustomClusters::empty());

    assert_panics(|| {
        Utils::get_cluster_command_response(&cluster, "abcd");
    });
    assert_panics(|| {
        Utils::get_cluster_command_response(&cluster, 99999);
    });
}

#[test]
#[ignore = "not implemented yet"]
fn gets_global_command() {
    // by ID
    let command = Utils::get_global_command(2);
    assert_eq!(command.ID, 2);
    assert_eq!(command.name, "writeUndiv");

    // by name
    let command = Utils::get_global_command("read");
    assert_eq!(command.ID, 0);
    assert_eq!(command.name, "read");
    assert_eq!(command.response, Some(1));
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_getting_invalid_global_command() {
    assert_panics(|| {
        Utils::get_global_command(99999);
    });
    assert_panics(|| {
        Utils::get_global_command("abcd");
    });
}

#[test]
#[ignore = "not implemented yet"]
fn checks_cluster_name() {
    // upstream: `Zcl.Utils.isClusterName(name)`
    assert!(Utils::is_cluster_name("genBasic"));
    assert!(Utils::is_cluster_name("genAlarms"));
    assert!(!Utils::is_cluster_name("invalid"));
}

#[test]
#[ignore = "not implemented yet"]
fn gets_foundation_command() {
    // upstream: `Zcl.Utils.getFoundationCommand(0)`
    let command = Utils::get_foundation_command(0);
    assert_eq!(command.name, "read");
    assert_eq!(command.ID, 0);

    let command = Utils::get_foundation_command_by_name("read");
    assert_eq!(command.name, "read");
    assert_eq!(command.ID, 0);
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_getting_invalid_foundation_command_id() {
    assert_panics(|| {
        Utils::get_foundation_command(9999);
    });
    assert_panics(|| {
        Utils::get_foundation_command_by_name("doesnotexist");
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_attribute_not_writable() {
    let attribute = create_attribute();

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(1));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn returns_default_when_value_is_null_and_default_exists() {
    let attribute = Attribute {
        write: true,
        default: Some(ZclValue::num(42)),
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::Null),
        ZclValue::num(42)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn na_n_with_default_returns_default() {
    let attribute = Attribute {
        write: true,
        default: Some(ZclValue::num(7)),
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::Number(f64::NAN)),
        ZclValue::num(7)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn na_n_without_default_returns_non_value_sentinel() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::UINT8,
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::Number(f64::NAN)),
        ZclValue::num(255)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_trying_to_write_non_value_on_unsupported_datatype() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::DATA8,
        ..create_attribute()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::Number(f64::NAN));
    });

    // upstream also checks `processParameterWrite` in a separate describe
    let parameter = Parameter {
        r#type: DataType::DATA8,
        ..create_parameter()
    };
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::Number(f64::NAN));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn level_control_for_lighting_attributes_current_level_and_options() {
    let cluster = Utils::get_cluster("genLevelCtrl", None, &CustomClusters::empty());
    let options = Utils::get_cluster_attribute(&cluster, "options", None).expect("options");
    let current_level =
        Utils::get_cluster_attribute(&cluster, "currentLevel", None).expect("currentLevel");

    assert_eq!(
        Utils::process_attribute_write(&options, ZclValue::num(0x00)),
        ZclValue::num(0x00)
    );
    assert_eq!(
        Utils::process_attribute_write(&options, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    assert_panics(|| {
        Utils::process_attribute_write(&current_level, ZclValue::num(0x01));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn lighting_control_ctrl_attributes_start_up_color_temperature() {
    let cluster = Utils::get_cluster("lightingColorCtrl", None, &CustomClusters::empty());
    let attribute = Utils::get_cluster_attribute(&cluster, "startUpColorTemperature", None)
        .expect("startUpColorTemperature");

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::num(0xfeff)),
        ZclValue::num(0xfeff)
    );
    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::num(0xffff)),
        ZclValue::num(0xffff)
    );
    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(0xff00));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn rssi_location_attributes_coordinate1_and_path_loss_exponent() {
    let cluster = Utils::get_cluster("genRssiLocation", None, &CustomClusters::empty());
    let coordinate1 =
        Utils::get_cluster_attribute(&cluster, "coordinate1", None).expect("coordinate1");
    let path_loss_exponent =
        Utils::get_cluster_attribute(&cluster, "pathLossExponent", None).expect("pathLossExponent");

    assert_eq!(
        Utils::process_attribute_write(&coordinate1, ZclValue::num(-0x8000)),
        ZclValue::num(-0x8000)
    );
    assert_eq!(
        Utils::process_attribute_write(&coordinate1, ZclValue::num(0x7fff)),
        ZclValue::num(0x7fff)
    );
    assert_eq!(
        Utils::process_attribute_write(&coordinate1, ZclValue::num(0x0012)),
        ZclValue::num(0x0012)
    );
    assert_eq!(
        Utils::process_attribute_write(&path_loss_exponent, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    assert_eq!(
        Utils::process_attribute_write(&path_loss_exponent, ZclValue::Number(f64::NAN)),
        ZclValue::num(0xffff)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn process_attribute_pre_read_specific() {
    // upstream: `processAttributePreRead(attr)` throws when `read: false`
    let attribute = Attribute {
        read: false,
        ..create_attribute()
    };

    assert_panics(|| {
        Utils::process_attribute_pre_read(&attribute);
    });
}

#[test]
#[ignore = "not implemented yet"]
fn process_attribute_write_specific() {
    // upstream duplicate of the level-control write case
    let cluster = Utils::get_cluster("genLevelCtrl", None, &CustomClusters::empty());
    let options = Utils::get_cluster_attribute(&cluster, "options", None).expect("options");

    assert_eq!(
        Utils::process_attribute_write(&options, ZclValue::num(0x00)),
        ZclValue::num(0x00)
    );
    assert_eq!(
        Utils::process_attribute_write(&options, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn process_attribute_post_read_specific() {
    let cluster = Utils::get_cluster("genLevelCtrl", None, &CustomClusters::empty());
    let current_level =
        Utils::get_cluster_attribute(&cluster, "currentLevel", None).expect("currentLevel");
    let options = Utils::get_cluster_attribute(&cluster, "options", None).expect("options");

    let result = Utils::process_attribute_post_read(&current_level, ZclValue::num(0xff));
    assert!(!result.is_nan()); // technically should be true for genLevelCtrlForLighting but handled by ZHC
    assert_eq!(
        Utils::process_attribute_post_read(&current_level, ZclValue::num(0xfe)),
        ZclValue::num(0xfe)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&current_level, ZclValue::num(200)),
        ZclValue::num(200)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&options, ZclValue::num(0x00)),
        ZclValue::num(0x00)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&options, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn basic_attributes_zcl_version_and_power_source() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let zcl_version =
        Utils::get_cluster_attribute(&cluster, "zclVersion", None).expect("zclVersion");
    let power_source =
        Utils::get_cluster_attribute(&cluster, "powerSource", None).expect("powerSource");

    // max: 0xff
    assert_eq!(
        Utils::process_attribute_post_read(&zcl_version, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    // default: 0xff
    assert_eq!(
        Utils::process_attribute_post_read(&power_source, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&zcl_version, ZclValue::num(0x02)),
        ZclValue::num(0x02)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&power_source, ZclValue::num(0x03)),
        ZclValue::num(0x03)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn device_temperature_config_attribute_current_temperature() {
    let cluster = Utils::get_cluster("genDeviceTempCfg", None, &CustomClusters::empty());
    let attribute = Utils::get_cluster_attribute(&cluster, "currentTemperature", None)
        .expect("currentTemperature");

    assert!(Utils::process_attribute_post_read(&attribute, ZclValue::num(-32768)).is_nan());
    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::num(200)),
        ZclValue::num(200)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::num(-200)),
        ZclValue::num(-200)
    );
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(201));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn maps_invalid_sentinel_to_na_n() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::UINT16,
        ..create_attribute()
    };
    assert!(Utils::process_attribute_post_read(&attribute, ZclValue::num(65535)).is_nan());

    // upstream also checks `processParameterRead`
    let parameter = Parameter {
        r#type: DataType::UINT16,
        ..create_parameter()
    };
    assert!(Utils::process_parameter_read(&parameter, ZclValue::num(65535)).is_nan());
}

#[test]
#[ignore = "not implemented yet"]
fn maps_invalid_sentinel_to_na_n_with_different_min() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::INT16,
        min: Some(-32767.0),
        ..create_attribute()
    };
    assert!(Utils::process_attribute_post_read(&attribute, ZclValue::num(-32768)).is_nan());

    let parameter = Parameter {
        r#type: DataType::INT16,
        min: Some(-32767.0),
        ..create_parameter()
    };
    assert!(Utils::process_parameter_read(&parameter, ZclValue::num(-32768)).is_nan());
}

#[test]
#[ignore = "not implemented yet"]
fn maps_invalid_sentinel_to_na_n_with_different_max() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::UINT16,
        max: Some(65534.0),
        ..create_attribute()
    };
    assert!(Utils::process_attribute_post_read(&attribute, ZclValue::num(65535)).is_nan());

    let parameter = Parameter {
        r#type: DataType::UINT16,
        max: Some(65534.0),
        ..create_parameter()
    };
    assert!(Utils::process_parameter_read(&parameter, ZclValue::num(65535)).is_nan());
}

#[test]
#[ignore = "not implemented yet"]
fn returns_invalid_sentinel_unchanged_if_same_as_min_ignore_invalid_sentinel() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::INT16,
        min: Some(-32768.0),
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::num(-32768)),
        ZclValue::num(-32768)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_invalid_sentinel_unchanged_if_same_as_min_skips_invalid_sentinel() {
    let parameter = Parameter {
        r#type: DataType::INT16,
        min: Some(-32768.0),
        ..create_parameter()
    };

    assert_eq!(
        Utils::process_parameter_read(&parameter, ZclValue::num(-32768)),
        ZclValue::num(-32768)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_invalid_sentinel_unchanged_if_same_as_max_ignore_invalid_sentinel() {
    let attribute = Attribute {
        write: true,
        r#type: DataType::UINT16,
        max: Some(65535.0),
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::num(65535)),
        ZclValue::num(65535)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_invalid_sentinel_unchanged_if_same_as_max_skips_invalid_sentinel() {
    let parameter = Parameter {
        r#type: DataType::UINT16,
        max: Some(65535.0),
        ..create_parameter()
    };

    assert_eq!(
        Utils::process_parameter_read(&parameter, ZclValue::num(65535)),
        ZclValue::num(65535)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_null_when_value_is_null_and_no_default() {
    let attribute = Attribute {
        write: true,
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::Null),
        ZclValue::Null
    );
    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::Null),
        ZclValue::Null
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_value_unchanged_when_it_equals_default_skips_restrictions() {
    let attribute = Attribute {
        write: true,
        default: Some(ZclValue::num(50)),
        min: Some(60.0),
        ..create_attribute()
    };

    assert_eq!(
        Utils::process_attribute_write(&attribute, ZclValue::num(50)),
        ZclValue::num(50)
    );
    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::num(50)),
        ZclValue::num(50)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn returns_value_when_null() {
    let parameter = create_parameter();

    assert_eq!(
        Utils::process_parameter_write(&parameter, ZclValue::Null),
        ZclValue::Null
    );
    assert_eq!(
        Utils::process_parameter_read(&parameter, ZclValue::Null),
        ZclValue::Null
    );
}

#[test]
#[ignore = "not implemented yet"]
fn throws_below_min() {
    let attribute = Attribute {
        write: true,
        min: Some(10.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        min: Some(10.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(5));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_below_min_excl() {
    let attribute = Attribute {
        write: true,
        minExcl: Some(10.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        minExcl: Some(10.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(5));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(5));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_at_min_excl() {
    let attribute = Attribute {
        write: true,
        minExcl: Some(10.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        minExcl: Some(10.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(10));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(10));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(10));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(10));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_above_max() {
    let attribute = Attribute {
        write: true,
        max: Some(20.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        max: Some(20.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(30));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_above_max_excl() {
    let attribute = Attribute {
        write: true,
        maxExcl: Some(20.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        maxExcl: Some(20.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(30));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(30));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_at_max_excl() {
    let attribute = Attribute {
        write: true,
        maxExcl: Some(20.0),
        ..create_attribute()
    };
    let parameter = Parameter {
        maxExcl: Some(20.0),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::num(20));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::num(20));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::num(20));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::num(20));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_not_length() {
    let attribute = Attribute {
        write: true,
        length: Some(10),
        ..create_attribute()
    };
    let parameter = Parameter {
        length: Some(10),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::text("abcde"));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_below_min_len() {
    let attribute = Attribute {
        write: true,
        minLen: Some(10),
        ..create_attribute()
    };
    let parameter = Parameter {
        minLen: Some(10),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::text("abcde"));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::text("abcde"));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn throws_above_max_len() {
    let attribute = Attribute {
        write: true,
        maxLen: Some(2),
        ..create_attribute()
    };
    let parameter = Parameter {
        maxLen: Some(2),
        ..create_parameter()
    };

    assert_panics(|| {
        Utils::process_attribute_write(&attribute, ZclValue::text("xyz"));
    });
    assert_panics(|| {
        Utils::process_attribute_post_read(&attribute, ZclValue::text("xyz"));
    });
    assert_panics(|| {
        Utils::process_parameter_write(&parameter, ZclValue::text("xyz"));
    });
    assert_panics(|| {
        Utils::process_parameter_read(&parameter, ZclValue::text("xyz"));
    });
}

#[test]
#[ignore = "not implemented yet"]
fn process_parameter_write_specific() {
    let cluster = Utils::get_cluster("genRssiLocation", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command(&cluster, "setAbsolute");
    let coordinate1 = command
        .parameters
        .iter()
        .find(|param| param.name == "coordinate1")
        .expect("coordinate1")
        .clone();
    let path_loss_exponent = command
        .parameters
        .iter()
        .find(|param| param.name == "pathLossExponent")
        .expect("pathLossExponent")
        .clone();

    assert_eq!(
        Utils::process_parameter_write(&coordinate1, ZclValue::num(-0x8000)),
        ZclValue::num(-0x8000)
    );
    assert_eq!(
        Utils::process_parameter_write(&coordinate1, ZclValue::num(0x7fff)),
        ZclValue::num(0x7fff)
    );
    assert_eq!(
        Utils::process_parameter_write(&coordinate1, ZclValue::num(0x0012)),
        ZclValue::num(0x0012)
    );
    assert_eq!(
        Utils::process_parameter_write(&path_loss_exponent, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    assert_eq!(
        Utils::process_parameter_write(&path_loss_exponent, ZclValue::Number(f64::NAN)),
        ZclValue::num(0xffff)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn process_parameter_read_specific() {
    let cluster = Utils::get_cluster("genRssiLocation", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command_response(&cluster, "locationDataNotification");
    let coordinate1 = command
        .parameters
        .iter()
        .find(|param| param.name == "coordinate1")
        .expect("coordinate1")
        .clone();
    let path_loss_exponent = command
        .parameters
        .iter()
        .find(|param| param.name == "pathLossExponent")
        .expect("pathLossExponent")
        .clone();

    assert!(Utils::process_parameter_read(&coordinate1, ZclValue::num(-0x8000)).is_nan());
    assert_eq!(
        Utils::process_parameter_read(&coordinate1, ZclValue::num(0x7fff)),
        ZclValue::num(0x7fff)
    );
    assert_eq!(
        Utils::process_parameter_read(&coordinate1, ZclValue::num(0x0012)),
        ZclValue::num(0x0012)
    );
    assert_eq!(
        Utils::process_parameter_read(&path_loss_exponent, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    assert!(Utils::process_parameter_read(&path_loss_exponent, ZclValue::num(0xffff)).is_nan());
}

#[test]
#[ignore = "not implemented yet"]
fn scenes_attributes_last_cfg_by() {
    let cluster = Utils::get_cluster("genScenes", None, &CustomClusters::empty());
    let attribute = Utils::get_cluster_attribute(&cluster, "lastCfgBy", None).expect("lastCfgBy");

    assert_eq!(
        Utils::process_attribute_post_read(&attribute, ZclValue::text("0xffffffffffffffff")),
        ZclValue::text("0xffffffffffffffff")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn scenes_cmd_rsp_get_scene_membership_rsp_parameter_capacity() {
    let cluster = Utils::get_cluster("genScenes", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command_response(&cluster, "getSceneMembershipRsp");
    let capacity = command
        .parameters
        .iter()
        .find(|param| param.name == "capacity")
        .expect("capacity")
        .clone();

    // processParameterWrite
    assert_eq!(
        Utils::process_parameter_write(&capacity, ZclValue::num(0xfe)),
        ZclValue::num(0xfe)
    );
    assert_eq!(
        Utils::process_parameter_write(&capacity, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
    // processParameterRead
    assert_eq!(
        Utils::process_parameter_read(&capacity, ZclValue::num(0xfe)),
        ZclValue::num(0xfe)
    );
    assert_eq!(
        Utils::process_parameter_read(&capacity, ZclValue::num(0xff)),
        ZclValue::num(0xff)
    );
}
