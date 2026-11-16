# `onboarding.test.ts` parity — per-case mapping

Source: `.upstream/zigbee2mqtt/test/onboarding.test.ts` (zigbee2mqtt 2.14.3,
`3c841fdf`), 29 `it` cases. Rust suite:
`crates/z2m-rs/tests/onboarding.rs` (19 tests).

Upstream mocks `node:http`/`node:fs` and drives the express handlers; the port
calls the same handlers directly (`submit_settings`, `submit_zip`,
`handle_request`, `read_http_request`, `onboard`) and asserts on their results
and on the files they write. Several upstream cases share one Rust test when
they exercise the same handler branch; the table records which.

| # | upstream `it` | Rust test |
|---|---|---|
| 1 | extracts uploaded ZIP files into the data path | `extracts_uploaded_zip_files_into_the_data_path` |
| 2 | rejects non-zip upload payloads | `rejects_non_zip_upload_payloads` |
| 3 | rejects ZIP upload payloads with invalid entry paths | `rejects_zip_upload_payloads_with_invalid_entry_paths` |
| 4 | rejects ZIP upload payloads with unsafe relative entry paths | `rejects_zip_upload_payloads_with_unsafe_relative_entry_paths` |
| 5 | handles empty ZIP upload payloads | `handles_empty_zip_upload_payloads` |
| 6 | handles request stream errors for submit endpoint | `handles_request_stream_errors_for_submit_endpoints` |
| 7 | handles request stream errors for submit-zip endpoint | `handles_request_stream_errors_for_submit_endpoints` |
| 8 | passes unknown onboarding routes to static file server | `passes_unknown_onboarding_and_failure_routes_to_the_static_file_server` |
| 9 | passes unknown failure-pages routes to static file server | `passes_unknown_onboarding_and_failure_routes_to_the_static_file_server` |
| 10 | returns false when onboarding server emits an error | `returns_an_error_when_the_onboarding_server_cannot_listen` |
| 11 | handles empty config submit | `handles_empty_config_submit` |
| 12 | creates config file and sets given settings | `creates_config_file_and_sets_given_settings` |
| 13 | creates config file and sets given unusual settings | `creates_config_file_and_sets_unusual_settings` |
| 14 | reruns onboard via ENV and sets given settings | `reruns_onboard_via_env_and_on_failed_start` |
| 15 | reruns onboard on failed start | `reruns_onboard_via_env_and_on_failed_start` |
| 16 | sets given settings - no frontend redirect | `sets_given_settings_without_and_with_frontend_redirect` |
| 17 | sets given settings - no frontend redirect via ENV | `sets_given_settings_without_and_with_frontend_redirect` |
| 18 | sets given settings - frontend SSL redirect | `sets_given_settings_without_and_with_frontend_redirect` |
| 19 | handles saving errors | `submit_reports_a_write_failure_instead_of_success` (`crates/z2m-rs/tests/persistence_errors.rs`): `Settings::write` returns `Result`, `/submit` answers 406 `{success:false, error}` like upstream |
| 20 | handles configuring onboarding via ENV | `handles_configuring_and_disabling_onboarding_via_env` |
| 21 | handles disabling onboarding server via ENV | `handles_configuring_and_disabling_onboarding_via_env` |
| 22 | handles configuring onboarding with config ENV overrides | `handles_configuring_and_disabling_onboarding_via_env` |
| 23 | runs migrations | `runs_migrations` |
| 24 | runs 1.x.x conflict migrations | `runs_migrations` (the shared `settings_migration` behavior; the 1.x conflict vectors are covered by `crates/z2m-rs/tests/settings_migration.rs`, 15 tests ported from the upstream migration cases) |
| 25 | handles validation failure | `handles_validation_failure` |
| 26 | handles non-required validation failure before applying envs | `handles_non_required_validation_failure_before_applying_envs` |
| 27 | handles invalid yaml file | `handles_invalid_yaml_file_and_load_errors` |
| 28 | handles error while loading yaml file | `handles_invalid_yaml_file_and_load_errors` |
| 29 | handles creating data path | `handles_creating_data_path` |

## Deviations fixed while porting

- **A missing `configuration.yaml` now starts from the upstream minimal
  defaults** (`writeMinimalDefaults`, `settings.ts:157-183`) instead of `{}`,
  so a fresh data directory passes required-schema validation and onboarding
  can complete.
- **Settings migrations run on read** (`migrateIfNecessary`,
  `settings.ts:186`): a `version: 4` file is upgraded and rewritten before use.
- **Truncated request bodies are rejected**: `read_http_request` returns
  `UnexpectedEof` when the stream ends before `Content-Length`, matching the
  express request-stream error upstream asserts.

## Case 19 (saving errors) — fixed

`Settings::write` now returns `Result<bool, String>` and the mutators
(`apply`, `changeFriendlyName`, `changeEntityOptions`, `addDevice`,
`removeDevice`, `blockDevice`, `addGroup`, `removeGroup`, `setOnboarding`)
propagate the I/O error like the upstream throw. `POST /submit` therefore
answers `406 {success:false, error}` (upstream's status) and the bridge
handlers turn the same error into `{status:"error"}`
