//! Resolver with the semantics of pinned
//! `zigbee-herdsman-converters/dist/index.js` (`getFromIndex`,
//! `findDefinition`, `findByDevice`, `isFingerprintMatch`).

use std::sync::Arc;

use regex::Regex;

use crate::model::{
    ArtifactIndex, Definition, DeviceEndpoint, DeviceProfile, DeviceType, EndpointFingerprint,
    Fingerprint, Resolution,
};

/// Upstream `normalizeModelID`: strip from the first NUL and trim.
pub fn normalize_model_id(model_id: &str) -> String {
    let stripped = model_id.split('\0').next().unwrap_or(model_id);
    stripped.trim().to_owned()
}

fn normalized_key(model_id: Option<&str>) -> String {
    match model_id {
        Some(value) if !value.is_empty() => value.to_lowercase(),
        _ => "null".to_owned(),
    }
}

fn candidates<'a>(index: &'a ArtifactIndex, model_id: Option<&str>) -> Option<&'a Vec<u32>> {
    let lookup = normalized_key(model_id);
    index
        .keys
        .get(&lookup)
        .or_else(|| index.keys.get(&normalize_model_id(&lookup)))
}

fn endpoint_id(endpoint: &DeviceEndpoint) -> Option<u8> {
    endpoint.id
}

fn endpoints_equal(left: &[DeviceEndpoint], right: &[EndpointFingerprint]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .all(|(device, fingerprint)| endpoint_id(device) == fingerprint.id)
}

fn array_equal<T: PartialEq>(left: &[T], right: &[T]) -> bool {
    left == right
}

fn regex_matches(spec: &crate::model::RegexSpec, value: &str) -> bool {
    let mut inline = String::new();
    for flag in spec.flags.chars() {
        match flag {
            'i' | 'm' | 's' => inline.push(flag),
            'u' | 'g' => {}
            _ => return false,
        }
    }
    let pattern = if inline.is_empty() {
        spec.source.clone()
    } else {
        format!("(?{inline}){}", spec.source)
    };
    Regex::new(&pattern)
        .map(|regex| regex.is_match(value))
        .unwrap_or(false)
}

pub(crate) fn fingerprint_matches(fingerprint: &Fingerprint, device: &DeviceProfile) -> bool {
    if fingerprint
        .application_version
        .is_some_and(|value| device.application_version != Some(value))
    {
        return false;
    }
    if fingerprint
        .manufacturer_id
        .is_some_and(|value| device.manufacturer_id != Some(value))
    {
        return false;
    }
    if let Some(value) = &fingerprint.r#type
        && !value.is_empty()
    {
        let device_type = match device.r#type {
            DeviceType::Router => "Router",
            DeviceType::EndDevice => "EndDevice",
            DeviceType::Coordinator => "Coordinator",
            DeviceType::Unknown => "",
        };
        if device_type != value {
            return false;
        }
    }
    if let Some(value) = &fingerprint.date_code
        && !value.is_empty()
        && device.date_code.as_deref() != Some(value)
    {
        return false;
    }
    if fingerprint
        .hardware_version
        .is_some_and(|value| device.hardware_version != Some(value))
    {
        return false;
    }
    if let Some(value) = &fingerprint.manufacturer_name
        && !value.is_empty()
        && device.manufacturer_name.as_deref() != Some(value)
    {
        return false;
    }
    if let Some(value) = &fingerprint.model_id
        && !value.is_empty()
        && device.model_id.as_deref() != Some(value)
    {
        return false;
    }
    if let Some(value) = &fingerprint.power_source
        && !value.is_empty()
        && device.power_source.as_deref() != Some(value)
    {
        return false;
    }
    if let Some(value) = &fingerprint.software_build_id
        && !value.is_empty()
        && device.software_build_id.as_deref() != Some(value)
    {
        return false;
    }
    if fingerprint
        .stack_version
        .is_some_and(|value| device.stack_version != Some(value))
    {
        return false;
    }
    if fingerprint
        .zcl_version
        .is_some_and(|value| device.zcl_version != Some(value))
    {
        return false;
    }
    if let Some(regex) = &fingerprint.ieee_addr {
        let Some(ieee) = &device.ieee_addr else {
            return false;
        };
        if !regex_matches(regex, ieee) {
            return false;
        }
    }
    if let Some(endpoints) = &fingerprint.endpoints {
        if !endpoints_equal(&device.endpoints, endpoints) {
            return false;
        }
        for (fingerprint, device_endpoint) in endpoints.iter().zip(device.endpoints.iter()) {
            let Some(id) = fingerprint.id else {
                continue;
            };
            if device_endpoint.id != Some(id)
                || fingerprint
                    .device_id
                    .is_some_and(|value| device_endpoint.device_id != Some(value))
                || fingerprint
                    .profile_id
                    .is_some_and(|value| device_endpoint.profile_id != Some(value))
            {
                return false;
            }
            if let Some(value) = &fingerprint.input_clusters
                && !array_equal(&device_endpoint.input_clusters, value)
            {
                return false;
            }
            if let Some(value) = &fingerprint.output_clusters
                && !array_equal(&device_endpoint.output_clusters, value)
            {
                return false;
            }
        }
    }
    true
}

