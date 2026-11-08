// Original: .upstream/zigbee2mqtt/lib/util/yaml.ts

use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Upstream `YAMLFileException`.
#[derive(Debug, Clone)]
pub struct YAMLFileException {
    pub file: PathBuf,
    pub reason: String,
}

impl YAMLFileException {
    pub fn new(reason: impl Into<String>, file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for YAMLFileException {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.file.display(), self.reason)
    }
}

impl std::error::Error for YAMLFileException {}

/// Upstream `read(file)`.
pub fn read(file: impl AsRef<Path>) -> Result<Value, YAMLFileException> {
    let file = file.as_ref();
    let content = std::fs::read_to_string(file)
        .map_err(|error| YAMLFileException::new(error.to_string(), file))?;
    let value: Value = serde_yaml::from_str(&content)
        .map_err(|error| YAMLFileException::new(error.to_string(), file))?;
    if !value.is_object() {
        return Err(YAMLFileException::new(
            format!(
                "The content of {} is expected to be an object",
                file.display()
            ),
            file,
        ));
    }
    Ok(value)
}

/// Upstream `readIfExists(file, fallback = {})`.
pub fn read_if_exists(
    file: impl AsRef<Path>,
    fallback: Option<Value>,
) -> Result<Value, YAMLFileException> {
    if file.as_ref().exists() {
        read(file)
    } else {
        Ok(fallback.unwrap_or_else(|| Value::Object(Default::default())))
    }
}

/// Upstream `writeIfChanged(file, content)`.
pub fn write_if_changed(file: impl AsRef<Path>, content: &Value) -> Result<(), YAMLFileException> {
    let file = file.as_ref();
    let before = read_if_exists(file, None)?;
    if &before != content {
        let serialized = serde_yaml::to_string(content)
            .map_err(|error| YAMLFileException::new(error.to_string(), file))?;
        std::fs::write(file, serialized)
            .map_err(|error| YAMLFileException::new(error.to_string(), file))?;
    }
    Ok(())
}

/// Upstream `updateIfChanged(file, key, value)`.
pub fn update_if_changed(
    file: impl AsRef<Path>,
    key: &str,
    value: Value,
) -> Result<(), YAMLFileException> {
    let file = file.as_ref();
    let mut content = read(file)?;
    let changed = content.get(key).map(|current| current != &value) != Some(false);
    if changed {
        content[key] = value;
        write_if_changed(file, &content)?;
    }
    Ok(())
}

#[allow(non_snake_case)]
pub fn YAMLFileException(error: YAMLFileException) -> YAMLFileException {
    error
}
