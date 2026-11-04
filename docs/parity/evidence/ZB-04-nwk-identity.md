# ZB-04 — unknown NWK identification and address reindexing

Rust implementation: `src/bridge.rs` (`UnknownDeviceTracker`, message/announce
handling), `src/devices/mod.rs` (`update_nwk_addr`). Behavior: ZIGBEE-004; plan
ZB-04.

## Source contract

Pinned herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- `src/controller/controller.ts:75–76,635–669` (`identifyUnknownDevice`): a set of
  unknown network addresses de-duplicates identification. The address stays in
  the set after failure for the rest of the process (anti-spam), and is removed
  after a successful IEEE-address response.
- `controller.ts:671–678` (`checkDeviceNetworkAddress`) updates the device's
  network address and persists it when it changes; a successful resolution also
  removes the unknown key.
- `controller.ts:681+` handles network-address/announce responses and updates
  existing devices.

Original controller tests and mocks should be re-read/executed before this row
can be `VERIFIED`; this change is source-trace based.

## Rust change

- Added `UnknownDeviceTracker` with the same lifecycle: first observation asks
  the coordinator for the IEEE address, repeated frames for the same NWK are
  suppressed, `identified` clears the entry on a resolved address or a device
  announce.
- The bridge no longer sends an IEEE-address request for every frame from an
  unresolved NWK.
- Existing `DeviceRegistry::update_nwk_addr` continues to remove the stale
  secondary index and update the IEEE map when a known device changes address;
  its regression test remains in `src/devices/mod.rs`.

## Tests added

- `src/bridge.rs`: `unknown_device_tracker_dedups_until_identified` — first
  request allowed, duplicate suppressed, independent NWK tracked separately,
  successful identification allows a later retry.
- Existing `devices::tests::update_nwk_addr` covers old-NWK removal.

## Remaining gaps (not verified)

- No coordinator integration test: the actual `request_ieee_addr` round trip and
  response matching are not exercised here.
- If the request fails immediately at the transport layer, the tracker keeps the
  address suppressed for the process lifetime (upstream parity) but the Rust
  code does not expose a manual reset.
- NWK-change persistence to `database.db`, sleepy-device announce timing and
  duplicate address resolution under load remain untested.
- No HIL evidence; ZIGBEE-004 stays `IN_PROGRESS`.
