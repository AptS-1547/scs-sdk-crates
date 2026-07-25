//! Compile-fail fixture proving input constructors are type checked.

#![forbid(unsafe_code)]

#[derive(Default)]
struct Plugin;

scs_sdk_plugin::export_input_plugin!(Plugin);
