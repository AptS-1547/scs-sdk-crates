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
/// After the framework validates the SDK version, initialization pointer, and
/// idle lifecycle state, the input is evaluated exactly once for that accepted
/// initialization attempt. It may be a constructor, a struct literal, or any
/// other expression whose value implements
/// `scs_sdk_plugin::TelemetryPlugin`. The compiler enforces that trait bound
/// when the generated factory coerces the value into the framework's plugin
/// object; an expression returning another type is a compile-time error.
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
/// Invoke this macro exactly once in a plugin `cdylib`. Both exported names are
/// fixed by the SCS loader contract, so two invocations in one link unit would
/// define the same process runtime and symbols twice.
///
/// # Compile fixtures
///
/// The example below remains ignored as a rustdoc test because this proc-macro
/// crate cannot depend back on `scs-sdk-plugin` without creating a Cargo
/// dependency cycle. The same consumer code is compiled independently in
/// `scs-sdk-plugin/tests/fixtures/export-plugin/pass`. A sibling fixture omits
/// the trait implementation and must fail with E0277. Windows and Linux fixture
/// builds also inspect the finished dynamic export tables for both SCS symbols.
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
        // SCS stores callback context pointers beyond initialization, so the
        // runtime's address must never move while the library is loaded. A
        // process-lifetime static provides that stable root; individual event
        // and channel contexts remain in Arc allocations owned by Runtime.
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
            // SAFETY: This function is the loader-facing ABI boundary. The
            // caller contract above guarantees that `params` identifies the
            // live SDK structure for `version` and that execution is the
            // serialized game-main-thread initialization call. Runtime borrows
            // foreign data only for this call and contains every Rust panic.
            unsafe {
                __SCS_SDK_PLUGIN_RUNTIME.initialize(version, params, || {
                    // The coercion to Box<dyn TelemetryPlugin> inside Runtime's
                    // factory parameter is deliberate: it makes a constructor
                    // returning the wrong type fail during compilation instead
                    // of reaching an ABI entry point at runtime.
                    ::std::boxed::Box::new(#constructor)
                })
            }
        }

        #[doc = "Stops the active telemetry plugin and releases SDK registrations."]
        #[doc = ""]
        #[doc = "SCS calls this entry point during its serialized shutdown lifecycle."]
        #[doc = "The framework invokes the product shutdown hook, unregisters callbacks"]
        #[doc = "in reverse order, and retains any context whose SDK unregistration fails."]
        #[unsafe(no_mangle)]
        pub extern "system" fn scs_telemetry_shutdown() {
            __SCS_SDK_PLUGIN_RUNTIME.shutdown();
        }
    }
    .into()
}
