// Original: .upstream/zigbee2mqtt/lib/util/settings.schema.json
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Rust replacement for the Ajv validators compiled in
// `lib/util/settings.ts:20-27`:
//   - `ajvSetting`                       (schema, `allErrors: true`)
//   - `ajvRestartRequired`               (schema + failing `requiresRestart` keyword)
//   - `ajvRestartRequiredDeviceOptions`  (`schema.definitions.device`)
//   - `ajvRestartRequiredGroupOptions`   (`schema.definitions.group`)
//
// Ajv exception text is reproduced (`must be string`, `must have required
// property 'mqtt'`, `must be equal to one of the allowed values`, ...).
// Only keywords used by the pinned schema are interpreted; annotation
// keywords (`title`, `description`, `default`, `examples`, `readOnly`) are
// ignored like Ajv does.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

const SCHEMA_JSON: &str = include_str!("settings.schema.json");

/// One validator error, mirroring an Ajv `ErrorObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub keyword: &'static str,
    /// JSON pointer, e.g. `/advanced/log_level` (Ajv `instancePath`).
    pub instance_path: String,
    /// Ajv `message`, e.g. `must be string`.
    pub message: String,
}

impl SchemaError {
    fn new(keyword: &'static str, instance_path: &str, message: String) -> Self {
        Self {
            keyword,
            instance_path: instance_path.to_string(),
            message,
        }
    }

    /// Upstream `` `${error.instancePath.substring(1)} ${error.message}` ``
    /// (`settings.ts:273`, `settings.ts:329`).
    pub fn render(&self) -> String {
        format!(
            "{} {}",
            self.instance_path
                .strip_prefix('/')
                .unwrap_or(&self.instance_path),
            self.message
        )
    }
}

/// Parsed `settings.schema.json`.
pub fn schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(SCHEMA_JSON).expect("settings.schema.json must be valid JSON")
    })
}

/// Upstream `ajvSetting(value)`; `true` when there are no errors.
pub fn is_valid(instance: &Value) -> bool {
    validate(instance).is_empty()
}

/// Upstream `ajvSetting.errors` (`settings.ts:271-273`).
pub fn validate(instance: &Value) -> Vec<SchemaError> {
    let root = schema();
    let mut errors = Vec::new();
    visit(root, instance, "", root, &mut errors);
    errors
}

/// Upstream `validateNonRequired` (`settings.ts:322-333`): all schema errors
/// except `required`, rendered as `"<instancePath> <message>"`.
pub fn validate_non_required(instance: &Value) -> Vec<SchemaError> {
    validate(instance)
        .into_iter()
        .filter(|error| error.keyword != "required")
        .collect()
}

/// Upstream `ajvRestartRequired.errors.find(e => e.keyword === "requiresRestart")`
/// (`settings.ts:505-507`). The keyword is `validate: (s) => !s`, where Ajv
/// passes the *schema* value (`true`), so the error fires for every present
/// property carrying `requiresRestart`, whatever its value is.
pub fn restart_required(instance: &Value) -> bool {
    fn walk(schema: &Value, instance: &Value, root: &Value) -> bool {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return resolve_pointer(root, reference)
                .is_some_and(|target| walk(target, instance, root));
        }

        if schema.get("requiresRestart").and_then(Value::as_bool) == Some(true) {
            return true;
        }

        match instance {
            Value::Object(object) => {
                if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                    for (name, subschema) in properties {
                        if let Some(value) = object.get(name) {
                            if walk(subschema, value, root) {
                                return true;
                            }
                        }
                    }
                }

                if let Some(patterns) = schema.get("patternProperties").and_then(Value::as_object) {
                    for (pattern, subschema) in patterns {
                        for (key, value) in object {
                            if matches_pattern(pattern, key) && walk(subschema, value, root) {
                                return true;
                            }
                        }
                    }
                }

                if let Some(additional) = schema.get("additionalProperties") {
                    if additional.is_object() {
                        let properties = schema.get("properties").and_then(Value::as_object);
                        let patterns = schema.get("patternProperties").and_then(Value::as_object);
                        for (key, value) in object {
                            let known = properties.is_some_and(|map| map.contains_key(key));
                            let matched = patterns.is_some_and(|map| {
                                map.keys().any(|pattern| matches_pattern(pattern, key))
                            });
                            if !known && !matched && walk(additional, value, root) {
                                return true;
                            }
                        }
                    }
                }

                false
            }
            Value::Array(items) => {
                if let Some(items_schema) = schema.get("items") {
                    match items_schema {
                        Value::Array(schemas) => schemas
                            .iter()
                            .take(items.len())
                            .zip(items)
                            .any(|(subschema, value)| walk(subschema, value, root)),
                        _ => items.iter().any(|value| walk(items_schema, value, root)),
                    }
                } else {
                    false
                }
            }
            _ => {
                // `oneOf` branches can still carry `requiresRestart` properties.
                schema
                    .get("oneOf")
                    .and_then(Value::as_array)
                    .is_some_and(|branches| {
                        branches.iter().any(|branch| walk(branch, instance, root))
                    })
            }
        }
    }

    let root = schema();
    walk(root, instance, root)
}

