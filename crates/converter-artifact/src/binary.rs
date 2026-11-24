//! `z2idx1` binary reader. All counts and offsets are bounds-checked; the
//! parser never trusts the manifest and never executes module code.

use std::collections::BTreeMap;

use crate::model::{
    ArtifactIndex, Definition, EndpointFingerprint, FileRecord, Fingerprint, RegexSpec, WhiteLabel,
};
use crate::{ArtifactError, Result};

const MAGIC: &[u8; 6] = b"Z2IDX1";
const VERSION: u16 = 1;
const NONE: u32 = 0xffff_ffff;

struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| ArtifactError("index offset overflow".to_owned()))?;
        if end > self.data.len() {
            return Err(ArtifactError(format!(
                "truncated index: need {length} bytes at {}",
                self.offset
            )));
        }
        let value = &self.data[self.offset..end];
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

fn string(strings: &[String], id: u32) -> Result<Option<String>> {
    if id == NONE {
        return Ok(None);
    }
    strings
        .get(id as usize)
        .cloned()
        .map(Some)
        .ok_or_else(|| ArtifactError(format!("invalid string id {id}")))
}

fn required_string(strings: &[String], id: u32, field: &str) -> Result<String> {
    string(strings, id)?.ok_or_else(|| ArtifactError(format!("missing {field}")))
}

fn optional_u32(value: u32) -> Option<u32> {
    (value != NONE).then_some(value)
}

fn fingerprint(reader: &mut Reader<'_>, strings: &[String]) -> Result<Fingerprint> {
    let model_id = string(strings, reader.u32()?)?;
    let manufacturer_name = string(strings, reader.u32()?)?;
    let manufacturer_id = optional_u32(reader.u32()?);
    let r#type = string(strings, reader.u32()?)?;
    let power_source = string(strings, reader.u32()?)?;
    let application_version = optional_u32(reader.u32()?);
    let zcl_version = optional_u32(reader.u32()?);
    let stack_version = optional_u32(reader.u32()?);
    let hardware_version = optional_u32(reader.u32()?);
    let date_code = string(strings, reader.u32()?)?;
    let software_build_id = string(strings, reader.u32()?)?;
    let priority = Some(reader.i32()?);
    let ieee_source = string(strings, reader.u32()?)?;
    let ieee_flags = string(strings, reader.u32()?)?;
    let ieee_addr = match (ieee_source, ieee_flags) {
        (Some(source), Some(flags)) => Some(RegexSpec { source, flags }),
        (None, None) => None,
        _ => return Err(ArtifactError("partial ieeeAddr regex".to_owned())),
    };
    let endpoint_count_raw = reader.u32()?;
    let endpoint_count = if endpoint_count_raw == NONE {
        0
    } else {
        endpoint_count_raw as usize
    };
    if endpoint_count > 10_000 {
        return Err(ArtifactError("implausible endpoint count".to_owned()));
    }
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for _ in 0..endpoint_count {
        let id = optional_u32(reader.u32()?).map(|value| value as u8);
        let profile_id = optional_u32(reader.u32()?).map(|value| value as u16);
        let device_id = optional_u32(reader.u32()?).map(|value| value as u16);
        let input_raw = reader.u16()?;
        let input_count = if input_raw == u16::MAX {
            0
        } else {
            input_raw as usize
        };
        let mut input_clusters = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            input_clusters.push(reader.u16()?);
        }
        let output_raw = reader.u16()?;
        let output_count = if output_raw == u16::MAX {
            0
        } else {
            output_raw as usize
        };
        let mut output_clusters = Vec::with_capacity(output_count);
        for _ in 0..output_count {
            output_clusters.push(reader.u16()?);
        }
        endpoints.push(EndpointFingerprint {
            id,
            profile_id,
            device_id,
            input_clusters: (input_raw != u16::MAX).then_some(input_clusters),
            output_clusters: (output_raw != u16::MAX).then_some(output_clusters),
        });
    }
    Ok(Fingerprint {
        application_version,
        manufacturer_id,
        r#type,
        date_code,
        hardware_version,
        manufacturer_name,
        model_id,
        power_source,
        software_build_id,
        stack_version,
        zcl_version,
        ieee_addr,
        endpoints: (endpoint_count_raw != NONE).then_some(endpoints),
        priority,
    })
}

