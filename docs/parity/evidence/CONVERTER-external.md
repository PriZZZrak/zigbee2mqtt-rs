# Evidence: Converters common libs + on-demand (CONVERTER-001..006)

## Goal
Support 4000+ devices without 5MB file, 90% windfront, pure Rust.

## Source pins
- converters 9dabbe4 v26.115.1 src/lib/{types,modernExtend,exposes,reporting,tuya}.ts + src/converters/*

## Rust mapping
- src/converters/types.rs: DefinitionWithExtend
- src/converters/modern_extend.rs: on_off, light_brightness, battery
- src/converters/exposes.rs: light_brightness, temperature
- src/zigbee/reporting.rs: bind, payload
- src/converters/tuya.rs: EF00 dp 1
- src/converters/external.rs: boa_engine, /etc/herdsman-converters, registry.json via gh-pages

## Still not as original
- modernExtend only 3 of ~50 helpers
- exposes only 4 of 100+ types
- reporting only bind/payload, not configureReporting per cluster
- tuya only dp 1, not full EF00
- external eval stub (boa_engine not yet wired to real fromZigbee)
- No generated definitions for 4000 devices (registry.json naive)
