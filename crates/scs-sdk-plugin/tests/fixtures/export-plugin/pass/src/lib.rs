//! Independent compile-pass fixture for [`scs_sdk_plugin::export_plugin!`].
//!
//! This crate deliberately resembles the smallest real application plugin. It
//! depends only on `scs-sdk-plugin`, implements the safe lifecycle trait, and
//! invokes the public re-export of the proc macro. Platform build scripts then
//! inspect this crate's finished `cdylib` rather than assuming successful macro
//! expansion also implies correct loader-visible exports.

#![forbid(unsafe_code)]
#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![deny(
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::unwrap_used
)]

use scs_sdk_plugin::{PluginContext, PluginResult, TelemetryPlugin};

/// Minimal product-owned state used to prove the constructor expression is
/// accepted without any raw SDK types or framework-private imports.
#[derive(Default)]
struct Plugin {
    initialized: bool,
}

impl TelemetryPlugin for Plugin {
    /// Performs an explicit state transition while requesting no subscriptions.
    ///
    /// A zero-subscription plugin is useful here: the fixture tests macro and
    /// lifecycle wiring only, without coupling export verification to one
    /// particular channel or event catalog entry.
    fn initialize(&mut self, _context: &mut PluginContext<'_>) -> PluginResult {
        self.initialized = true;
        Ok(())
    }

    /// Restores product state during framework-controlled shutdown.
    fn shutdown(&mut self, _context: &mut PluginContext<'_>) {
        self.initialized = false;
    }
}

// This is the exact consumer-facing form promised by the framework. The
// expansion contains the audited ABI boundary, while this source file remains
// valid under `forbid(unsafe_code)` and imports no `__private` implementation
// details.
scs_sdk_plugin::export_plugin!(Plugin::default());
