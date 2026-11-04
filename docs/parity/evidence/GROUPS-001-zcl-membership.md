# GROUPS-001 — ZCL group membership commands

Rust implementation: `src/zigbee/zcl/clusters/groups.rs`,
`src/bridge.rs` (`group/members/add|remove|remove_all`, `send_group_command`),
`CoordinatorHandle::request_zcl_expected`. Behavior: GROUPS-001; plan FEAT-01.

## Source contract

Pinned app/herdsman:

- `zigbee2mqtt/lib/extension/groups.ts:198–315`: add/remove requires a group and
  a device; `remove_all` requires only the device and calls
  `endpoint.removeFromAllGroups()`, then emits membership changes for affected
  groups. The response payload echoes the involved group/device.
- Herdsman endpoint group methods send ZCL Groups cluster (0x0004) commands:
  addGroup 0x00, viewGroup 0x01, getGroupMembership 0x02, removeGroup 0x03,
  removeAllGroups 0x04. The device response is awaited before reporting success.
- `zclWaitressValidator` matches the expected command id (or a default response)
  for the same TSN/endpoint/cluster.

## Rust change

- Added Groups-cluster builders with 16-byte name truncation, membership query and
  remove-all, plus a response parser for add/view/remove responses, membership
  lists and default responses.
- Added `request_zcl_expected` (transaction + coordinator wrappers) so a
  cluster-specific response command can override the default expected response.
- Bridge add/remove/remove_all now:
  1. resolve group/device/endpoint (remove_all needs no group);
  2. send the real ZCL command via `request_zcl_expected` and parse the response;
  3. update local membership/database/config only after a success status;
  4. return an `error` response with the status/transport error instead of a
     fake `ok`.
- `remove_all` now sends RemoveAllGroups to the endpoint and removes that
  device/endpoint from all local groups, matching the upstream direction.

## Tests and evidence

- `add_group_payload_layout`, name truncation, remove/remove-all payloads,
  get-membership payload, add/default response parsing, membership response.
- Full suite: `cargo test --locked` → **336 passed** (304 library, 2 binary,
  30 compatibility); clippy exit 0 with 5 existing warnings; release build OK.

## Divergences and gaps (not verified)

- No fake-coordinator/bridge E2E for membership; only payload/response codecs are
  tested.
- Group ZCL command fanout (sending On/Off/Level/Color to a group address) and
  group state publication are not implemented.
- Bind/reporting disable flags (`skip_disable_reporting`) are ignored.
- Group scenes, member persistence into coordinator group table, and removal of
  stale groups from devices are not ported.
- No hardware or coordinator confirmation evidence; GROUPS-001 stays
  `IN_PROGRESS`.
