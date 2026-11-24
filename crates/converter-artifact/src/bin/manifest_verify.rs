use converter_artifact::Manifest;
fn main() {
    let root_arg = std::env::args().nth(1).expect("artifact root");
    let root = std::path::Path::new(&root_arg);
    let manifest = Manifest::load(&root.join("manifest.json")).expect("manifest");
    manifest
        .verify_compatibility(1, "quickjs-oxide", "0.5.0")
        .expect("compatibility");
    manifest.verify_package(root).expect("package hashes");
    let index = converter_artifact::from_binary(
        &std::fs::read(root.join(&manifest.index.file)).expect("index"),
    )
    .expect("parse index");
    assert_eq!(index.definitions.len() as u64, manifest.index.definitions);
    assert_eq!(index.keys.len() as u64, manifest.index.lookup_keys);
    println!(
        "{}",
        serde_json::json!({"artifact":manifest.artifact_id,"indexSha256":manifest.index.sha256,"sharedTreeSha256":manifest.shared.sha256,"definitions":index.definitions.len(),"keys":index.keys.len(),"modules":manifest.modules.entries.len()})
    );
}
