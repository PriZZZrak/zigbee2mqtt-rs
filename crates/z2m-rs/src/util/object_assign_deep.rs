// Original: .upstream/zigbee2mqtt/lib/util/objectAssignDeep.ts
// TODO: port objectAssignDeep.

/// Merge all `sources` into `target` recursively.
///
/// Upstream signature is `objectAssignDeep(target, ...sources)`. Rust has no
/// variadic arguments, so sources are passed as a slice. `target` is mutated
/// and returned.
#[allow(unused)]
pub fn object_assign_deep<'a>(
    target: &'a mut serde_json::Value,
    sources: &[serde_json::Value],
) -> &'a mut serde_json::Value {
    todo!()
}
