// Harness that wires the nested adapter/ test files into a Cargo test target.

#[path = "adapter/adapter_test.rs"]
mod adapter_test;

#[path = "adapter/ember/ash_test.rs"]
mod ember_ash_test;

#[path = "adapter/ember/consts.rs"]
mod ember_consts;

#[path = "adapter/ember/ember_adapter_test.rs"]
mod ember_ember_adapter_test;

#[path = "adapter/ember/ezsp_buffalo_test.rs"]
mod ember_ezsp_buffalo_test;

#[path = "adapter/ember/ezsp_error_test.rs"]
mod ember_ezsp_error_test;

#[path = "adapter/ember/ezsp_test.rs"]
mod ember_ezsp_test;

#[path = "adapter/ember/math_test.rs"]
mod ember_math_test;

#[path = "adapter/ezsp/frame_test.rs"]
mod ezsp_frame_test;

#[path = "adapter/ezsp/uart_test.rs"]
mod ezsp_uart_test;

#[path = "adapter/z_stack/adapter_test.rs"]
mod z_stack_adapter_test;

#[path = "adapter/z_stack/constants_test.rs"]
mod z_stack_constants_test;

#[path = "adapter/z_stack/structs_test.rs"]
mod z_stack_structs_test;

#[path = "adapter/z_stack/unpi_test.rs"]
mod z_stack_unpi_test;

#[path = "adapter/z_stack/znp_test.rs"]
mod z_stack_znp_test;

#[path = "adapter/zboss/fix_zdo_response_test.rs"]
mod zboss_fix_zdo_response_test;

#[path = "adapter/zboss/zboss_adapter_test.rs"]
mod zboss_zboss_adapter_test;

#[path = "adapter/zigate/patch_zdo_buffalo_be_test.rs"]
mod zigate_patch_zdo_buffalo_be_test;

#[path = "adapter/zigate/zdo_test.rs"]
mod zigate_zdo_test;

#[path = "adapter/zoh/utils_test.rs"]
mod zoh_utils_test;

#[path = "adapter/zoh/zoh_adapter_test.rs"]
mod zoh_zoh_adapter_test;