fn visit(
    schema: &Value,
    instance: &Value,
    path: &str,
    root: &Value,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if let Some(target) = resolve_pointer(root, reference) {
            visit(target, instance, path, root, errors);
        }
        return;
    }

    if let Some(type_spec) = schema.get("type") {
        if !matches_type(type_spec, instance) {
            errors.push(SchemaError::new(
                "type",
                path,
                format!("must be {}", type_names(type_spec)),
            ));
        }
    }

    if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
        if !allowed.iter().any(|value| value == instance) {
            errors.push(SchemaError::new(
                "enum",
                path,
                "must be equal to one of the allowed values".to_string(),
            ));
        }
    }

    match instance {
        Value::Object(object) => visit_object(schema, object, path, root, errors),
        Value::Array(items) => visit_array(schema, items, path, root, errors),
        Value::Number(number) => {
            if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
                if number.as_f64().is_some_and(|value| value < minimum) {
                    errors.push(SchemaError::new(
                        "minimum",
                        path,
                        format!("must be >= {}", format_number(minimum)),
                    ));
                }
            }
            if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
                if number.as_f64().is_some_and(|value| value > maximum) {
                    errors.push(SchemaError::new(
                        "maximum",
                        path,
                        format!("must be <= {}", format_number(maximum)),
                    ));
                }
            }
        }
        Value::String(text) => {
            if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
                if !matches_pattern(pattern, text) {
                    errors.push(SchemaError::new(
                        "pattern",
                        path,
                        format!("must match pattern {pattern:?}"),
                    ));
                }
            }
        }
        _ => {}
    }

    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let mut matched = 0;
        let mut branch_errors = Vec::new();
        for branch in branches {
            let mut current = Vec::new();
            visit(branch, instance, path, root, &mut current);
            if current.is_empty() {
                matched += 1;
            } else {
                branch_errors.extend(current);
            }
        }
        if matched == 0 {
            errors.extend(branch_errors);
        }
        if matched != 1 {
            errors.push(SchemaError::new(
                "oneOf",
                path,
                "must match exactly one schema in oneOf".to_string(),
            ));
        }
    }
}

fn visit_object(
    schema: &Value,
    object: &serde_json::Map<String, Value>,
    path: &str,
    root: &Value,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(name) {
                errors.push(SchemaError::new(
                    "required",
                    path,
                    format!("must have required property '{name}'"),
                ));
            }
        }
    }

    if let Some(property_names) = schema.get("propertyNames") {
        for key in object.keys() {
            let mut key_errors = Vec::new();
            // Ajv reports `propertyNames` sub-errors (e.g. `pattern`) at the
            // parent instance path.
            visit(
                property_names,
                &Value::String(key.clone()),
                path,
                root,
                &mut key_errors,
            );
            if !key_errors.is_empty() {
                errors.extend(key_errors);
                errors.push(SchemaError::new(
                    "propertyNames",
                    path,
                    "property name must be valid".to_string(),
                ));
            }
        }
    }

    let properties = schema.get("properties").and_then(Value::as_object);
    let patterns = schema.get("patternProperties").and_then(Value::as_object);

    if let Some(additional) = schema.get("additionalProperties") {
        for (key, value) in object {
            let known = properties.is_some_and(|map| map.contains_key(key));
            let matched =
                patterns.is_some_and(|map| map.keys().any(|pattern| matches_pattern(pattern, key)));
            if known || matched {
                continue;
            }
            match additional {
                Value::Bool(false) => errors.push(SchemaError::new(
                    "additionalProperties",
                    path,
                    "must NOT have additional properties".to_string(),
                )),
                Value::Object(_) => {
                    visit(additional, value, &child_path(path, key), root, errors);
                }
                _ => {}
            }
        }
    }

    if let Some(properties) = properties {
        for (name, subschema) in properties {
            if let Some(value) = object.get(name) {
                visit(subschema, value, &child_path(path, name), root, errors);
            }
        }
    }

    if let Some(patterns) = patterns {
        for (key, value) in object {
            for (pattern, subschema) in patterns {
                if matches_pattern(pattern, key) {
                    visit(subschema, value, &child_path(path, key), root, errors);
                }
            }
        }
    }
}

