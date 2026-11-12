// Original: .upstream/zigbee-herdsman/src/controller/helpers/zclFrameConverter.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::panic::AssertUnwindSafe;

use serde_json::{Map, Value};

use crate::utils::logger::logger;
use crate::zspec::zcl::definition::manufacturer_code::ManufacturerCode;
use crate::zspec::zcl::utils::{CustomClusters, Utils};
use crate::zspec::zcl::zcl_frame::Frame;

const NS: &str = "zh:controller:zcl";

/// Upstream `LEGRAND_GROUP_MANUF_CODE`.
const LEGRAND_GROUP_MANUF_CODE: u16 = ManufacturerCode::LEGRAND_GROUP;

/// Upstream `attributeKeyValue(frame, deviceManufacturerID, customClusters)` —
/// zclFrameConverter.ts:18.
pub fn attribute_key_value(
    frame: &Frame,
    device_manufacturer_id: Option<u16>,
    custom_clusters: &CustomClusters,
) -> Value {
    let mut payload = Map::new();
    let legrand_workaround = frame.header.manufacturer_code.is_none()
        && device_manufacturer_id == Some(LEGRAND_GROUP_MANUF_CODE);
    let cluster = if legrand_workaround {
        Utils::get_cluster(
            frame.cluster.name.clone(),
            device_manufacturer_id,
            custom_clusters,
        )
    } else {
        Utils::get_cluster(
            frame.cluster.ID,
            frame.header.manufacturer_code,
            custom_clusters,
        )
    };
    let manufacturer_code = if legrand_workaround {
        device_manufacturer_id
    } else {
        frame.header.manufacturer_code
    };

    let items = frame.payload.as_array().cloned().unwrap_or_default();
    for item in items {
        // Per ZCL spec, non-success `readRsp` records carry no value.
        if let Some(status) = item.get("status").and_then(Value::as_u64) {
            if status != 0 {
                continue;
            }
        }

        let Some(attribute_id) = item.get("attrId").and_then(Value::as_u64) else {
            continue;
        };
        let attribute =
            Utils::get_cluster_attribute(&cluster, attribute_id as u32, manufacturer_code);

        if let Some(attribute) = attribute {
            let attr_data = item.get("attrData").cloned().unwrap_or(Value::Null);
            let processed = std::panic::catch_unwind(AssertUnwindSafe(|| {
                Utils::process_attribute_post_read(
                    &attribute,
                    crate::zspec::zcl::utils::ZclValue::from_json(&attr_data),
                )
            }));
            match processed {
                Ok(value) => {
                    payload.insert(attribute.name.clone(), value.to_json());
                }
                Err(payload) => {
                    let message = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| {
                            payload
                                .downcast_ref::<&str>()
                                .map(|value| value.to_string())
                        })
                        .unwrap_or_default();
                    logger().debug(
                        &format!(
                            "Ignoring attribute {} from response: Error: {message}",
                            attribute.name
                        ),
                        NS,
                    );
                }
            }
        } else {
            payload.insert(
                attribute_id.to_string(),
                item.get("attrData").cloned().unwrap_or(Value::Null),
            );
        }
    }

    Value::Object(payload)
}

/// Upstream `attributeList(frame, deviceManufacturerID, customClusters)` —
/// zclFrameConverter.ts:48.
pub fn attribute_list(
    frame: &Frame,
    device_manufacturer_id: Option<u16>,
    custom_clusters: &CustomClusters,
) -> Vec<Value> {
    let mut payload = Vec::new();
    let legrand_workaround = frame.header.manufacturer_code.is_none()
        && device_manufacturer_id == Some(LEGRAND_GROUP_MANUF_CODE);
    let cluster = if legrand_workaround {
        Utils::get_cluster(
            frame.cluster.name.clone(),
            device_manufacturer_id,
            custom_clusters,
        )
    } else {
        Utils::get_cluster(
            frame.cluster.ID,
            frame.header.manufacturer_code,
            custom_clusters,
        )
    };
    let manufacturer_code = if legrand_workaround {
        device_manufacturer_id
    } else {
        frame.header.manufacturer_code
    };

    let items = frame.payload.as_array().cloned().unwrap_or_default();
    for item in items {
        let Some(attribute_id) = item.get("attrId").and_then(Value::as_u64) else {
            continue;
        };
        let attribute =
            Utils::get_cluster_attribute(&cluster, attribute_id as u32, manufacturer_code);
        payload.push(match attribute {
            Some(attribute) => Value::String(attribute.name),
            None => Value::Number(attribute_id.into()),
        });
    }

    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zspec::zcl::zcl_frame::Frame;
    use serde_json::json;

    fn frame(payload: Value) -> Frame {
        Frame::create(0, 0, true, None, 1, "read", 6, payload, json!({}), 0)
    }

    #[test]
    fn maps_known_attributes_and_keeps_unknown_ids() {
        let custom = CustomClusters::empty();
        let converted = attribute_key_value(
            &frame(json!([
                {"attrId": 0, "attrData": true},
                {"attrId": 0x1234, "attrData": 7}
            ])),
            None,
            &custom,
        );
        assert_eq!(converted["onOff"], json!(true));
        assert_eq!(converted["4660"], json!(7));

        let list = attribute_list(
            &frame(json!([{"attrId": 0}, {"attrId": 0x1234}])),
            None,
            &custom,
        );
        assert_eq!(list, vec![json!("onOff"), json!(4660)]);
    }

    #[test]
    fn skips_non_success_read_response_records() {
        let custom = CustomClusters::empty();
        let converted = attribute_key_value(
            &frame(json!([
                {"attrId": 0, "status": 1, "attrData": true},
                {"attrId": 0, "status": 0, "attrData": false}
            ])),
            None,
            &custom,
        );
        assert_eq!(converted["onOff"], json!(false));
    }
}