fn definition(reader: &mut Reader<'_>, strings: &[String]) -> Result<Definition> {
    let file = reader.u32()?;
    let index = reader.u32()?;
    let model = required_string(strings, reader.u32()?, "definition model")?;
    let vendor = required_string(strings, reader.u32()?, "definition vendor")?;
    let description = required_string(strings, reader.u32()?, "definition description")?;
    let version = required_string(strings, reader.u32()?, "definition version")?;
    let zigbee_count = reader.u16()? as usize;
    let mut zigbee_model = Vec::with_capacity(zigbee_count);
    for _ in 0..zigbee_count {
        zigbee_model.push(required_string(strings, reader.u32()?, "zigbeeModel")?);
    }
    let fingerprint_count = reader.u16()? as usize;
    let mut fingerprints = Vec::with_capacity(fingerprint_count);
    for _ in 0..fingerprint_count {
        fingerprints.push(fingerprint(reader, strings)?);
    }
    let white_count = reader.u16()? as usize;
    let mut white_label = Vec::with_capacity(white_count);
    for _ in 0..white_count {
        let model = required_string(strings, reader.u32()?, "whiteLabel model")?;
        let vendor = string(strings, reader.u32()?)?;
        let description = string(strings, reader.u32()?)?;
        let fp_count = reader.u16()? as usize;
        let mut fingerprints = Vec::with_capacity(fp_count);
        for _ in 0..fp_count {
            fingerprints.push(fingerprint(reader, strings)?);
        }
        white_label.push(WhiteLabel {
            model,
            vendor,
            description,
            fingerprints,
        });
    }
    Ok(Definition {
        file,
        index,
        model,
        vendor,
        description,
        version,
        zigbee_model,
        fingerprints,
        white_label,
    })
}

pub fn from_binary(data: &[u8]) -> Result<ArtifactIndex> {
    let mut reader = Reader::new(data);
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(ArtifactError("invalid z2idx1 magic".to_owned()));
    }
    let version = reader.u16()?;
    if version != VERSION {
        return Err(ArtifactError(format!(
            "unsupported z2idx1 version {version}"
        )));
    }
    let _flags = reader.u8()?;
    let _reserved = reader.u8()?;
    let string_count = reader.u32()? as usize;
    let file_count = reader.u32()? as usize;
    let definition_count = reader.u32()? as usize;
    let key_count = reader.u32()? as usize;
    let candidate_count = reader.u32()? as usize;
    if string_count > 1_000_000
        || file_count > 100_000
        || definition_count > 100_000
        || key_count > 1_000_000
        || candidate_count > 10_000_000
    {
        return Err(ArtifactError("implausible index counts".to_owned()));
    }
    let mut strings = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        let length = reader.u32()? as usize;
        if length > 1 << 20 {
            return Err(ArtifactError("implausible string length".to_owned()));
        }
        let bytes = reader.take(length)?;
        strings.push(
            std::str::from_utf8(bytes)
                .map_err(|error| ArtifactError(format!("invalid UTF-8 in index: {error}")))?
                .to_owned(),
        );
    }
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let path = required_string(&strings, reader.u32()?, "file path")?;
        let bytes = reader.u64()?;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(reader.take(32)?);
        files.push(FileRecord {
            path,
            bytes,
            sha256: hash.iter().map(|byte| format!("{byte:02x}")).collect(),
        });
    }
    let mut definitions = Vec::with_capacity(definition_count);
    for _ in 0..definition_count {
        definitions.push(definition(&mut reader, &strings)?);
    }
    let mut keys = BTreeMap::new();
    let mut total_candidates = 0usize;
    for _ in 0..key_count {
        let key = required_string(&strings, reader.u32()?, "lookup key")?;
        let count = reader.u32()? as usize;
        total_candidates = total_candidates
            .checked_add(count)
            .ok_or_else(|| ArtifactError("candidate count overflow".to_owned()))?;
        if total_candidates > candidate_count {
            return Err(ArtifactError(
                "candidate table exceeds header count".to_owned(),
            ));
        }
        let mut candidates = Vec::with_capacity(count);
        for _ in 0..count {
            let candidate = reader.u32()?;
            if candidate as usize >= definitions.len() {
                return Err(ArtifactError(format!(
                    "key {key:?} references missing definition {candidate}"
                )));
            }
            candidates.push(candidate);
        }
        keys.insert(key, candidates);
    }
    if total_candidates != candidate_count {
        return Err(ArtifactError(format!(
            "candidate count mismatch: header {candidate_count}, body {total_candidates}"
        )));
    }
    if reader.offset != data.len() {
        return Err(ArtifactError(format!(
            "trailing bytes after index: {}",
            data.len() - reader.offset
        )));
    }
    for (index, definition) in definitions.iter().enumerate() {
        if definition.file as usize >= files.len() {
            return Err(ArtifactError(format!(
                "definition {index} references missing file {}",
                definition.file
            )));
        }
    }
    Ok(ArtifactIndex {
        files,
        definitions,
        keys,
    })
}
