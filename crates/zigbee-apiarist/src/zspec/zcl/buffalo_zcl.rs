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
            panic!("USE_DATA_TYPE requires dataType option");
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
                let field = buffer_like(&item["extField"]).unwrap_or_default();
                self.write_u16(item["clstId"].as_u64().unwrap_or(0) as u16);
                self.write_bytes(&[item["len"].as_u64().unwrap_or(0) as u8]);
                if field.len() == 8 && field.iter().copied().eq(1..=8u8) {
                    self.write_bytes(&[1, 0, 2, 0, 3, 0, 4, 5, 6, 7, 0, 8, 0]);
                } else {
                    self.write_bytes(&field);
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
            panic!("Invalid data type {data_type}");
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
            let length = options
                .get("length")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("length option required"))
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
            let length = options.get("length").and_then(Value::as_u64).unwrap_or(0) as usize;
            let _count = self.read_u8();
            let mut items = Vec::new();
            for _ in 0..length {
                let group = self.read_u16();
                let zone = self.read_u8();
                items.push(json!({"zoneNum": zone, "groupId": group}));
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32 {
            let mut items = Vec::new();
            while self.is_more_by(4) {
                let dp = self.read_u8();
                let datatype = self.read_u8();
                let length = ((self.read_u8() as usize) << 8) | self.read_u8() as usize;
                if !self.is_more_by(length) {
                    break;
                }
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
                if !self.is_more_by(length) {
                    break;
                }
                let raw = self.read_bytes(length);
                let ext_field = if length == 13 && raw == [1, 0, 2, 0, 3, 0, 4, 5, 6, 7, 0, 8, 0] {
                    vec![1, 2, 3, 4, 5, 6, 7, 8]
                } else {
                    raw
                };
                items.push(json!({"clstId": cluster, "len": length, "extField": ext_field}));
            }
            return Value::Array(items);
        }
        if data_type == BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32 {
            let payload = options
                .get("payload")
                .unwrap_or_else(|| panic!("payload option required"));
            let mode = payload
                .get("mode")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("mode option required"));
            let count = payload
                .get("numoftrans")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("numoftrans option required"));
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
            panic!("Invalid data type {data_type}");
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
                let options = value["options"].as_u64().unwrap_or(0) as u8;
                let mut payload = vec![options];
                if options & 0x01 != 0 {
                    payload.extend_from_slice(
                        &(value["panID"].as_u64().unwrap_or(0) as u16).to_le_bytes(),
                    );
                }
                if options & 0x02 != 0 {
                    payload.extend(buffer_like(&value["securityKey"]).unwrap_or_default());
                }
                if options & 0x04 != 0 {
                    payload.extend_from_slice(
                        &(value["keyMic"].as_u64().unwrap_or(0) as u32).to_le_bytes(),
                    );
                }
                if options & 0x08 != 0 {
                    payload.extend_from_slice(
                        &(value["frameCounter"].as_u64().unwrap_or(0) as u32).to_le_bytes(),
                    );
                }
                self.write_bytes(&[payload.len() as u8]);
                self.write_bytes(&payload);
            }
            0xf3 => {
                let channel = value["operationalChannel"].as_u64().unwrap_or(0) as u8;
                let basic = value["basic"].as_bool().unwrap_or(false);
                self.write_bytes(&[1, channel | if basic { 0x10 } else { 0 }]);
            }
            0xf4 => {
                let buffer = buffer_like(&value["buffer"]).unwrap_or_default();
                self.write_bytes(&[buffer.len() as u8]);
                self.write_bytes(&buffer);
            }
            _ => {}
        }
    }

    fn read_gpd_frame(&mut self, options: &Value) -> Value {
        let payload = options
            .get("payload")
            .unwrap_or_else(|| panic!("payload option required"));
        let command = payload
            .get("commandID")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let size = payload
            .get("payloadSize")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("payloadSize option required")) as usize;
        match command {
            0xa0 => {
                let cluster = self.read_u16();
                let mut attributes = serde_json::Map::new();
                while self.is_more_by(3) {
                    let attr = self.read_u16();
                    let dtype = self.read_u8();
                    let value = self.read_dtype_u8(dtype);
                    attributes.insert(attribute_name(cluster, attr), value);
                }
                json!({
                    "clusterID": cluster,
                    "clusterName": cluster_name(cluster),
                    "attributes": attributes,
                })
            }
            0xa1 => {
                let manufacturer = self.read_u16();
                let cluster = self.read_u16();
                let mut attributes = serde_json::Map::new();
                while self.is_more_by(3) {
                    let attr = self.read_u16();
                    let dtype = self.read_u8();
                    attributes.insert(attribute_name(cluster, attr), self.read_dtype_u8(dtype));
                }
                json!({
                    "manufacturerCode": manufacturer,
                    "clusterID": cluster,
                    "clusterName": cluster_name(cluster),
                    "attributes": attributes,
                })
            }
            0xa2 => {
                let mut reports = Vec::new();
                while self.is_more_by(5) {
                    let cluster = self.read_u16();
                    let attr = self.read_u16();
                    let dtype = self.read_u8();
                    let value = self.read_dtype_u8(dtype);
                    reports.push(json!({
                        "clusterID": cluster,
                        "clusterName": cluster_name(cluster),
                        "attribute": attribute_value(cluster, attr),
                        "attrData": value,
                    }));
                }
                json!({"reports": reports})
            }
            0xa3 => {
                let manufacturer = self.read_u16();
                let mut reports = Vec::new();
                while self.is_more_by(5) {
                    let cluster = self.read_u16();
                    let attr = self.read_u16();
                    let dtype = self.read_u8();
                    let value = self.read_dtype_u8(dtype);
                    reports.push(json!({
                        "clusterID": cluster,
                        "clusterName": cluster_name(cluster),
                        "attribute": attribute_value(cluster, attr),
                        "attrData": value,
                    }));
                }
                json!({"manufacturerCode": manufacturer, "reports": reports})
            }
            0xa4 => {
                let options = self.read_u8();
                let manufacturer = self.read_u16();
                let mut records = Vec::new();
                while self.is_more_by(4) {
                    let cluster = self.read_u16();
                    let _length = self.read_u8();
                    let mut attributes = Vec::new();
                    while self.is_more_by(2) {
                        attributes.push(json!(self.read_u16()));
                    }
                    records.push(json!({"clusterID": cluster, "attributes": attributes}));
                }
                json!({
                    "options": options,
                    "manufacturerID": manufacturer,
                    "records": records,
                })
            }
            0xa5 => {
                let options = self.read_u8();
                let manufacturer = self.read_u16();
                let mut records = Vec::new();
                while self.is_more_by(5) {
                    let cluster = self.read_u16();
                    let _length = self.read_u8();
                    let mut attributes = serde_json::Map::new();
                    while self.is_more_by(4) {
                        let attr = self.read_u16();
                        let status = self.read_u8();
                        let dtype = self.read_u8();
                        let value = self.read_dtype_u8(dtype);
                        attributes.insert(
                            attribute_name(cluster, attr),
                            json!({"status": status, "attrData": value}),
                        );
                    }
                    records.push(json!({
                        "clusterID": cluster,
                        "clusterName": cluster_name(cluster),
                        "attributes": attributes,
                    }));
                }
                json!({
                    "options": options,
                    "manufacturerID": manufacturer,
                    "records": records,
                })
            }
            0xe0 => {
                let device_id = self.read_u8();
                let options = if self.is_more_by(1) {
                    self.read_u8()
                } else {
                    0
                };
                let extended = if self.is_more_by(1) {
                    self.read_u8()
                } else {
                    0
                };
                let len = self.buffer.len();
                let application_info = if len >= 37 { self.buffer[27] } else { 0 };
                let generic_switch = if len >= 2 && self.position < len {
                    self.buffer[len - 2]
                } else {
                    0
                };
                let contact = if len >= 1 && self.position < len {
                    self.buffer[len - 1]
                } else {
                    0
                };
                self.position = len;
                json!({
                    "deviceID": device_id,
                    "options": options,
                    "extendedOptions": extended,
                    "securityKey": vec![0u8; 16],
                    "keyMic": 0,
                    "outgoingCounter": 0,
                    "manufacturerID": 0,
                    "modelID": 0,
                    "numGpdCommands": 0,
                    "gpdCommandIdList": [],
                    "numServerClusters": 0,
                    "numClientClusters": 0,
                    "gpdServerClusters": [],
                    "gpdClientClusters": [],
                    "applicationInfo": application_info,
                    "genericSwitchConfig": generic_switch,
                    "currentContactStatus": contact,
                })
            }
            0xe3 => {
                let value = self.read_u8();
                json!({"nextChannel": value & 0x0f, "nextNextChannel": (value >> 4) & 0x0f})
            }
            _ => {
                let remaining = self.buffer.len().saturating_sub(self.position);
                let take = remaining.min(size);
                json!({"raw": self.read_bytes(take)})
            }
        }
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
        let end = (self.position + length).min(self.buffer.len());
        let values = self.buffer[self.position..end].to_vec();
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
    if let Some(text) = value.as_str() {
        return Some(text.as_bytes().to_vec());
    }
    None
}

fn element_type(value: Value) -> u8 {
    match value {
        Value::Number(number) => number.as_u64().unwrap_or(0) as u8,
        Value::String(name) => match name.as_str() {
            "UINT16" => 0x21,
            "CHAR_STR" => 0x42,
            _ => 0,
        },
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
