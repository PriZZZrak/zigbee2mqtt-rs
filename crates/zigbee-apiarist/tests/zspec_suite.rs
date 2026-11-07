// Harness that wires the nested zspec/ test files into a Cargo test target.

#[path = "zspec/zcl/buffalo_test.rs"]
mod zcl_buffalo_test;

#[path = "zspec/zcl/foundation_test.rs"]
mod zcl_foundation_test;

#[path = "zspec/zcl/frame_test.rs"]
mod zcl_frame_test;

#[path = "zspec/zcl/utils_test.rs"]
mod zcl_utils_test;

#[path = "zspec/zdo/buffalo_test.rs"]
mod zdo_buffalo_test;

#[path = "zspec/zdo/utils_test.rs"]
mod zdo_utils_test;
