// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/index.js (`findByDevice`/`findDefinition`)
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// Lookup/loading mechanism. R1 registers no converter database (AGENTS §5 Q4),
// so with an empty registry `find_by_device` returns `None` — the upstream
// "not found" fallback. External definitions can be registered the same way
// the upstream external-converter loader would register them.

use std::collections::HashMap;
use std::sync::Arc;

use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;

use crate::definition::{Definition, DefinitionRef};
use crate::exposes::Fingerprint;

/// Upstream `normalizeModelID`.
pub fn normalize_model_id(model_id: &str) -> String {
    model_id.trim().to_lowercase()
}

fn is_fingerprint_match(fingerprint: &Fingerprint, device: &ZhDevice) -> bool {
    if let Some(model_id) = &fingerprint.model_id {
        if device.model_id().as_deref() != Some(model_id.as_str()) {
            return false;
        }
    }
    if let Some(manufacturer_name) = &fingerprint.manufacturer_name {
        if device.manufacturer_name().as_deref() != Some(manufacturer_name.as_str()) {
            return false;
        }
    }
    if let Some(r#type) = &fingerprint.type_ {
        if device.r#type().as_str() != r#type {
            return false;
        }
    }
    if let Some(endpoint_id) = fingerprint.endpoint_id {
        if device.get_endpoint(endpoint_id).is_none() {
            return false;
        }
    }
    true
}

/// Upstream definition registry + external definitions
/// (`externalDefinitionsLookup`, `definitions`).
#[derive(Default)]
pub struct DefinitionRegistry {
    definitions: Vec<DefinitionRef>,
    by_model: HashMap<String, Vec<usize>>,
    external_count: usize,
}

impl DefinitionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `addDefinition` — internal/device database registration.
    pub fn register(&mut self, definition: Definition) {
        self.insert(definition);
    }

    /// Upstream `addExternalDefinition` — `extension/externalConverters.ts`.
    pub fn register_external(&mut self, definition: Definition) {
        self.insert(definition);
        self.external_count += 1;
    }

    fn insert(&mut self, definition: Definition) {
        let definition = Arc::new(definition);
        let index = self.definitions.len();
        let models = definition.zigbee_model.clone();
        self.definitions.push(definition);
        for model in models {
            self.by_model
                .entry(normalize_model_id(&model))
                .or_default()
                .push(index);
        }
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn external_count(&self) -> usize {
        self.external_count
    }

    pub fn all(&self) -> Vec<DefinitionRef> {
        self.definitions.clone()
    }

    /// Upstream `findByDevice(device, generateForUnknown)` (`index.js:411`).
    /// `generateForUnknown` is accepted for call-site parity; `generateDefinition`
    /// is not ported yet (AGENTS §5 Q4), so the fallback is `None`.
    pub fn find_by_device(
        &self,
        device: &ZhDevice,
        _generate_for_unknown: bool,
    ) -> Option<DefinitionRef> {
        if device.r#type() == DeviceType::Coordinator {
            return None;
        }

        let model_id = normalize_model_id(device.model_id().as_deref().unwrap_or_default());
        let mut candidates: Vec<DefinitionRef> = self
            .by_model
            .get(&model_id)
            .map(|indexes| {
                indexes
                    .iter()
                    .map(|index| self.definitions[*index].clone())
                    .collect()
            })
            .unwrap_or_default();

        if candidates.is_empty() {
            return None;
        }

        let definition = if candidates.len() == 1 && !candidates[0].zigbee_model.is_empty() {
            candidates.remove(0)
        } else {
            let mut best: Option<(i32, DefinitionRef)> = None;
            for candidate in &candidates {
                for fingerprint in &candidate.fingerprint {
                    if is_fingerprint_match(fingerprint, device)
                        && best
                            .as_ref()
                            .is_none_or(|(priority, _)| fingerprint.priority > *priority)
                    {
                        best = Some((fingerprint.priority, candidate.clone()));
                    }
                }
            }

            if let Some((_, definition)) = best {
                definition
            } else {
                let matched = candidates.into_iter().find(|candidate| {
                    candidate
                        .zigbee_model
                        .iter()
                        .any(|model| normalize_model_id(model) == model_id)
                });
                matched?
            }
        };

        // Upstream `findByDevice` white-label substitution.
        for white_label in &definition.white_label {
            if white_label
                .fingerprint
                .iter()
                .any(|fingerprint| is_fingerprint_match(fingerprint, device))
            {
                let mut replaced = Definition::new(
                    &white_label.model,
                    white_label.vendor.as_deref().unwrap_or(&definition.vendor),
                    white_label
                        .description
                        .as_deref()
                        .unwrap_or(&definition.description),
                );
                replaced.zigbee_model = definition.zigbee_model.clone();
                replaced.exposes = definition.exposes.clone();
                replaced.options = definition.options.clone();
                replaced.meta = definition.meta.clone();
                replaced.endpoint = definition.endpoint.clone();
                replaced.configure = definition.configure.clone();
                replaced.version = definition.version.clone();
                replaced.ota = definition.ota.clone();
                replaced.from_zigbee = definition.from_zigbee.clone();
                replaced.to_zigbee = definition.to_zigbee.clone();
                replaced.on_event = definition.on_event.clone();
                replaced.generated = definition.generated;
                replaced.white_label = definition.white_label.clone();
                replaced.fingerprint = definition.fingerprint.clone();
                return Some(Arc::new(replaced));
            }
        }

        Some(definition)
    }

    /// Upstream `findByDefinition(definition)` (`index.js:203`).
    pub fn find_by_definition(&self, definition: &Definition) -> Option<DefinitionRef> {
        self.definitions
            .iter()
            .find(|candidate| {
                candidate.model == definition.model && candidate.vendor == definition.vendor
            })
            .cloned()
    }
}
