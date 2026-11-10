# NV recovery attempt (2026-11-10)

After the forbidden re-commissioning the live devices stopped answering. Two
non-commissioning recovery attempts were made on the same coordinator.

## 1. Byte-exact NV snapshot

`examples/hil_apply_snapshot.rs` applies the snapshot captured while the
network was working (`ref/hil-2026-11-09/adapter-dump.json`): all 16 NV items
and the APS key-data / network-security-material / TCLK tables, then a soft
reset. It never deletes the NIB and never forms a network.

```bash
Z2M_SNAPSHOT_APPLY=1 Z2M_SERIAL_PORT=/dev/ttyUSB0 \
Z2M_SNAPSHOT=ref/hil-2026-11-09/adapter-dump.json \
  cargo run -p zigbee-apiarist --example hil_apply_snapshot
```

After the write the NV is byte-identical to the working dump:

```text
nv diffs: none
```

Result: still **0 associated devices**, no `nwkAddrRsp`.

## 2. Provisioning network keys

The two random provisioning keys from the destructive runs were tried as the
coordinator key (PAN/channel unchanged):

- `d1e9a1d2b9a768165095ccc5bc187001` (restore run)
- `f7713c55f63b584d137252733dc8dbd5` (commissioning run)

Neither produced a response. The original key was restored afterwards.

## Device reachability

```text
0x404ccafffe5c4ce0 nwk=18008: mac=Ok(None) nwkAddrReq=timeout
0x1020bafffef50ee8 nwk=41997: mac=Ok(None) nwkAddrReq=timeout
0xa4c13887bf43cc0f nwk=15398: mac=Err(9999) nwkAddrReq=SRSP timeout
```

Both mains routers confirm frames at MAC level but do not decrypt/answer.
Neither known provisioning key matches, so the devices hold an unknown network
key (from the first partial provisioning run) or are powered down.

## Conclusion

The coordinator NV is back to the exact working snapshot. Device recovery now
requires physical action: power-cycle / pairing mode, then permit join.
