#![allow(non_snake_case)]
#![allow(clippy::module_inception)]

/// Crate version used by the z2m-rs bridge for the
/// `Starting zigbee-herdsman (<version>)` log line (`lib/zigbee.ts:33`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod adapter;
pub mod buffalo;
pub mod controller;
pub mod models;
pub mod utils;
pub mod zspec;
