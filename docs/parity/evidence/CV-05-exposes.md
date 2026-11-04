# CV-05 — prepared exposes API and persistent-context blocker

Rust implementation: `src/converters/external.rs` (`exposes`, `cjs_bootstrap`,
`__prepareDefinition`). Behavior: CONVERTER-017; plan CV-05.

## What was delivered

- `ExternalConverter::exposes(model, device)` evaluates the prepared definition's
  `exposes`:
  - array exposes are returned as JSON;
  - function exposes are called with the device object (including
    `getEndpoint`);
  - function values inside expose objects are dropped during serialization;
  - modern-extend exposes are merged when the definition exposes array.
- `__prepareDefinition` now preserves a function-valued `exposes` instead of
  spreading it.
- Fixture tests:
  - `exposes_array_is_returned_as_json` (name/unit);
  - `exposes_function_receives_the_device` (model-dependent expose name).

Full suite: `cargo test --locked` → **328 passed** (296 library, 2 binary,
30 compatibility). Clippy exits 0 with five existing warnings; native release
build succeeds.

## Blocker: persistent per-device Boa context

The next requested step (persistent context + shared store + timers) requires an
architecture decision; the current code cannot safely store a Boa context:

- Boa 0.19 `Context` contains `job_queue: Rc<dyn JobQueue>` and
  `module_loader: Rc<dyn ModuleLoader>` (plus Rc-backed values), so `Context` is
  **not `Send`**.
- `ExternalConverter` is shared as `Arc<ExternalConverter>` by the bridge and the
  binary uses the default multi-thread Tokio runtime (`src/main.rs`,
  `#[tokio::main]`). Storing a `Context` in the shared struct is unsound; a
  thread-local map can lose state when the task migrates threads.
- Timers additionally need a JS callback store and a scheduler; shared module
  store semantics need a stable device object per context.

Options for approval:

1. **Dedicated converter actor thread (recommended):** a `std::thread` owns all
   Boa contexts and module caches; `ExternalConverter` sends blocking jobs over a
   channel. Preserves the single bridge business owner, makes `Context` ownership
   sound, and allows per-device contexts/timers. Requires measuring per-device
   memory on the router and adding a shutdown protocol.
2. **Single-thread Tokio + thread-local contexts:** change the runtime to
   `current_thread` (already conceptually one bridge loop) and keep contexts in a
   thread-local keyed by device. Simpler, but changes scheduler topology and
   makes future parallel work unsafe.
3. **Keep per-call contexts (status quo):** no multi-frame module state and no
   timers; only `meta.state` persists. This cannot satisfy CV-06/07.

No option was implemented blindly: AGENTS requires approval for a concurrency
topology change, and the memory budget per device is unknown.

## Remaining gaps

- Exposes are wired into Home Assistant discovery for numeric/binary/switch/light
  (`HA-003`); complex entity kinds and cleanup still use fallback/are missing.
- Prepared exposes omit complete meta/configure/onEvent semantics and
  manufacturer-specific overrides.
- No real package or Node oracle for exposes.
