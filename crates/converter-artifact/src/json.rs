use std::collections::BTreeMap;

use serde::Deserialize;

use crate::model::{ArtifactIndex, Definition, FileRecord};
use crate::{ArtifactError, Result};

#[derive(Deserialize)]
struct JsonIndex {
    files: Vec<FileRecord>,
    definitions: Vec<Definition>,
    keys: BTreeMap<String, Vec<u32>>,
}

pub fn from_json(data: &[u8]) -> Result<ArtifactIndex> {
    let parsed: JsonIndex = serde_json::from_slice(data)
        .map_err(|error| ArtifactError(format!("invalid index JSON: {error}")))?;
    validate(&parsed.files, &parsed.definitions, &parsed.keys)?;
    Ok(ArtifactIndex {
        files: parsed.files,
        definitions: parsed.definitions,
        keys: parsed.keys,
    })
}

fn validate(
    files: &[FileRecord],
    definitions: &[Definition],
    keys: &BTreeMap<String, Vec<u32>>,
) -> Result<()> {
    for (index, definition) in definitions.iter().enumerate() {
        if definition.file as usize >= files.len() {
            return Err(ArtifactError(format!(
                "definition {index} references missing file id {}",
                definition.file
            )));
        }
    }
    for (key, candidates) in keys {
        for candidate in candidates {
            if *candidate as usize >= definitions.len() {
                return Err(ArtifactError(format!(
                    "key {key:?} references missing definition {candidate}"
                )));
            }
        }
    }
    Ok(())
}
