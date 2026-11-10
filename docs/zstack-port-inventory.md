# `adapter/z-stack` port inventory

Upstream: `zigbee-herdsman` v11.2.0 (`.upstream/zigbee-herdsman/src/adapter/z-stack`).
Rust: `crates/zigbee-apiarist/src/adapter/z_stack` (+ `models/startup_options.rs`).

All 52 upstream `.ts` files have a Rust counterpart. Barrel files
(`constants/index.ts`, `structs/index.ts`, `unpi/index.ts`, `utils/index.ts`,
`znp/index.ts`) map to the Rust `mod.rs` files; `znp/tstype.ts` is type-only and
is represented by the Rust structs/enums in `znp/definition.rs`,
`znp/zpi_object.rs` and `znp/buffalo_znp.rs`.

| Upstream group | Files | Rust | Status |
|---|---|---|---|
| `adapter/adapter-backup.ts` | 1 | `adapter/adapter_backup.rs` | PORTED — `createBackup` HIL, restore validated once (re-run forbidden) |
| `adapter/adapter-nv-memory.ts` | 1 | `adapter/adapter_nv_memory.rs` | PORTED — wire transcript + HIL `init` |
| `adapter/endpoints.ts` | 1 | `adapter/endpoints.rs` | PORTED — exact table test (incl. Shelly profile `0xc001`) |
| `adapter/manager.ts` | 1 | `adapter/manager.rs` | PORTED — startup HIL; PTY strategy tests for all three strategies |
| `adapter/tstype.ts` | 1 | `adapter/tstype.rs` | PORTED |
| `adapter/zStackAdapter.ts` | 1 | `adapter/z_stack_adapter.rs` | PORTED — HIL start/ZDO/ZCL/interPAN/params/backup |
| `constants/*` | 10 | `constants/*` | PORTED — generated + differential fixtures |
| `models/startup-options.ts` | 1 | `models/startup_options.rs` | PORTED |
| `structs/*` | 18 | `structs/*` | PORTED — `differential_structs` fixtures |
| `unpi/*` | 3 | `unpi/*` | PORTED — `differential_frame` |
| `utils/*` | 3 | `utils/*` | PORTED |
| `znp/buffaloZnp.ts` | 1 | `znp/buffalo_znp.rs` | PORTED — `differential_buffalo_znp` |
| `znp/definition.ts` | 1 | `znp/definition.rs` + `definition_data.rs` | PORTED — 241 commands, `differential_definition` |
| `znp/parameterType.ts` | 1 | `znp/parameter_type.rs` | PORTED |
| `znp/utils.ts` | 1 | `znp/utils.rs` | PORTED |
| `znp/znp.ts` | 1 | `znp/znp.rs` | PORTED — PTY transcript (`differential_znp`) |
| `znp/zpiObject.ts` | 1 | `znp/zpi_object.rs` | PORTED — `differential_zpi_object` |

Known gaps outside this folder's code: the CC2652 DTR/RTS bootloader path has
no serial-hardware test (PTY covers ping + CC2530/CC2531 magic byte), and the
destructive restore/commissioning paths are forbidden to re-run on a live
network (`AGENTS.md` §3.11).