fn visit_array(
    schema: &Value,
    items: &[Value],
    path: &str,
    root: &Value,
    errors: &mut Vec<SchemaError>,
) {
    let Some(items_schema) = schema.get("items") else {
        return;
    };

    match items_schema {
        Value::Array(schemas) => {
            for (index, item) in items.iter().enumerate() {
                if let Some(subschema) = schemas.get(index) {
                    visit(
                        subschema,
                        item,
                        &child_path(path, &index.to_string()),
                        root,
                        errors,
                    );
                }
            }
        }
        _ => {
            for (index, item) in items.iter().enumerate() {
                visit(
                    items_schema,
                    item,
                    &child_path(path, &index.to_string()),
                    root,
                    errors,
                );
            }
        }
    }
}

fn child_path(path: &str, key: &str) -> String {
    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn resolve_pointer<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix('#')?;
    if pointer.is_empty() {
        return Some(root);
    }
    let mut current = root;
    for token in pointer.strip_prefix('/')?.split('/') {
        let token = token.replace("~1", "/").replace("~0", "~");
        current = current.get(&token)?;
    }
    Some(current)
}

fn matches_type(type_spec: &Value, instance: &Value) -> bool {
    match type_spec {
        Value::String(name) => matches_single_type(name, instance),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .any(|name| matches_single_type(name, instance)),
        // Annotations (or property schemas) that are not a type constraint.
        _ => true,
    }
}

fn matches_single_type(name: &str, instance: &Value) -> bool {
    match name {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "number" => instance.is_number(),
        "integer" => instance.as_f64().is_some_and(|value| value.fract() == 0.0),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        _ => true,
    }
}

fn type_names(type_spec: &Value) -> String {
    match type_spec {
        Value::String(name) => name.clone(),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn matches_pattern(pattern: &str, text: &str) -> bool {
    fn compiled(pattern: &str) -> regex::bytes::Regex {
        // JS `\d`/`\w` are ASCII-only unless the `u` flag is set; byte regexes
        // keep that behaviour while still accepting `.` over UTF-8 input.
        regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .build()
            .expect("settings.schema.json patterns are valid")
    }

    static CACHE: OnceLock<Mutex<HashMap<String, regex::bytes::Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache
        .entry(pattern.to_string())
        .or_insert_with(|| compiled(pattern))
        .is_match(text.as_bytes())
}

/// Upstream `settings.schema.json.definitions.<kind>.properties` keys.
pub fn definition_property_names(kind: &str) -> Vec<String> {
    let root = schema();
    let Some(definition) = resolve_pointer(root, &format!("#/definitions/{kind}")) else {
        return Vec::new();
    };
    definition
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

/// Upstream device/group option validators that only mark `requiresRestart`.
pub fn definition_restart_required(kind: &str, options: &Value) -> bool {
    let root = schema();
    let Some(definition) = resolve_pointer(root, &format!("#/definitions/{kind}")) else {
        return false;
    };
    let Some(properties) = definition.get("properties").and_then(Value::as_object) else {
        return false;
    };
    properties.iter().any(|(name, subschema)| {
        subschema.get("requiresRestart").and_then(Value::as_bool) == Some(true)
            && options.get(name).is_some()
    })
}
