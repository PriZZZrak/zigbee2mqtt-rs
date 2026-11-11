// Original: .upstream/zigbee-herdsman/src/zspec/zcl/buffaloZcl.ts

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct BuffaloZcl {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZcl {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    pub fn get_written(&self) -> Vec<u8> {
        self.buffer[..self.position.min(self.buffer.len())].to_vec()
    }

    pub fn write(&mut self, data_type: u32, value: &Value, options: &Value) {
        self.write_type(data_type, value, options);
    }

    pub fn read(&mut self, data_type: u32, options: &Value) -> Value {
        self.read_type(data_type, options)
    }

    fn write_type(&mut self, data_type: u32, value: &Value, options: &Value) {
        use crate::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};
        if data_type == DataType::NO_DATA as u32 || data_type == DataType::UNKNOWN as u32 {
            return;
        }
        if data_type == BuffaloZclDataType::USE_DATA_TYPE as u32 {
            if let Some(nested) = options.get("dataType").and_then(Value::as_u64) {
                return self.write_type(nested as u32, value, options);
            }
            if let Some(bytes) = buffer_like(value) {
                return self.write_bytes(&bytes);
            }
            panic!("Cannot write USE_DATA_TYPE without dataType option specified");
        }
        if data_type == BuffaloZclDataType::BUFFER as u32 {
            if let Some(bytes) = buffer_like(value) {
                self.write_bytes(&bytes);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_UINT8 as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_bytes(&[item.as_u64().unwrap_or(0) as u8]);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_UINT16 as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_u16(item.as_u64().unwrap_or(0) as u16);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_UINT24 as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_uint(item.as_u64().unwrap_or(0), 3);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_UINT32 as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_uint(item.as_u64().unwrap_or(0), 4);
            }
            return;
        }
        if data_type == BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32 {
            let number = value.as_u64().unwrap_or(0);
            self.write_bytes(&[
                ((number >> 16) & 0xff) as u8,
                ((number >> 8) & 0xff) as u8,
                (number & 0xff) as u8,
            ]);
            return;
        }
        if data_type == BuffaloZclDataType::LIST_ZONEINFO as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_bytes(&[item["zoneID"].as_u64().unwrap_or(0) as u8]);
                self.write_u16(item["zoneStatus"].as_u64().unwrap_or(0) as u16);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_MIBOXER_ZONES as u32 {
            let items = value.as_array().cloned().unwrap_or_default();
            self.write_bytes(&[items.len() as u8]);
            for item in items {
                self.write_u16(item["groupId"].as_u64().unwrap_or(0) as u16);
                self.write_bytes(&[item["zoneNum"].as_u64().unwrap_or(0) as u8]);
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                let data = buffer_like(&item["data"]).unwrap_or_default();
                self.write_bytes(&[item["dp"].as_u64().unwrap_or(0) as u8]);
                self.write_bytes(&[item["datatype"].as_u64().unwrap_or(0) as u8]);
                self.write_bytes(&[(data.len() >> 8) as u8, (data.len() & 0xff) as u8]);
                self.write_bytes(&data);
            }
            return;
        }
        if data_type == BuffaloZclDataType::EXTENSION_FIELD_SETS as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                let cluster = item["clstId"].as_u64().unwrap_or(0) as u16;
                self.write_u16(cluster);
                self.write_bytes(&[item["len"].as_u64().unwrap_or(0) as u8]);
                for (index, entry) in item["extField"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                {
                    let dtype = extension_field_sets_data_type(cluster, index);
                    self.write_dtype(dtype as u8, entry);
                }
            }
            return;
        }
        if data_type == BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32 {
            for item in value.as_array().cloned().unwrap_or_default() {
                self.write_u16(item["transitionTime"].as_u64().unwrap_or(0) as u16);
                if let Some(value) = item.get("heatSetpoint") {
                    self.write_u16(value.as_u64().unwrap_or(0) as u16);
                }
                if let Some(value) = item.get("coolSetpoint") {
                    self.write_u16(value.as_u64().unwrap_or(0) as u16);
                }
            }
            return;
        }
        if data_type == BuffaloZclDataType::MI_STRUCT as u32 {
            return;
        }
        if data_type == BuffaloZclDataType::STRUCTURED_SELECTOR as u32 {
            if value.is_null() {
                return;
            }
            if let Some(indexes) = value.get("indexes").and_then(Value::as_array) {
                self.write_bytes(&[indexes.len() as u8]);
                for index in indexes {
                    self.write_u16(index.as_u64().unwrap_or(0) as u16);
                }
            } else {
                self.write_bytes(&[value["indicatorType"].as_u64().unwrap_or(0) as u8]);
            }
            return;
        }
        if data_type == BuffaloZclDataType::GPD_FRAME as u32 {
            return self.write_gpd_frame(value, options);
        }
        let Some(dtype) = data_type_to_dtype(data_type) else {
            if let Some(bytes) = buffer_like(value) {
                self.write_bytes(&bytes);
                return;
            }
            panic!("Write for '{data_type}' not available");
        };
        self.write_dtype(dtype, value);
    }

    fn read_type(&mut self, data_type: u32, options: &Value) -> Value {
        use crate::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};
        if data_type == DataType::NO_DATA as u32 || data_type == DataType::UNKNOWN as u32 {
            return Value::Null;
        }
        if data_type == BuffaloZclDataType::USE_DATA_TYPE as u32 {
            if let Some(nested) = options.get("dataType").and_then(Value::as_u64) {
                return self.read_type(nested as u32, options);
            }
            return self.read_buffer_value(options);
        }
        if data_type == BuffaloZclDataType::BUFFER as u32 {
            return self.read_buffer_value(options);
        }
        if data_type == BuffaloZclDataType::LIST_UINT8 as u32
            || data_type == BuffaloZclDataType::LIST_UINT16 as u32
            || data_type == BuffaloZclDataType::LIST_UINT24 as u32
            || data_type == BuffaloZclDataType::LIST_UINT32 as u32
            || data_type == BuffaloZclDataType::LIST_ZONEINFO as u32
        {
            let name = match data_type {
                value if value == BuffaloZclDataType::LIST_UINT8 as u32 => "LIST_UINT8",
                value if value == BuffaloZclDataType::LIST_UINT16 as u32 => "LIST_UINT16",
                value if value == BuffaloZclDataType::LIST_UINT24 as u32 => "LIST_UINT24",
                value if value == BuffaloZclDataType::LIST_UINT32 as u32 => "LIST_UINT32",
                _ => "LIST_ZONEINFO",
            };
            let length = options
                .get("length")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("Cannot read {name} without length option specified"))
                as usize;
            let mut items = Vec::new();
            for _ in 0..length {
                items.push(match data_type {
                    value if value == BuffaloZclDataType::LIST_UINT8 as u32 => {
                        json!(self.read_u8())
                    }
                    value if value == BuffaloZclDataType::LIST_UINT16 as u32 => {
                        json!(self.read_u16())
                    }
                    value if value == BuffaloZclDataType::LIST_UINT24 as u32 => {
                        json!(self.read_uint(3))
                    }
                    value if value == BuffaloZclDataType::LIST_UINT32 as u32 => {
                        json!(self.read_uint(4))
                    }
                    _ => json!({
                        "zoneID": self.read_u8(),
                        "zoneStatus": self.read_u16(),
                    }),
                });
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32 {
            let bytes = self.read_bytes(3);
            return json!(
                (u64::from(bytes[0]) << 16) | (u64::from(bytes[1]) << 8) | u64::from(bytes[2])
            );
        }
        if data_type == BuffaloZclDataType::LIST_MIBOXER_ZONES as u32 {
            let count = self.read_u8() as usize;
            let mut items = Vec::new();
            for _ in 0..count {
                let group = self.read_u16();
                let zone = self.read_u8();
                items.push(json!({"zoneNum": zone, "groupId": group}));
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32 {
            let mut items = Vec::new();
            while self.position < self.buffer.len() {
                if !self.is_more_by(4) {
                    self.position = self.buffer.len();
                    break;
                }
                let dp = self.read_u8();
                let datatype = self.read_u8();
                let length = ((self.read_u8() as usize) << 8) | self.read_u8() as usize;
                items.push(json!({
                    "dp": dp,
                    "datatype": datatype,
                    "data": self.read_bytes(length),
                }));
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::EXTENSION_FIELD_SETS as u32 {
            let mut items = Vec::new();
            while self.is_more_by(3) {
                let cluster = self.read_u16();
                let length = self.read_u8() as usize;
                let end = self.position + length;
                if !self.is_more_by(length) {
                    break;
                }
                let mut ext_field = Vec::new();
                let mut index = 0usize;
                while self.position < end {
                    let dtype = extension_field_sets_data_type(cluster, index);
                    ext_field.push(self.read_dtype(dtype as u8));
                    index += 1;
                }
                items.push(json!({"clstId": cluster, "len": length, "extField": ext_field}));
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32 {
            let error = || -> ! {
                panic!("Cannot read LIST_THERMO_TRANSITIONS without required payload options specified")
            };
            let payload = options.get("payload").unwrap_or_else(|| error());
            let mode = payload
                .get("mode")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| error());
            let count = payload
                .get("numoftrans")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| error());
            let mut items = Vec::new();
            for _ in 0..count {
                let time = self.read_u16();
                let mut item = json!({"transitionTime": time});
                if mode & 0b01 != 0 {
                    item["heatSetpoint"] = json!(self.read_u16());
                }
                if mode & 0b10 != 0 {
                    item["coolSetpoint"] = json!(self.read_u16());
                }
                items.push(item);
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::MI_STRUCT as u32 {
            return self.read_mi_struct();
        }
        if data_type == BuffaloZclDataType::STRUCTURED_SELECTOR as u32 {
            let first = self.read_u8();
            if first == 0 {
                return json!({"indicatorType": 0});
            }
            if first == 0x10 || first == 0x20 {
                return json!({"indicatorType": first});
            }
            if first > 32 {
                panic!("Structured selector indicator outside range");
            }
            let mut indexes = Vec::new();
            for _ in 0..first {
                indexes.push(json!(self.read_u16()));
            }
            return json!({"indexes": indexes});
        }
        if data_type == BuffaloZclDataType::GPD_FRAME as u32 {
            return self.read_gpd_frame(options);
        }
        let Some(dtype) = data_type_to_dtype(data_type) else {
            panic!("Read for '{data_type}' not available");
        };
        self.read_dtype(dtype)
    }

    fn is_more_by(&self, count: usize) -> bool {
        self.position + count <= self.buffer.len()
    }

    fn read_buffer_value(&mut self, options: &Value) -> Value {
        let length = options
            .get("length")
            .and_then(Value::as_u64)
            .map(|length| length as usize)
            .unwrap_or(self.buffer.len().saturating_sub(self.position));
        json!(self.read_bytes(length))
    }

    fn write_gpd_frame(&mut self, value: &Value, _options: &Value) {
        let command = value["commandID"].as_u64().unwrap_or(0);
        match command {
            0xf0 => {
                // Commissioning Reply
                let options = value["options"].as_u64().unwrap_or(0) as u8;
                let pan_id_present = options & 0x01 != 0;
                let gpd_security_key_present = options & 0x02 != 0;
                let gpd_key_encryption = (options >> 2) & 0x01 != 0;
                let security_level = (options >> 3) & 0x03;
                let has_gpd_key_mic = gpd_key_encryption && gpd_security_key_present;
                let has_frame_counter = gpd_security_key_present
                    && gpd_key_encryption
                    && (security_level == 0b10 || security_level == 0b11);

                self.write_bytes(&[1
                    + if pan_id_present { 2 } else { 0 }
                    + if gpd_security_key_present { 16 } else { 0 }
                    + if has_gpd_key_mic { 4 } else { 0 }
                    + if has_frame_counter { 4 } else { 0 }]);
                self.write_bytes(&[options]);

                if pan_id_present {
                    self.write_u16(value["panID"].as_u64().unwrap_or(0) as u16);
                }

                if gpd_security_key_present {
                    let mut security_key = buffer_like(&value["securityKey"]).unwrap_or_default();
                    security_key.resize(16, 0);
                    self.write_bytes(&security_key[..16]);
                }

                if has_gpd_key_mic {
                    self.write_uint(value["keyMic"].as_u64().unwrap_or(0), 4);
                }

                if has_frame_counter {
                    self.write_uint(value["frameCounter"].as_u64().unwrap_or(0), 4);
                }
            }
            0xf3 => {
                // Channel configuration
                let channel = value["operationalChannel"].as_u64().unwrap_or(0) as u8;
                let basic = value["basic"].as_bool().unwrap_or(false);
                self.write_bytes(&[1]);
                self.write_bytes(&[(channel & 0x0f) | if basic { 0x10 } else { 0 }]);
            }
            0xf4 | 0xf5 => {
                self.write_custom_gpd_frame(value);
            }
            command if (0xf7..=0xff).contains(&command) => {
                self.write_custom_gpd_frame(value);
            }
            // 0xf1: Write Attributes
            // 0xf2: Read Attributes
            // 0xf6: ZCL Tunneling
            _ => {}
        }
    }

    fn write_custom_gpd_frame(&mut self, value: &Value) {
        let buffer = buffer_like(&value["buffer"]).unwrap_or_default();
        self.write_bytes(&[buffer.len() as u8]);
        self.write_bytes(&buffer);
    }

    pub(crate) fn read_gpd_frame(&mut self, options: &Value) -> Value {
        let Some(payload) = options.get("payload") else {
            panic!("Cannot read GPD_FRAME without required payload options specified");
        };
        let Some(payload_size) = payload.get("payloadSize") else {
            panic!("Cannot read GPD_FRAME without required payload options specified");
        };
        if payload_size.is_null() {
            return json!({});
        }
        let size = payload_size.as_u64().unwrap_or(0);
        if size == 0 {
            return json!({});
        }
        let size = size as usize;
        let start_position = self.position;
        let command = payload
            .get("commandID")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        let result = match command {
            0xe0 => {
                // Commissioning
                let device_id = self.read_u8();
                let gpd_options = self.read_u8();
                let mut extended_options = 0u8;
                let mut security_key = vec![0u8; 16];
                let mut key_mic = 0u32;
                let mut outgoing_counter = 0u32;
                let mut application_info = 0u8;
                let mut manufacturer_id = 0u16;
                let mut model_id = 0u16;
                let mut num_gpd_commands = 0u8;
                let mut gpd_command_id_list: Vec<u8> = Vec::new();
                let mut num_server_clusters = 0u8;
                let mut num_client_clusters = 0u8;
                let mut gpd_server_clusters: Vec<u8> = Vec::new();
                let mut gpd_client_clusters: Vec<u8> = Vec::new();
                let mut generic_switch_config = 0u8;
                let mut current_contact_status = 0u8;

                if gpd_options & 0x80 != 0 {
                    extended_options = self.read_u8();
                }
                if extended_options & 0x20 != 0 {
                    security_key = self.read_bytes(16);
                }
                if extended_options & 0x40 != 0 {
                    key_mic = self.read_uint(4) as u32;
                }
                if extended_options & 0x80 != 0 {
                    outgoing_counter = self.read_uint(4) as u32;
                }
                if gpd_options & 0x04 != 0 {
                    application_info = self.read_u8();
                }
                if application_info & 0x01 != 0 {
                    manufacturer_id = self.read_u16();
                }
                if application_info & 0x02 != 0 {
                    model_id = self.read_u16();
                }
                if application_info & 0x04 != 0 {
                    num_gpd_commands = self.read_u8();
                    gpd_command_id_list = self.read_bytes(num_gpd_commands as usize);
                }
                if application_info & 0x08 != 0 {
                    let length = self.read_u8();
                    num_server_clusters = length & 0xf;
                    num_client_clusters = (length >> 4) & 0xf;
                    gpd_server_clusters = self.read_bytes(2 * num_server_clusters as usize);
                    gpd_client_clusters = self.read_bytes(2 * num_client_clusters as usize);
                }
                if application_info & 0x10 != 0 {
                    let length = self.read_u8();
                    if length >= 1 {
                        generic_switch_config = self.read_u8();
                    }
                    if length >= 2 {
                        current_contact_status = self.read_u8();
                    }
                }

                json!({
                    "deviceID": device_id,
                    "options": gpd_options,
                    "extendedOptions": extended_options,
                    "securityKey": security_key,
                    "keyMic": key_mic,
                    "outgoingCounter": outgoing_counter,
                    "applicationInfo": application_info,
                    "manufacturerID": manufacturer_id,
                    "modelID": model_id,
                    "numGpdCommands": num_gpd_commands,
                    "gpdCommandIdList": gpd_command_id_list,
                    "numServerClusters": num_server_clusters,
                    "numClientClusters": num_client_clusters,
                    "gpdServerClusters": gpd_server_clusters,
                    "gpdClientClusters": gpd_client_clusters,
                    "genericSwitchConfig": generic_switch_config,
                    "currentContactStatus": current_contact_status,
                })
            }
            0xe3 => {
                // Channel Request
                let channel_options = self.read_u8();
                json!({
                    "nextChannel": channel_options & 0xf,
                    "nextNextChannel": (channel_options >> 4) & 0xf,
                })
            }
            0xa0 => {
                // Attribute Reporting
                let cluster_id = self.read_u16();
                let cluster = gpd_cluster(cluster_id, None);
                let mut attributes = serde_json::Map::new();
                while self.position.saturating_sub(start_position) < size {
                    let attribute_id = self.read_u16();
                    let data_type = self.read_u8();
                    let attribute_name = gpd_attribute_name(&cluster, attribute_id, None);
                    let value = self.read_type(data_type as u32, options);
                    attributes.insert(attribute_name, value);
                }
                json!({
                    "clusterID": cluster_id,
                    "clusterName": cluster.name,
                    "attributes": attributes,
                })
            }
            0xa1 => {
                // Manufacturer-specific Attribute Reporting
                let manufacturer_code = self.read_u16();
                let cluster_id = self.read_u16();
                let cluster = gpd_cluster(cluster_id, Some(manufacturer_code));
                let mut attributes = serde_json::Map::new();
                while self.position.saturating_sub(start_position) < size {
                    let attribute_id = self.read_u16();
                    let data_type = self.read_u8();
                    let attribute_name =
                        gpd_attribute_name(&cluster, attribute_id, Some(manufacturer_code));
                    let value = self.read_type(data_type as u32, options);
                    attributes.insert(attribute_name, value);
                }
                json!({
                    "manufacturerCode": manufacturer_code,
                    "clusterID": cluster_id,
                    "clusterName": cluster.name,
                    "attributes": attributes,
                })
            }
            0xa2 => {
                // Multi-Cluster Reporting
                let mut reports = Vec::new();
                while self.position.saturating_sub(start_position) < size {
                    let cluster_id = self.read_u16();
                    let cluster = gpd_cluster(cluster_id, None);
                    let attribute_id = self.read_u16();
                    let data_type = self.read_u8();
                    let attribute_name = gpd_attribute_name(&cluster, attribute_id, None);
                    let attr_data = self.read_type(data_type as u32, options);
                    reports.push(json!({
                        "clusterID": cluster_id,
                        "clusterName": cluster.name,
                        "attribute": attribute_value_from_name(&attribute_name, attribute_id),
                        "attrData": attr_data,
                    }));
                }
                json!({"reports": reports})
            }
            0xa3 => {
                // Manufacturer-specific Multi-Cluster Reporting
                let manufacturer_code = self.read_u16();
                let mut reports = Vec::new();
                while self.position.saturating_sub(start_position) < size {
                    let cluster_id = self.read_u16();
                    let cluster = gpd_cluster(cluster_id, Some(manufacturer_code));
                    let attribute_id = self.read_u16();
                    let data_type = self.read_u8();
                    let attribute_name =
                        gpd_attribute_name(&cluster, attribute_id, Some(manufacturer_code));
                    let attr_data = self.read_type(data_type as u32, options);
                    reports.push(json!({
                        "clusterID": cluster_id,
                        "clusterName": cluster.name,
                        "attribute": attribute_value_from_name(&attribute_name, attribute_id),
                        "attrData": attr_data,
                    }));
                }
                json!({"manufacturerCode": manufacturer_code, "reports": reports})
            }
            0xa4 => {
                // Request Attributes
                let gpd_options = self.read_u8();
                let manufacturer = if gpd_options & 0x02 != 0 {
                    Some(self.read_u16())
                } else {
                    None
                };
                let mut records = Vec::new();
                while self.position.saturating_sub(start_position) < size {
                    let cluster_id = self.read_u16();
                    let records_length = self.read_u8() as usize / 2;
                    let mut attributes = Vec::new();
                    for _ in 0..records_length {
                        attributes.push(json!(self.read_u16()));
                    }
                    records.push(json!({"clusterID": cluster_id, "attributes": attributes}));
                }
                let mut frame = json!({
                    "options": gpd_options,
                    "records": records,
                });
                if let Some(manufacturer) = manufacturer {
                    frame["manufacturerID"] = json!(manufacturer);
                }
                frame
            }
            0xa5 => {
                // Read Attributes Response
                let gpd_options = self.read_u8();
                let manufacturer = if gpd_options & 0x02 != 0 {
                    Some(self.read_u16())
                } else {
                    None
                };
                let mut records = Vec::new();
                while self.position.saturating_sub(start_position) < size {
                    let cluster_id = self.read_u16();
                    let cluster = gpd_cluster(cluster_id, manufacturer);
                    let records_length = self.read_u8();
                    let mut attributes = serde_json::Map::new();
                    let mut read_length = 0usize;

                    while read_length < records_length as usize {
                        let start_pos = self.position;
                        let attribute_id = self.read_u16();
                        let status = self.read_u8();
                        let data_type = self.read_u8();
                        let attribute_name =
                            gpd_attribute_name(&cluster, attribute_id, manufacturer);
                        let value = self.read_type(data_type as u32, options);
                        attributes
                            .insert(attribute_name, json!({"status": status, "attrData": value}));
                        read_length += self.position.saturating_sub(start_pos);
                    }

                    records.push(json!({
                        "clusterID": cluster_id,
                        "clusterName": cluster.name,
                        "attributes": attributes,
                    }));
                }
                let mut frame = json!({
                    "options": gpd_options,
                    "records": records,
                });
                if let Some(manufacturer) = manufacturer {
                    frame["manufacturerID"] = json!(manufacturer);
                }
                frame
            }
            _ => {
                // might contain `gppNwkAddr`, `gppGpdLink` & `mic` from ZCL cluster after
                // this, so limit by `payloadSize`
                json!({"raw": self.read_bytes(size)})
            }
        };

        self.set_position(start_position + size);

        result
    }

    fn read_mi_struct(&mut self) -> Value {
        if self.buffer.get(self.position) == Some(&0xff) {
            self.position += 1;
            return json!({});
        }
        let length = self.read_u8() as usize;
        let end = (self.position + length).min(self.buffer.len());
        let mut output = serde_json::Map::new();
        while self.position < end {
            let id = self.read_u8();
            if self.position >= end {
                break;
            }
            let dtype = self.read_u8();
            let value = self.read_dtype_u8(dtype);
            output.insert(id.to_string(), value);
        }
        self.position = self.buffer.len();
        Value::Object(output)
    }

    fn write_dtype(&mut self, dtype: u8, value: &Value) {
        use crate::zspec::zcl::definition::enums::DataType;
        if dtype == DataType::DATE as u8 {
            for key in ["year", "month", "dayOfMonth", "dayOfWeek"] {
                let byte = if key == "year" {
                    let year = value.get("year").and_then(Value::as_i64);
                    match year {
                        Some(year) => ((year - 1900) & 0xff) as u8,
                        None => 0xff,
                    }
                } else {
                    value
                        .get(key)
                        .and_then(Value::as_f64)
                        .map(|number| if number.is_nan() { 0xff } else { number as u8 })
                        .unwrap_or(0xff)
                };
                self.write_bytes(&[byte]);
            }
            return;
        }
        if dtype == DataType::TOD as u8 {
            for key in ["hours", "minutes", "seconds", "hundredths"] {
                let byte = value
                    .get(key)
                    .and_then(Value::as_f64)
                    .map(|number| if number.is_nan() { 0xff } else { number as u8 })
                    .unwrap_or(0xff);
                self.write_bytes(&[byte]);
            }
            return;
        }
        if dtype == DataType::ARRAY as u8 {
            if value.is_null() {
                self.write_bytes(&[0, 0xff, 0xff]);
                return;
            }
            let element_type = element_type(value["elementType"].clone());
            let elements = value["elements"].as_array().cloned().unwrap_or_default();
            self.write_bytes(&[element_type]);
            self.write_u16(elements.len() as u16);
            for element in elements {
                self.write_dtype(element_type, &element);
            }
            return;
        }
        if dtype == DataType::STRUCT as u8 {
            if value.is_null() {
                self.write_bytes(&[0xff, 0xff]);
                return;
            }
            let elements = value.as_array().cloned().unwrap_or_default();
            self.write_bytes(&[(elements.len() & 0xff) as u8, (elements.len() >> 8) as u8]);
            for element in elements {
                let element_type = element["elmType"].as_u64().unwrap_or(0) as u8;
                self.write_bytes(&[element_type]);
                self.write_dtype(element_type, &element["elmVal"]);
            }
            return;
        }
        if dtype == DataType::OCTET_STR as u8 {
            if value.is_null() {
                self.write_bytes(&[0xff]);
                return;
            }
            let bytes = buffer_like(value).unwrap_or_default();
            self.write_bytes(&[bytes.len() as u8]);
            self.write_bytes(&bytes);
            return;
        }
        if dtype == DataType::CHAR_STR as u8 {
            if value.is_null() {
                self.write_bytes(&[0xff]);
                return;
            }
            if let Some(bytes) = value.as_array().map(|items| {
                items
                    .iter()
                    .map(|item| item.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<_>>()
            }) {
                self.write_bytes(&bytes);
                return;
            }
            let text = value.as_str().unwrap_or("").as_bytes().to_vec();
            self.write_bytes(&[text.len() as u8]);
            self.write_bytes(&text);
            return;
        }
        if dtype == DataType::LONG_OCTET_STR as u8 {
            if value.is_null() {
                self.write_bytes(&[0xff, 0xff]);
                return;
            }
            let bytes = buffer_like(value).unwrap_or_default();
            self.write_u16(bytes.len() as u16);
            self.write_bytes(&bytes);
            return;
        }
        if dtype == DataType::LONG_CHAR_STR as u8 {
            if value.is_null() {
                self.write_bytes(&[0xff, 0xff]);
                return;
            }
            let bytes = value.as_str().unwrap_or("").as_bytes().to_vec();
            self.write_u16(bytes.len() as u16);
            self.write_bytes(&bytes);
            return;
        }
        if dtype == DataType::IEEE_ADDR as u8 {
            let hex = value.as_str().unwrap_or("0x0").trim_start_matches("0x");
            let padded = format!("{hex:0>16}");
            let mut bytes = Vec::new();
            let mut index = padded.len();
            while index >= 2 {
                bytes.push(u8::from_str_radix(&padded[index - 2..index], 16).unwrap_or(0));
                index -= 2;
            }
            self.write_bytes(&bytes);
            return;
        }
        if dtype == DataType::SEC_KEY as u8 {
            self.write_bytes(&buffer_like(value).unwrap_or_default());
            return;
        }
        if dtype == DataType::SINGLE_PREC as u8 {
            self.write_bytes(&(value.as_f64().unwrap_or(0.0) as f32).to_le_bytes());
            return;
        }
        if dtype == DataType::DOUBLE_PREC as u8 {
            self.write_bytes(&value.as_f64().unwrap_or(0.0).to_le_bytes());
            return;
        }
        if dtype == DataType::SEMI_PREC as u8 {
            let half = f32_to_half(value.as_f64().unwrap_or(0.0) as f32);
            self.write_u16(half);
            return;
        }
        let signed = matches!(dtype, 0x28..=0x2f);
        let length = match dtype {
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => 1,
            0x09 | 0x19 | 0x21 | 0x29 | 0x31 | 0xe8 | 0xe9 => 2,
            0x0a | 0x1a | 0x22 | 0x2a => 3,
            0x0b | 0x1b | 0x23 | 0x2b | 0xe2 | 0xea => 4,
            0x0c | 0x1c | 0x24 | 0x2c => 5,
            0x0d | 0x1d | 0x25 | 0x2d => 6,
            0x0e | 0x1e | 0x26 | 0x2e => 7,
            0x0f | 0x1f | 0x27 | 0x2f => 8,
            _ => 0,
        };
        if length == 0 {
            return;
        }
        let number = if signed {
            value.as_i64().unwrap_or(0) as u64
        } else {
            value.as_u64().unwrap_or(0)
        };
        self.write_uint(number, length);
    }

    fn read_dtype(&mut self, dtype: u8) -> Value {
        self.read_dtype_with(dtype)
    }

    fn read_dtype_u8(&mut self, dtype: u8) -> Value {
        self.read_dtype_with(dtype)
    }

    fn read_dtype_with(&mut self, dtype: u8) -> Value {
        use crate::zspec::zcl::definition::enums::DataType;
        if dtype == DataType::DATE as u8 {
            let year = self.read_u8();
            let month = self.read_u8();
            let day = self.read_u8();
            let week = self.read_u8();
            return json!({
                "year": date_part(year, true),
                "month": date_part(month, false),
                "dayOfMonth": date_part(day, false),
                "dayOfWeek": date_part(week, false),
            });
        }
        if dtype == DataType::TOD as u8 {
            return json!({
                "hours": date_part(self.read_u8(), false),
                "minutes": date_part(self.read_u8(), false),
                "seconds": date_part(self.read_u8(), false),
                "hundredths": date_part(self.read_u8(), false),
            });
        }
        if dtype == DataType::ARRAY as u8 {
            let element_type = self.read_u8();
            let count = self.read_u16();
            if count == 0xffff {
                return json!([]);
            }
            let mut items = Vec::new();
            for _ in 0..count {
                items.push(self.read_dtype_with(element_type));
            }
            return Value::Array(items);
        }
        if dtype == DataType::STRUCT as u8 {
            let count = usize::from(self.read_u8()) | (usize::from(self.read_u8()) << 8);
            if count == 0xffff {
                return json!([]);
            }
            let mut items = Vec::new();
            for _ in 0..count {
                let element_type = self.read_u8();
                let value = self.read_dtype_with(element_type);
                items.push(json!({"elmType": element_type, "elmVal": value}));
            }
            return Value::Array(items);
        }
        if dtype == DataType::OCTET_STR as u8 {
            let length = self.read_u8();
            if length == 0xff {
                return json!([]);
            }
            return json!(self.read_bytes(length as usize));
        }
        if dtype == DataType::CHAR_STR as u8 {
            let length = self.read_u8();
            if length == 0xff {
                return json!("");
            }
            return json!(String::from_utf8_lossy(&self.read_bytes(length as usize)).to_string());
        }
        if dtype == DataType::LONG_OCTET_STR as u8 {
            let length = self.read_u16();
            if length == 0xffff {
                return json!([]);
            }
            return json!(self.read_bytes(length as usize));
        }
        if dtype == DataType::LONG_CHAR_STR as u8 {
            let length = self.read_u16();
            if length == 0xffff {
                return json!("");
            }
            return json!(String::from_utf8_lossy(&self.read_bytes(length as usize)).to_string());
        }
        if dtype == DataType::IEEE_ADDR as u8 {
            let bytes = self.read_bytes(8);
            let mut reversed = bytes;
            reversed.reverse();
            return json!(format!(
                "0x{}",
                reversed
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ));
        }
        if dtype == DataType::SEC_KEY as u8 {
            return json!(self.read_bytes(16));
        }
        if dtype == DataType::SINGLE_PREC as u8 {
            let bytes = self.read_bytes(4);
            return float_json(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64);
        }
        if dtype == DataType::DOUBLE_PREC as u8 {
            let bytes = self.read_bytes(8);
            return json!(f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
            ]));
        }
        if dtype == DataType::SEMI_PREC as u8 {
            return json!(half_to_f32(self.read_u16()));
        }
        let length = match dtype {
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => 1,
            0x09 | 0x19 | 0x21 | 0x29 | 0x31 | 0xe8 | 0xe9 => 2,
            0x0a | 0x1a | 0x22 | 0x2a => 3,
            0x0b | 0x1b | 0x23 | 0x2b | 0xe2 | 0xea => 4,
            0x0c | 0x1c | 0x24 | 0x2c => 5,
            0x0d | 0x1d | 0x25 | 0x2d => 6,
            0x0e | 0x1e | 0x26 | 0x2e => 7,
            0x0f | 0x1f | 0x27 | 0x2f => 8,
            _ => 0,
        };
        if length == 0 {
            return Value::Null;
        }
        let signed = matches!(dtype, 0x28..=0x2f);
        let raw = self.read_uint(length);
        if signed {
            let bits = length * 8;
            let sign_bit = 1u64 << (bits - 1);
            if raw & sign_bit != 0 {
                json!((raw as i128 - (1i128 << bits)) as i64)
            } else {
                json!(raw)
            }
        } else {
            json!(raw)
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        if self.position + bytes.len() > self.buffer.len() {
            self.buffer.resize(self.position + bytes.len(), 0);
        }
        self.buffer[self.position..self.position + bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
    }

    fn read_bytes(&mut self, length: usize) -> Vec<u8> {
        // Upstream readBuffer advances by the requested length and returns a
        // shorter slice when the buffer ends early.
        let end = self.position + length;
        let available = end.min(self.buffer.len());
        let values = self.buffer[self.position..available].to_vec();
        self.position = end;
        values
    }

    fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_uint(&mut self, value: u64, length: usize) {
        self.write_bytes(&value.to_le_bytes()[..length]);
    }

    fn read_u8(&mut self) -> u8 {
        let value = self.buffer.get(self.position).copied().unwrap_or(0);
        self.position += 1;
        value
    }

    fn read_u16(&mut self) -> u16 {
        let bytes = self.read_bytes(2);
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    fn read_u32(&mut self) -> u32 {
        let bytes = self.read_bytes(4);
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn read_uint(&mut self, length: usize) -> u64 {
        let bytes = self.read_bytes(length);
        let mut value = 0u64;
        for (index, byte) in bytes.iter().enumerate() {
            value |= u64::from(*byte) << (index * 8);
        }
        value
    }

    pub fn write_structured_selector(
        &mut self,
        value: &serde_json::Value,
        options: &serde_json::Value,
    ) {
        self.write(
            crate::zspec::zcl::definition::enums::BuffaloZclDataType::STRUCTURED_SELECTOR as u32,
            value,
            options,
        );
    }

    pub fn read_structured_selector(&mut self, options: &serde_json::Value) -> serde_json::Value {
        self.read(
            crate::zspec::zcl::definition::enums::BuffaloZclDataType::STRUCTURED_SELECTOR as u32,
            options,
        )
    }
}

fn buffer_like(value: &Value) -> Option<Vec<u8>> {
    if let Some(items) = value.as_array() {
        return Some(
            items
                .iter()
                .map(|item| item.as_u64().unwrap_or(0) as u8)
                .collect(),
        );
    }
    None
}

fn element_type(value: Value) -> u8 {
    match value {
        Value::Number(number) => number.as_u64().unwrap_or(0) as u8,
        Value::String(name) => {
            use crate::zspec::zcl::definition::enums::DataType;
            match name.as_str() {
                "NO_DATA" => DataType::NO_DATA as u8,
                "DATA8" => DataType::DATA8 as u8,
                "DATA16" => DataType::DATA16 as u8,
                "DATA24" => DataType::DATA24 as u8,
                "DATA32" => DataType::DATA32 as u8,
                "DATA40" => DataType::DATA40 as u8,
                "DATA48" => DataType::DATA48 as u8,
                "DATA56" => DataType::DATA56 as u8,
                "DATA64" => DataType::DATA64 as u8,
                "BOOLEAN" => DataType::BOOLEAN as u8,
                "BITMAP8" => DataType::BITMAP8 as u8,
                "BITMAP16" => DataType::BITMAP16 as u8,
                "BITMAP24" => DataType::BITMAP24 as u8,
                "BITMAP32" => DataType::BITMAP32 as u8,
                "BITMAP40" => DataType::BITMAP40 as u8,
                "BITMAP48" => DataType::BITMAP48 as u8,
                "BITMAP56" => DataType::BITMAP56 as u8,
                "BITMAP64" => DataType::BITMAP64 as u8,
                "UINT8" => DataType::UINT8 as u8,
                "UINT16" => DataType::UINT16 as u8,
                "UINT24" => DataType::UINT24 as u8,
                "UINT32" => DataType::UINT32 as u8,
                "UINT40" => DataType::UINT40 as u8,
                "UINT48" => DataType::UINT48 as u8,
                "UINT56" => DataType::UINT56 as u8,
                "UINT64" => DataType::UINT64 as u8,
                "INT8" => DataType::INT8 as u8,
                "INT16" => DataType::INT16 as u8,
                "INT24" => DataType::INT24 as u8,
                "INT32" => DataType::INT32 as u8,
                "INT40" => DataType::INT40 as u8,
                "INT48" => DataType::INT48 as u8,
                "INT56" => DataType::INT56 as u8,
                "INT64" => DataType::INT64 as u8,
                "ENUM8" => DataType::ENUM8 as u8,
                "ENUM16" => DataType::ENUM16 as u8,
                "SEMI_PREC" => DataType::SEMI_PREC as u8,
                "SINGLE_PREC" => DataType::SINGLE_PREC as u8,
                "DOUBLE_PREC" => DataType::DOUBLE_PREC as u8,
                "OCTET_STR" => DataType::OCTET_STR as u8,
                "CHAR_STR" => DataType::CHAR_STR as u8,
                "LONG_OCTET_STR" => DataType::LONG_OCTET_STR as u8,
                "LONG_CHAR_STR" => DataType::LONG_CHAR_STR as u8,
                "ARRAY" => DataType::ARRAY as u8,
                "STRUCT" => DataType::STRUCT as u8,
                "SET" => DataType::SET as u8,
                "BAG" => DataType::BAG as u8,
                "TOD" => DataType::TOD as u8,
                "DATE" => DataType::DATE as u8,
                "UTC" => DataType::UTC as u8,
                "CLUSTER_ID" => DataType::CLUSTER_ID as u8,
                "ATTR_ID" => DataType::ATTR_ID as u8,
                "BAC_OID" => DataType::BAC_OID as u8,
                "IEEE_ADDR" => DataType::IEEE_ADDR as u8,
                "SEC_KEY" => DataType::SEC_KEY as u8,
                "UNKNOWN" => DataType::UNKNOWN as u8,
                _ => 0,
            }
        }
        _ => 0,
    }
}

fn data_type_to_dtype(data_type: u32) -> Option<u8> {
    match data_type {
        0x00 | 0x08 | 0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x0e | 0x0f | 0x10 | 0x18 | 0x19
        | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e | 0x1f | 0x20 | 0x21 | 0x22 | 0x23 | 0x24 | 0x25
        | 0x26 | 0x27 | 0x28 | 0x29 | 0x2a | 0x2b | 0x2c | 0x2d | 0x2e | 0x2f | 0x30 | 0x31
        | 0x38 | 0x39 | 0x3a | 0x41 | 0x42 | 0x43 | 0x44 | 0x48 | 0x4c | 0x50 | 0x51 | 0xe0
        | 0xe1 | 0xe2 | 0xe8 | 0xe9 | 0xea | 0xf0 | 0xf1 => Some(data_type as u8),
        _ => None,
    }
}

fn f32_to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let mut exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mut mantissa = (bits >> 13) & 0x3ff;
    if exponent <= 0 {
        mantissa = (mantissa | 0x400) >> (1 - exponent);
        exponent = 0;
    } else if exponent >= 31 {
        exponent = 31;
        mantissa = 0;
    }
    sign | ((exponent as u16) << 10) | mantissa as u16
}

fn half_to_f32(half: u16) -> f64 {
    let sign = if half & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = ((half >> 10) & 0x1f) as i32;
    let mantissa = f64::from(half & 0x03ff);
    if exponent == 0 {
        sign * mantissa * 2f64.powi(-24)
    } else if exponent == 31 {
        sign * f64::INFINITY
    } else {
        sign * (1.0 + mantissa / 1024.0) * 2f64.powi(exponent - 15)
    }
}

fn date_part(value: u8, year: bool) -> Value {
    if value == 0xff {
        json!(f64::NAN)
    } else if year {
        json!(u16::from(value) + 1900)
    } else {
        json!(value)
    }
}

fn cluster_name(cluster: u16) -> String {
    match cluster {
        0x0000 => "genBasic".to_string(),
        other => other.to_string(),
    }
}

fn gpd_cluster(
    cluster_id: u16,
    manufacturer_code: Option<u16>,
) -> crate::zspec::zcl::utils::Cluster {
    crate::zspec::zcl::utils::Utils::get_cluster(
        cluster_id,
        manufacturer_code,
        &crate::zspec::zcl::utils::CustomClusters::empty(),
    )
}

fn gpd_attribute_name(
    cluster: &crate::zspec::zcl::utils::Cluster,
    attribute_id: u16,
    manufacturer_code: Option<u16>,
) -> String {
    crate::zspec::zcl::utils::Utils::get_cluster_attribute(cluster, attribute_id, manufacturer_code)
        .map(|attribute| attribute.name)
        .unwrap_or_else(|| attribute_id.to_string())
}

fn attribute_value_from_name(name: &str, attribute_id: u16) -> Value {
    match name.parse::<u64>() {
        Ok(_) => json!(attribute_id),
        Err(_) => json!(name),
    }
}

fn attribute_name(cluster: u16, attr: u16) -> String {
    if cluster == 0x0000 {
        match attr {
            0x0000 => "zclVersion".to_string(),
            0x0001 => "appVersion".to_string(),
            0x0002 => "stackVersion".to_string(),
            0x0003 => "hwVersion".to_string(),
            0x0004 => "manufacturerName".to_string(),
            0x0005 => "modelId".to_string(),
            0x0006 => "dateCode".to_string(),
            0x0007 => "powerSource".to_string(),
            other => other.to_string(),
        }
    } else {
        attr.to_string()
    }
}

fn float_json(value: f64) -> Value {
    if value == 0.0 || (value.fract() == 0.0 && value.abs() < 9.0e15) {
        json!(value as i64)
    } else {
        json!(value)
    }
}

fn attribute_value(cluster: u16, attr: u16) -> Value {
    if cluster == 0x0000 {
        match attr {
            0x0000 => json!("zclVersion"),
            0x0001 => json!("appVersion"),
            0x0002 => json!("stackVersion"),
            0x0003 => json!("hwVersion"),
            0x0004 => json!("manufacturerName"),
            0x0005 => json!("modelId"),
            0x0006 => json!("dateCode"),
            0x0007 => json!("powerSource"),
            other => json!(other),
        }
    } else {
        json!(attr)
    }
}

/// Upstream `EXTENSION_FIELD_SETS_DATA_TYPE`.
fn extension_field_sets_data_type(
    cluster: u16,
    index: usize,
) -> crate::zspec::zcl::definition::enums::DataType {
    use crate::zspec::zcl::definition::enums::DataType::{UINT16, UINT8};
    let types: &[crate::zspec::zcl::definition::enums::DataType] = match cluster {
        6 | 8 => &[UINT8],
        258 => &[UINT8, UINT8],
        768 => &[UINT16, UINT16, UINT16, UINT8, UINT8, UINT8, UINT16, UINT16],
        other => panic!("No extension field set data types for cluster '{other}'"),
    };
    *types.get(index).unwrap_or_else(|| {
        panic!("No extension field set data type for cluster '{cluster}' index '{index}'")
    })
}
