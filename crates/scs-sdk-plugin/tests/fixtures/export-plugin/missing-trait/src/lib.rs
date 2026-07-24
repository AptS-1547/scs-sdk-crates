//! Compile-fail fixture proving that exported constructors are type checked.
//!
//! The macro accepts a general Rust expression syntactically, but its generated
//! runtime factory requires the expression's value to implement
//! `TelemetryPlugin`. This fixture intentionally omits that implementation so
//! the compiler, rather than a runtime branch, rejects the plugin.

#![forbid(unsafe_code)]

/// Looks like a constructible plugin value but deliberately lacks the required
/// `scs_sdk_plugin::TelemetryPlugin` implementation.
#[derive(Default)]
struct Plugin;

// Expected diagnostic: E0277, with a trait-bound failure mentioning
// `TelemetryPlugin`. The fixture runner verifies both facts so an unrelated
// syntax or dependency error does not masquerade as the intended failure.
scs_sdk_plugin::export_plugin!(Plugin::default());
