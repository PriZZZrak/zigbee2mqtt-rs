//! `js-yaml.dump`-compatible YAML writer for settings values.
//!
//! Upstream `lib/util/yaml.ts` writes with `dump(content)` and the default
//! options (indent 2, indented block sequences, line width 80). `serde_yaml_ng`
//! is a reader here; its emitter uses a different sequence indentation and
//! scalar-style rules, so writing a config with it changes the bytes even when
//! the values are unchanged. This module reproduces the observable output for
//! the JSON-shaped settings tree (maps, sequences, strings, numbers, booleans,
//! null), including the js-yaml 1.1 implicit-scalar quoting and line folding.

use serde_json::Value;

/// Upstream `dump(content)` (no custom options).
pub fn dump(value: &Value) -> String {
    let mut output = String::new();
    write_value(value, 0, &mut output);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn write_value(value: &Value, indent: usize, output: &mut String) {
    match value {
        Value::Object(map) if !map.is_empty() => write_mapping(map, indent, output, false),
        Value::Array(items) if !items.is_empty() => write_sequence(items, indent, output, false),
        _ => {
            output.push_str(&" ".repeat(indent));
            output.push_str(&inline(value, indent));
            output.push('\n');
        }
    }
}

fn write_mapping(
    map: &serde_json::Map<String, Value>,
    indent: usize,
    output: &mut String,
    first_inline: bool,
) {
    for (index, (key, value)) in map.iter().enumerate() {
        if !(first_inline && index == 0) {
            output.push_str(&" ".repeat(indent));
        }
        let key_text = render_string(key, indent, true);
        output.push_str(&key_text);
        output.push(':');
        match value {
            Value::Object(child) if !child.is_empty() => {
                output.push('\n');
                write_mapping(child, indent + 2, output, false);
            }
            Value::Array(child) if !child.is_empty() => {
                output.push('\n');
                write_sequence(child, indent + 2, output, false);
            }
            _ => {
                output.push(' ');
                output.push_str(&inline(value, indent + 2));
                output.push('\n');
            }
        }
    }
}

fn write_sequence(items: &[Value], indent: usize, output: &mut String, first_inline: bool) {
    for (index, item) in items.iter().enumerate() {
        if !(first_inline && index == 0) {
            output.push_str(&" ".repeat(indent));
        }
        output.push('-');
        match item {
            Value::Object(map) if !map.is_empty() => {
                output.push(' ');
                write_mapping(map, indent + 2, output, true);
            }
            Value::Array(child) if !child.is_empty() => {
                output.push(' ');
                write_sequence(child, indent + 2, output, true);
            }
            _ => {
                output.push(' ');
                output.push_str(&inline(item, indent + 2));
                output.push('\n');
            }
        }
    }
}

fn inline(value: &Value, content_indent: usize) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(value) => render_string(value, content_indent, false),
        Value::Array(_) => "[]".to_string(),
        Value::Object(_) => "{}".to_string(),
    }
}

fn render_string(value: &str, content_indent: usize, is_key: bool) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value.chars().any(|character| {
        character != '\n'
            && (character == '\t'
                || character == '\r'
                || (character as u32) < 0x20
                || character == '\u{7f}')
    }) {
        return double_quoted(value);
    }
    if value.contains('\n') {
        if is_key {
            return double_quoted(value);
        }
        return block_literal(value, content_indent);
    }
    let quoted = needs_quotes(value);
    if !is_key
        && !quoted
        && value.contains(' ')
        && value.chars().count() > 80usize.saturating_sub(content_indent)
    {
        return folded_block(value, content_indent);
    }
    if quoted {
        if value.contains('\'') {
            return format!("'{}'", value.replace('\'', "''"));
        }
        return format!("'{value}'");
    }
    value.to_string()
}

fn needs_quotes(value: &str) -> bool {
    let first = value.chars().next().unwrap_or('\0');
    if first.is_whitespace() {
        return true;
    }
    if matches!(
        first,
        '#' | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
            | '['
            | ']'
            | '{'
            | '}'
            | ','
    ) {
        return true;
    }
    if matches!(first, '-' | '?' | ':') {
        let rest = value.chars().nth(1);
        if rest.is_none() || rest.is_some_and(char::is_whitespace) {
            return true;
        }
    }
    if value.ends_with(' ') || value.ends_with('\t') {
        return true;
    }
    if value.contains(": ") || value.contains(" #") {
        return true;
    }
    if value == "~" || value == "=" {
        return true;
    }
    if value.starts_with("---") || value.starts_with("...") {
        let rest = value.chars().nth(3);
        if rest.is_none() || rest.is_some_and(char::is_whitespace) {
            return true;
        }
    }
    is_implicit_scalar(value)
}

