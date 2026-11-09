// Harness that wires the nested adapter/ test files into a Cargo test target.

#[path = "adapter/ember/ezsp_buffalo_test.rs"]
mod ember_ezsp_buffalo_test;

#[path = "adapter/zboss/fix_zdo_response_test.rs"]
mod zboss_fix_zdo_response_test;

#[path = "adapter/zigate/patch_zdo_buffalo_be_test.rs"]
mod zigate_patch_zdo_buffalo_be_test;
