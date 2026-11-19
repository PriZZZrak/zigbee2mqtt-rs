// Test/oracle helper: dump JSON from stdin as YAML with the same writer used by
// the settings file (`serde_yaml_ng`).
use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let value: serde_json::Value = serde_json::from_str(&input).unwrap();
    print!("{}", z2m_rs::util::yaml_dump::dump(&value));
}
