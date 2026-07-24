//! Procedural exports for the SCS telemetry plugin ABI.
//!
//! The generated functions are intentionally kept out of application crates:
//! all raw pointers, calling-convention declarations, symbol attributes, and
//! panic containment live in the framework boundary. A plugin author supplies
//! only a normal Rust expression which constructs a [`TelemetryPlugin`]
//! implementation.
//!
//! [`TelemetryPlugin`]: https://docs.rs/scs-sdk-plugin/latest/scs_sdk_plugin/trait.TelemetryPlugin.html

#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, parse_macro_input};

/// Generates the two symbols loaded by ETS2 or ATS for one plugin instance.
///
/// The input is evaluated once for each successful SDK initialization attempt.
/// It may be a constructor, a struct literal, or any other expression whose
/// value implements `scs_sdk_plugin::TelemetryPlugin`.
///
/// The expansion owns a process-lifetime runtime object and emits:
///
/// - `scs_telemetry_init`, using the SCS system calling convention;
/// - `scs_telemetry_shutdown`, which drains registrations before dropping the
///   active plugin value.
///
/// ABI functions and raw SDK pointers exist only inside the expansion and
/// `scs-sdk-plugin`; the invoking application's source remains ordinary safe
/// Rust.
///
/// # Example
///
/// ```ignore
/// #[derive(Default)]
/// struct MyPlugin;
///
/// impl scs_sdk_plugin::TelemetryPlugin for MyPlugin {
///     fn initialize(
///         &mut self,
///         _context: &mut scs_sdk_plugin::PluginContext<'_>,
///     ) -> scs_sdk_plugin::PluginResult {
///         Ok(())
///     }
/// }
///
/// scs_sdk_plugin::export_plugin!(MyPlugin::default());
/// ```
#[proc_macro]
pub fn export_plugin(input: TokenStream) -> TokenStream {
    let constructor = parse_macro_input!(input as Expr);

    quote! {
        // This process-lifetime address is used as the opaque callback context
        // registered with SCS. `Runtime` internally serializes lifecycle state
        // and retains failed-unregistration contexts until they are harmless.
        static __SCS_SDK_PLUGIN_RUNTIME: ::scs_sdk_plugin::__private::Runtime =
            ::scs_sdk_plugin::__private::Runtime::new();

        #[doc = "Initializes the telemetry plugin through the safe scs-sdk-plugin runtime."]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "The game must pass the live initialization structure matching `version`"]
        #[doc = "and obey the SCS Telemetry SDK main-thread lifecycle contract."]
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn scs_telemetry_init(
            version: ::scs_sdk_plugin::__private::ScsU32,
            params: *const ::scs_sdk_plugin::__private::ScsTelemetryInitParams,
        ) -> ::scs_sdk_plugin::__private::ScsResult {
            // SAFETY: This exported function is called directly by SCS. Its
            // public safety contract forwards the exact pointer, version, and
            // main-thread requirements to the framework boundary.
            unsafe {
                __SCS_SDK_PLUGIN_RUNTIME.initialize(version, params, || {
                    ::std::boxed::Box::new(#constructor)
                })
            }
        }

        #[doc = "Stops the active telemetry plugin and releases SDK registrations."]
        #[unsafe(no_mangle)]
        pub extern "system" fn scs_telemetry_shutdown() {
            __SCS_SDK_PLUGIN_RUNTIME.shutdown();
        }
    }
    .into()
}