fn white_label_model<'a>(
    definition: &'a Definition,
    device: &DeviceProfile,
) -> Option<(&'a str, Option<&'a str>, Option<&'a str>)> {
    for white in &definition.white_label {
        if white
            .fingerprints
            .iter()
            .any(|fingerprint| fingerprint_matches(fingerprint, device))
        {
            return Some((
                white.model.as_str(),
                white.vendor.as_deref(),
                white.description.as_deref(),
            ));
        }
    }
    None
}

fn resolution(
    index: &ArtifactIndex,
    definition: &Definition,
    _id: u32,
    device: &DeviceProfile,
) -> Option<Resolution> {
    let file = index.files.get(definition.file as usize)?.clone();
    let white = white_label_model(definition, device);
    let (model, vendor, description) = match white {
        Some((model, vendor, description)) => (
            model.to_owned(),
            vendor.unwrap_or(&definition.vendor).to_owned(),
            description.unwrap_or(&definition.description).to_owned(),
        ),
        None => (
            definition.model.clone(),
            definition.vendor.clone(),
            definition.description.clone(),
        ),
    };
    Some(Resolution {
        definition: Arc::new(definition.clone()),
        file,
        file_index: definition.index,
        model,
        vendor,
        description,
        version: definition.version.clone(),
        white_label: white.is_some(),
    })
}

pub(crate) fn resolve(index: &ArtifactIndex, device: &DeviceProfile) -> Option<Resolution> {
    if device.r#type == DeviceType::Coordinator {
        return None;
    }
    let ids = candidates(index, device.model_id.as_deref())?;
    if ids.is_empty() {
        return None;
    }
    let first = index.definitions.get(ids[0] as usize)?;
    if ids.len() == 1 && !first.zigbee_model.is_empty() {
        return resolution(index, first, ids[0], device);
    }

    let mut matched: Option<(&Definition, u32, i32)> = None;
    for id in ids {
        let Some(definition) = index.definitions.get(*id as usize) else {
            continue;
        };
        for fingerprint in &definition.fingerprints {
            let priority = fingerprint.priority.unwrap_or(0);
            if fingerprint_matches(fingerprint, device)
                && matched
                    .as_ref()
                    .map(|(_, _, best)| priority > *best)
                    .unwrap_or(true)
            {
                matched = Some((definition, *id, priority));
            }
        }
    }
    if let Some((definition, id, _)) = matched {
        return resolution(index, definition, id, device);
    }

    let model_id = device.model_id.as_deref()?;
    let normalized = normalize_model_id(model_id);
    for id in ids {
        let Some(definition) = index.definitions.get(*id as usize) else {
            continue;
        };
        if definition
            .zigbee_model
            .iter()
            .any(|model| model == model_id || model == &normalized)
        {
            return resolution(index, definition, *id, device);
        }
    }
    None
}
