# `scs-sdk-plugin`

[中文](README.zh.md) | **English**

`scs-sdk-plugin` is the safe application framework and audited runtime boundary
for native SCS telemetry plugins. It combines the typed
[`scs-sdk`](../scs-sdk/) layer with lifecycle management, transactional
registration, callback dispatch, panic containment, and stable foreign context
ownership so ordinary plugin source can remain safe Rust.

The crate deliberately re-exports both application-facing dependencies:

```rust
pub use scs_sdk as sdk;
pub use scs_sdk_plugin_macros::export_plugin;
```

A product plugin normally needs only `scs-sdk-plugin` in its manifest.

> The framework covers the public **SCS Telemetry SDK 1.14** interface. The
> SDK's input-device API is not implemented by this workspace yet.

This is an independent community crate and is not affiliated with or endorsed
by SCS Software.

## Application contract

Applications implement `TelemetryPlugin` with four explicit concerns:

1. `metadata` returns stable `PluginMetadata` containing the product name and
   build version used in lifecycle logs;
2. `compatibility` returns `PluginCompatibility`, including the minimum
   Telemetry API and a per-game `GameCompatibility` schema requirement;
3. `initialize` initializes product state and explicitly declares every event
   and channel subscription; and
4. `channel`, `event`, and `shutdown` handle delivered data and cleanup.

The runtime owns ABI entry points and foreign callback plumbing. Application
source does not need raw pointers, C strings, handwritten `extern` functions,
raw SCS unions, direct `scs-sdk-sys` access, or `unsafe` blocks.

## Minimal safe plugin

```rust
#![forbid(unsafe_code)]

use scs_sdk_plugin::sdk::{TelemetryApiVersion, channels, game};
use scs_sdk_plugin::{
    Game, GameCompatibility, PluginCompatibility, PluginContext,
    PluginMetadata, PluginResult, TelemetryPlugin,
};

static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];

#[derive(Default)]
struct Plugin;

impl TelemetryPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("My telemetry plugin", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        context.subscribe(channels::truck::SPEED)
    }
}

scs_sdk_plugin::export_plugin!(Plugin::default());
```

The application passes an ordinary constructor expression. The proc macro
generates exactly the two symbols required by the SCS loader and delegates both
to this crate's runtime.

## Explicit compatibility and game identity

`PluginCompatibility` keeps wrapper capability separate from product policy.
The runtime validates the declared minimum Telemetry API and each supported
game's minimum schema before product initialization. A later schema minor is
accepted only inside the declared major compatibility family; lower minima,
different majors, duplicated declarations, undeclared games, and ambiguous
`Game::Other` requirements are rejected deliberately.

`GameInfo` copies SCS's initialization-only strings into owned Rust values and
classifies the identifier as:

```text
Game::EuroTruckSimulator2
Game::AmericanTruckSimulator
Game::Other
```

Unknown IDs retain their original textual ID and display name. Application
code can query descriptor history through `GameInfo::minimum_schema_for` and
`GameInfo::supports` without rebuilding a second ETS2/ATS version policy.

Telemetry API version, game telemetry schema, and public game version remain
separate domains throughout initialization and logging.

## Explicit subscriptions

`PluginContext` never infers subscriptions from implemented hooks and never
subscribes an entire catalog automatically. The product explicitly requests
each capability during `initialize`.

The API keeps registration domains visible:

- scalar channels: `subscribe` and `subscribe_with_flags`;
- SDK-indexed channels: `subscribe_at` and `subscribe_at_with_flags` using
  `SdkIndex`;
- numbered trailer channels: `subscribe_trailer` using `TrailerIndex`; and
- numbered trailer plus SDK-indexed channels: `subscribe_trailer_at` using both
  strong index types.

Every family also has an explicitly named optional form. Required and optional
registrations have narrow, different failure rules:

- optional channels tolerate only `NotFound` and `UnsupportedType`;
- optional events tolerate only `Unsupported` and `NotFound`; and
- every other failure aborts initialization and preserves transactional
  rollback.

The framework checks Telemetry API and per-game schema availability before it
calls the SDK. A newer optional capability is skipped; a newer required
capability rejects initialization. Counts reported after commit include only
registrations SCS actually accepted.

## Safe callback surface

Callbacks receive framework-owned safe views:

| Type | Purpose |
| --- | --- |
| `ChannelUpdate<'a>` | Registered channel identity, flags, strong indices, and typed value decoding. |
| `TelemetryEvent<'a>` | Frame, pause/start, configuration, and gameplay callback payloads. |
| `ConfigurationEvent<'a>` | Typed configuration attributes and legacy/numbered trailer identity. |
| `GameplayEvent<'a>` | Typed gameplay attributes and known-value helpers. |

Borrowed payloads cannot outlive the hook invocation. Generic `string` and
`string_owned` access remains available beside helpers such as `shifter_type`,
`job_market`, and `fine_offence`, so a future unknown SDK string is not silently
collapsed into a known value.

`TelemetryEventKind` is a re-export of `scs_sdk::Event`, not a duplicate event
catalog. Registration identity and callback payloads therefore have distinct
types without maintaining two raw discriminator maps.

## Runtime guarantees

The audited runtime provides the shared lifecycle mechanics which product
plugins must not reimplement:

- deterministic idle, initializing, active, and shutdown state transitions;
- registration as a transaction, with partial failure rolled back in reverse
  completion order;
- normal shutdown unregistration in reverse order;
- plugin shutdown on both active teardown and rejected partial initialization;
- panic containment before every foreign ABI boundary;
- deliberate mutex-poison recovery rather than `unwrap` or `expect`;
- stable `Arc<Registration>` allocations for foreign-visible callback
  contexts;
- generation checks which isolate delayed callbacks from previous sessions;
- retained runtime backlinks and callback contexts when SDK unregistration
  fails; and
- callback-scope SDK calls kept on SCS's serialized game main thread.

Logical inactivity alone is not treated as proof that SCS discarded a pointer.
That distinction is essential for context provenance and stale-callback safety.

Runtime logs identify the plugin name and version, framework version, detected
game display name and ID, negotiated Telemetry API, game schema, committed
event/channel counts, initialization failures, and clean shutdown. Product
probe lines and business diagnostics remain application responsibilities.

## Scope boundary

This crate is reusable infrastructure. It does not implement web transports,
dispatch behavior, shared-memory protocols, save-game parsing, persistence, or
other product features. A product repository should build those concerns on
top of the safe hooks rather than moving them into the framework.

See the real application-boundary fixture at
[`examples/telemetry-plugin`](../../examples/telemetry-plugin/) and the isolated
consumer macro fixture under
[`tests/fixtures/export-plugin`](tests/fixtures/export-plugin/).

## Validation

Run from the repository root:

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --locked -p scs-sdk-plugin
cargo clippy --locked -p scs-sdk-plugin --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-plugin --no-deps
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
```

Changes to exports, macro integration, package names, or release linkage also
require the repository's Windows, Linux, and macOS release artifact and symbol
verification scripts.

## License

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

SCS-facing lifecycle contracts, ABI identifiers, compatibility metadata, and
related documentation derived from SDK 1.0 through 1.14 retain both original
SCS Software notices: [LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013) for SDK
1.0-1.5 and [LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016) for SDK 1.6-1.14.
The
[official SDK archive](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
remains third-party material and is not relicensed under the workspace license.
