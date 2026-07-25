//! Procedural exports for the SCS telemetry and input plugin ABIs.
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
///     fn metadata(&self) -> scs_sdk_plugin::PluginMetadata {
///         scs_sdk_plugin::PluginMetadata::new("My Plugin", env!("CARGO_PKG_VERSION"))
///     }
///
///     fn compatibility(&self) -> scs_sdk_plugin::PluginCompatibility {
///         use scs_sdk_plugin::sdk::{TelemetryApiVersion, game};
///         use scs_sdk_plugin::{Game, GameCompatibility, PluginCompatibility};
///
///         const GAMES: &[GameCompatibility] = &[GameCompatibility::new(
///             Game::EuroTruckSimulator2,
///             game::ets2::V1_00,
///         )];
///         PluginCompatibility::new(TelemetryApiVersion::V1_00, GAMES)
///     }
///
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

/// Generates the two loader-visible symbols for one SCS input plugin.
///
/// The constructor expression is evaluated once for each accepted input
/// initialization attempt and must produce a value implementing
/// `scs_sdk_plugin::InputPlugin`. The expansion owns an independent input
/// runtime and emits exactly `scs_input_init` and `scs_input_shutdown`.
///
/// This macro may coexist with `export_plugin` in one dynamic library because
/// the input and telemetry runtimes and exported symbol names are distinct.
/// Application source remains safe; raw pointers and ABI declarations are
/// generated only at the audited framework boundary.
#[proc_macro]
pub fn export_input_plugin(input: TokenStream) -> TokenStream {
    let constructor = parse_macro_input!(input as Expr);

    quote! {
        // Input callbacks retain the opaque device context after initialization.
        // The runtime therefore needs a stable process-lifetime root, separate
        // from the telemetry runtime used by export_plugin!.
        static __SCS_SDK_INPUT_PLUGIN_RUNTIME: ::scs_sdk_plugin::__private::InputRuntime =
            ::scs_sdk_plugin::__private::InputRuntime::new();

        #[doc = "Initializes the input plugin through the safe scs-sdk-plugin runtime."]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "The game must pass the live input initialization structure matching"]
        #[doc = "version and obey the SCS Input SDK main-thread lifecycle contract."]
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn scs_input_init(
            version: ::scs_sdk_plugin::__private::ScsU32,
            params: *const ::scs_sdk_plugin::__private::ScsInputInitParams,
        ) -> ::scs_sdk_plugin::__private::ScsResult {
            // SAFETY: This is the loader-facing input ABI boundary. The caller
            // provides the matching live initialization layout and serialized
            // main-thread invocation. InputRuntime contains every Rust panic
            // and keeps registered callback contexts at stable addresses.
            unsafe {
                __SCS_SDK_INPUT_PLUGIN_RUNTIME.initialize(version, params, || {
                    // The expected Box<dyn InputPlugin> return type makes an
                    // invalid constructor fail during compilation.
                    ::std::boxed::Box::new(#constructor)
                })
            }
        }

        #[doc = "Stops the active input plugin after SCS unregisters its devices."]
        #[doc = ""]
        #[doc = "The framework invokes the product shutdown hook and releases the"]
        #[doc = "successful generation's callback contexts."]
        #[unsafe(no_mangle)]
        pub extern "system" fn scs_input_shutdown() {
            __SCS_SDK_INPUT_PLUGIN_RUNTIME.shutdown();
        }
    }
    .into()
}