fn is_implicit_scalar(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "y" | "yes" | "n" | "no" | "true" | "false" | "on" | "off" | "null"
    ) {
        return true;
    }
    if value == "~" {
        return true;
    }
    if is_number_like(value) {
        return true;
    }
    is_timestamp_like(value)
}

fn is_number_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if matches!(lower.as_str(), ".inf" | "+.inf" | "-.inf" | ".nan") {
        return true;
    }
    let unsigned = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    if let Some(hex) = unsigned.strip_prefix("0x") {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    if let Some(octal) = unsigned.strip_prefix("0o") {
        return !octal.is_empty() && octal.chars().all(|c| ('0'..='7').contains(&c) || c == '_');
    }
    if let Some(binary) = unsigned.strip_prefix("0b") {
        return !binary.is_empty() && binary.chars().all(|c| matches!(c, '0' | '1' | '_'));
    }
    // Sexagesimal integers (`12:34:56`) are still numbers in the 1.1 schema.
    if unsigned.contains(':') {
        let mut parts = unsigned.split(':');
        if parts.clone().count() > 1 {
            return parts.all(|part| {
                !part.is_empty() && part.chars().all(|c| c.is_ascii_digit() || c == '_')
            });
        }
    }
    if unsigned.parse::<f64>().is_ok() {
        return unsigned.chars().any(|c| c.is_ascii_digit());
    }
    false
}

fn is_timestamp_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn double_quoted(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '\0' => result.push_str("\\0"),
            '\u{7}' => result.push_str("\\a"),
            '\u{8}' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\u{b}' => result.push_str("\\v"),
            '\u{c}' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            '\u{1b}' => result.push_str("\\e"),
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{85}' => result.push_str("\\N"),
            '\u{a0}' => result.push_str("\\_"),
            '\u{2028}' => result.push_str("\\L"),
            '\u{2029}' => result.push_str("\\P"),
            character if (character as u32) < 0x20 || character == '\u{7f}' => {
                result.push_str(&format!("\\x{:02X}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

fn block_literal(value: &str, content_indent: usize) -> String {
    let clip = value.ends_with('\n');
    let double_clip = value.ends_with("\n\n") || value == "\n";
    // js-yaml's `shiftOfContent - shiftOfParent` with the default indent 2.
    let indicator = if value.starts_with('\n') || value.starts_with(' ') {
        "2".to_string()
    } else {
        String::new()
    };
    let chomping = if double_clip {
        "+"
    } else if clip {
        ""
    } else {
        "-"
    };
    let body = indent_string(value, content_indent);
    // `dropEndingNewline`: remove exactly one trailing newline from the
    // indented body; the writer adds the document newline.
    let body = body.strip_suffix('\n').unwrap_or(&body);
    format!("|{indicator}{chomping}\n{body}")
}

fn folded_block(value: &str, content_indent: usize) -> String {
    let width = 80usize.saturating_sub(content_indent).max(40);
    let folded = fold_block_scalar(value, width);
    format!(">-\n{}", indent_string(&folded, content_indent))
}

fn indent_string(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fold_block_scalar(value: &str, width: usize) -> String {
    let mut lines = value.split('\n');
    let first = lines.next().unwrap_or("");
    let mut result = fold_line(first, width);
    let mut previous_more_indented =
        value.starts_with('\n') || value.starts_with(' ') || value.starts_with('\t');
    for line in lines {
        let more_indented = !line.is_empty() && (line.starts_with(' ') || line.starts_with('\t'));
        result.push('\n');
        if !previous_more_indented && !more_indented && !line.is_empty() {
            result.push('\n');
        }
        result.push_str(&fold_line(line, width));
        previous_more_indented = more_indented;
    }
    result
}

fn fold_line(line: &str, width: usize) -> String {
    if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut breaks = Vec::new();
    for index in 0..chars.len().saturating_sub(1) {
        if chars[index] == ' ' && chars[index + 1] != ' ' && chars[index + 1] != '\t' {
            breaks.push(index);
        }
    }
    let mut start = 0usize;
    let mut current = 0usize;
    let mut result = String::new();
    for next in breaks {
        if next.saturating_sub(start) > width {
            let end = if current > start { current } else { next };
            result.push('\n');
            result.extend(&chars[start..end]);
            start = end + 1;
        }
        current = next;
    }
    result.push('\n');
    if chars.len().saturating_sub(start) > width && current > start {
        result.extend(&chars[start..current]);
        result.push('\n');
        result.extend(&chars[current + 1..]);
    } else {
        result.extend(&chars[start..]);
    }
    result.strip_prefix('\n').unwrap_or(&result).to_string()
}
