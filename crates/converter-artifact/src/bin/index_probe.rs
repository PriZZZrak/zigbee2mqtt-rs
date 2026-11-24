use converter_artifact::{from_binary, from_json};
fn arg(name: &str) -> String {
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        if x == name {
            return a.next().expect("value");
        }
    }
    panic!("{name}")
}
fn status(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
fn main() {
    let format = arg("--format");
    let path = arg("--path");
    let data = std::fs::read(&path).expect("read");
    let start = std::time::Instant::now();
    let index = if format == "json" {
        from_json(&data).expect("json")
    } else {
        from_binary(&data).expect("binary")
    };
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        serde_json::json!({"format":format,"bytes":data.len(),"parseMs":ms,"definitions":index.definitions.len(),"keys":index.keys.len(),"rssKb":status("VmRSS:"),"peakRssKb":status("VmHWM:")})
    );
}
