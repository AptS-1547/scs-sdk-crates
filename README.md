# SCS SDK Rust Crates

**English** | [简体中文](README.zh.md)

Typed Rust bindings and a safe plugin framework for the SCS Software SDK. The current workspace covers the complete public Telemetry SDK 1.14 surface and includes a real ETS2 plugin example for validating the application boundary and cross-platform loader artifacts. Product-specific telemetry consumers belong in separate repositories.

The current foundation is implemented entirely in Rust and requires no C++ shim, CMake, or bindgen. The original official SDK distribution remains in `third-party/scs_sdk_1_14/` as the authoritative source for the ABI and constants.

## Foundation goals

The current design maintains the following boundaries:

1. **Complete SDK 1.14 coverage**: every ABI definition, channel, configuration item, gameplay event, game identifier, and version constant in the public telemetry headers enters the Rust layers.
2. **Product intent must be explicit**: the framework does not infer event subscriptions from implemented callbacks, nor does it automatically subscribe to the entire channel catalog.
3. **Application plugins use safe Rust only**: raw pointers, C strings, FFI callbacks, exported symbols, and `unsafe` are contained within the framework and lower layers.
4. **The framework owns correctness transactions**: reverse rollback after successful registrations, shutdown unregistration, panic containment, stale callback isolation, and context retention are lifecycle mechanisms that each product plugin should not have to reimplement.
5. **Cross-platform artifacts are verifiable**: the Windows DLL, Linux shared object, and macOS dynamic library are checked for their architecture and required SCS exports after building, rather than being trusted by filename extension alone.

## SDK 1.14 coverage

| Catalog | Count | High-level representation |
| --- | ---: | --- |
| Common channels | 4 | `channels::common::*` and `channels::common::ALL` |
| Truck channels | 84 | `channels::truck::*` and `channels::truck::ALL` |
| Trailer channels | 18 | `channels::trailer::*` and `channels::trailer::ALL` |
| Job channels | 1 | `channels::job::*` and `channels::job::ALL` |
| **Total channels** | **107** | `channels::ALL` |
| Configuration IDs | 6 | `configuration::ids::*` and `ids::ALL` |
| Configuration attributes | 60 | `configuration::attributes::*` and `attributes::ALL` |
| Configuration associations | 71 | `configuration::associations::*` and `associations::ALL` |
| H-shifter values | 4 | `ShifterType::ALL` |
| Job market values | 5 | `JobMarket::ALL` |
| Gameplay events | 6 | `gameplay::events::*` and `events::ALL` |
| Gameplay attributes | 15 | `gameplay::attributes::*` and `attributes::ALL` |
| Gameplay associations | 21 | `gameplay::associations::*` and `associations::ALL` |
| Fine offence values | 14 | `FineOffence::ALL` |

The raw ABI additionally covers:

- the Telemetry API version, initialization parameters, and function table;
- event and channel callback ABIs;
- SDK result codes, delivery flags, and `SCS_U32_NIL`;
- every public tagged-union value type;
- `fvector`, `dvector`, Euler angles, and single- and double-precision placements;
- frame-start, configuration, and gameplay callback payloads;
- ETS2 telemetry game versions 1.00 through 1.18;
- ATS telemetry game versions 1.00 through 1.05;
- ETS2 and ATS game IDs and game-version component functions.

The `ALL` catalogs are not another set of manually copied strings. `scs-sdk-sys` exposes raw name arrays in header order, while `scs-sdk` exposes type-erased catalogs that retain value types and indexed/scalar metadata. Tests compare every name, count, group order, and duplicate entry. Concrete decoding still uses `Channel<T>` and `Attribute<T>`, so catalog enumeration does not discard type information.

## Four-layer architecture

```text
examples/telemetry-plugin
        |
        | safe TelemetryPlugin API
        v
scs-sdk-plugin          lifecycle/runtime/framework
        |
        | typed SDK operations
        v
scs-sdk                 safe typed wrapper
        |
        | raw ABI
        v
scs-sdk-sys             no_std x86-64 ABI definitions

scs-sdk-plugin-macros   generates the two exported SCS entry points
```

Conceptually, the layers are ordered as follows:

```text
scs-sdk-sys <- scs-sdk <- scs-sdk-plugin <- scs-sdk-plugin-macros / application
```

In the actual Cargo dependency graph, `scs-sdk-plugin` re-exports `scs-sdk-plugin-macros`. The proc-macro expansion refers to the caller's existing `scs_sdk_plugin` dependency, avoiding a dependency cycle between the two crates.

### `scs-sdk-sys`

`crates/scs-sdk-sys/` is the handwritten x86-64 C ABI layer:

- it is `no_std`;
- it has no third-party Rust dependencies;
- it does not run bindgen or require Clang on the build machine;
- it defines function pointers, structures, unions, constants, and raw catalogs against the official SDK 1.14 headers;
- it represents fields used only for ABI alignment, whose values are not guaranteed to be initialized, as `MaybeUninit<u32>`;
- it uses compile-time assertions for critical structure sizes, alignments, and field offsets;
- it declares support only for the 64-bit ABI used by SCS games and makes no guarantees for 32-bit targets.

Raw pointers and foreign ABIs are allowed in this layer because its responsibility is to describe the C interface accurately. It does not provide application-level safety abstractions.

### `scs-sdk`

`crates/scs-sdk/` is the `no_std` typed wrapper:

- `TelemetryApi`, `TelemetrySession`, and the non-escaping `SdkCall` scope;
- `ScopedLogger` and the closed `LogLevel` enum;
- complete SDK result-code mapping;
- distinct `TelemetryApiVersion` and `GameSchemaVersion` types, plus typed
  `game::ets2::*` and `game::ats::*` schema-history constants projected from
  the raw headers;
- `Channel<T>`, `AnyChannel`, and `ChannelFlags`;
- `Attribute<T>`, `AnyAttribute`, `ConfigurationId`, and `GameplayEventId`;
- separate `SdkIndex`, `TrailerIndex`, and `TrailerConfigurationId` domains, so
  an indexed SDK value, a numbered trailer namespace, and the legacy
  unnumbered `trailer` configuration cannot be silently interchanged;
- `ConfigurationAttributeAssociation` and `GameplayAttributeAssociation`,
  with complete catalogs preserving which group or event carries each shared
  attribute;
- `GameSchemaAvailability` metadata retained by every built-in channel,
  configuration, gameplay descriptor, and descriptor association, with
  separate ETS2 and ATS minima derived from the official SDK 1.0 through 1.14
  header history;
- closed `ShifterType`, `JobMarket`, and `FineOffence` string-value catalogs
  with `ALL`, `COUNT`, `as_str`, `FromStr`, and value-level schema availability;
- `UnknownStringValue` for allocation-free parsing failure while the generic
  string APIs retain the original future value for diagnostics;
- `ValueRef`, which validates a tagged union's tag before reading the corresponding active member;
- `ValueType` capability metadata, including the official Telemetry API 1.01
  minimum for signed 64-bit values;
- Rust-owned geometry values: `FVector`, `DVector`, `Euler`, `FPlacement`, and `DPlacement`;
- sentinel-array iteration and typed attribute lookup through `NamedValues`;
- enumerable catalogs for 107 typed channels, 60 configuration attributes, 71
  configuration associations, 15 gameplay attributes, and 21 gameplay
  associations.

The high-level `DPlacement` value carries no ABI padding. It can be copied and retained, while the wrapper never reads uninitialized SDK alignment bytes during decoding.

The SDK requires calls back into the game to occur on the main thread and only while the game is directly invoking plugin initialization, an event callback, or shutdown. `SdkCall` uses a higher-ranked lifetime to prevent safe code from returning it, storing it globally, or sending it to another thread. The raw callback and context registration functions remain inside this layer's audited `unsafe` boundary for use by the runtime above it.

### `scs-sdk-plugin`

`crates/scs-sdk-plugin/` combines the lower-level capabilities into a safe application framework:

- the `TelemetryPlugin` lifecycle;
- explicit product identity through required `PluginMetadata`;
- explicit product requirements through required `PluginCompatibility`;
- explicit event and channel subscriptions through `PluginContext`;
- owned `GameInfo`, typed game detection through `Game::{EuroTruckSimulator2, AmericanTruckSimulator, Other}`, and canonical `minimum_schema_for` / `supports` queries for descriptor, association, and value capabilities;
- descriptor, SDK index, trailer index, and typed value decoding through `ChannelUpdate`;
- `TelemetryEvent`, `ConfigurationEvent`, and `GameplayEvent`, including typed
  trailer configuration identity plus high-level `shifter_type`, `job_market`,
  and `fine_offence` value decoders;
- game information, configuration strings, and gameplay strings as Rust `str`/`String` values;
- an initialization, reinitialization, and shutdown state machine;
- reverse transactional rollback after registration failures;
- game-schema preflight for required and optional descriptors, including the
  separately versioned numbered multi-trailer namespace;
- panic containment in callbacks and shutdown;
- mutex poison recovery;
- generation-based isolation of callbacks from old sessions;
- retention of foreign contexts when unregistration fails.

Registration contexts use `Arc<Registration>` to hold stable pointees and `AtomicBool` to represent whether the SDK side remains registered. The active and retired sets move only `Arc` handles, never the allocation, and do not invalidate foreign-pointer provenance by recreating an exclusive borrow. Each session also has a distinct generation, so even a delayed callback from an old session cannot enter a new plugin instance. This model has been verified under Miri strict provenance.

The runtime emits product and compatibility identity before product initialization, then reports the committed subscription counts. `game_display_name` is the complete display string supplied by SCS, while the API and schema versions remain separate typed fields:

```text
[scs-sdk-plugin] starting plugin name="SCS SDK Telemetry Example" version="0.1.0" framework_version="0.1.0"
[scs-sdk-plugin] detected game_display_name="Euro Truck Simulator 2 1.60.1.7s" game_id="eut2" telemetry_api=1.1 telemetry_schema=1.19
[scs-sdk-plugin] initialized plugin name="SCS SDK Telemetry Example" version="0.1.0" events=6 channels=8
```

API support has one owner: `scs-sdk::TelemetryApi` lists the versions whose
foreign initialization layouts have audited adapters. The framework consumes
that result instead of repeating a second version whitelist. A product's
`PluginCompatibility` is deliberately separate: it declares the oldest API
capabilities the product needs and a minimum schema for each supported game.
The runtime accepts later schema minors within the declared major, rejects a
different major for review, and validates everything before product
initialization or SDK registration.

The downloadable archive suffix, negotiated Telemetry API, and per-game
telemetry schema are three different version domains. The official SDK 1.0
through 1.14 archives establish this mapping:

| SDK archive | Telemetry API `CURRENT` | ETS2 schema `CURRENT` | ATS schema `CURRENT` |
| --- | --- | --- | --- |
| 1.0 | 1.00 | 1.05 | - |
| 1.1 | 1.00 | 1.07 | - |
| 1.2 | 1.00 | 1.08 | - |
| 1.3 | 1.00 | 1.09 | - |
| 1.4 | 1.00 | 1.10 | - |
| 1.5 | 1.00 | 1.12 | - |
| 1.6-1.8 | 1.00 | 1.12 | 1.00 |
| 1.9 | 1.00 | 1.13 | 1.00 |
| 1.10 | 1.01 | 1.14 | 1.01 |
| 1.11 | 1.01 | 1.15 | 1.02 |
| 1.12 | 1.01 | 1.16 | 1.03 |
| 1.13 | 1.01 | 1.17 | 1.04 |
| 1.14 | 1.01 | 1.18 | 1.05 |

In particular, the official SDK 1.10 through 1.14
`scssdk_telemetry.h` files are byte-identical, while their game-specific
headers continue to add descriptors. The wrapper records those additions on
`Channel`, `Attribute`, `ConfigurationId`, `GameplayEventId`, and `Event` as
per-game schema availability. It also records the 71 configuration and 21
gameplay attribute relationships separately, because an attribute can join a
second group later than its descriptor first appeared. `FineOffence` retains
the same distinction at value level. The numbered trailer namespace and
gameplay payloads share canonical capability constants under
`game::capabilities` rather than repeating schema numbers across catalogs.
Required registrations fail locally when the loading schema is too old;
optional registrations are skipped locally before the SDK sees the unavailable
name. SCS still decides channel-specific value conversions and may report
runtime absence. Plugins therefore do not ask users to select an SDK archive.

### `scs-sdk-plugin-macros`

`crates/scs-sdk-plugin-macros/` provides:

```rust
scs_sdk_plugin::export_plugin!(TelemetryExample::default());
```

The macro generates the two symbols discovered by the SCS loader:

```text
scs_telemetry_init
scs_telemetry_shutdown
```

It also generates process-lifetime stable runtime storage, ABI parameter conversion, and an unwind boundary. The application crate does not handwrite `extern "system"`, `no_mangle`, raw pointers, or a global runtime.

The macro is not supported merely by an `ignore`d rustdoc example. `crates/scs-sdk-plugin/tests/fixtures/export-plugin/` is a consumer workspace isolated from the main workspace. It has only a public `scs-sdk-plugin` path dependency and contains two packages:

- `pass` implements `TelemetryPlugin` and builds a real `cdylib` from `export_plugin!(Plugin::default())`;
- `missing-trait` retains the same constructor expression but omits the trait implementation, and must fail with E0277 and a `TelemetryPlugin` trait-bound diagnostic.

The passing fixture uses `#![forbid(unsafe_code)]` and undergoes the same source-boundary audit as the application plugin. Windows PE, Linux ELF, and macOS Mach-O builds additionally inspect the final external export tables for `scs_telemetry_init` and `scs_telemetry_shutdown` after LTO and symbol stripping. This avoids confusing “the macro expanded” with “the game loader can actually see the symbols.” The proc-macro rustdoc example remains ignored because a reverse dev-dependency from the macro crate to the framework would create a Cargo dependency cycle; the isolated fixture is the long-term test boundary for that consumer contract.

## Explicit subscriptions

A plugin must declare each intended subscription in `initialize`:

```rust
use scs_sdk_plugin::sdk::{
    ChannelFlags, SdkIndex, TelemetryApiVersion, TrailerIndex, channels, game,
};
use scs_sdk_plugin::{
    Game, GameCompatibility, PluginCompatibility, PluginContext, PluginMetadata,
    PluginResult, TelemetryEventKind, TelemetryPlugin,
};

struct Plugin;

static SUPPORTED_GAMES: [GameCompatibility; 1] = [GameCompatibility::new(
    Game::EuroTruckSimulator2,
    game::ets2::V1_00,
)];

impl TelemetryPlugin for Plugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("My Telemetry Plugin", env!("CARGO_PKG_VERSION"))
    }

    fn compatibility(&self) -> PluginCompatibility {
        PluginCompatibility::new(TelemetryApiVersion::V1_00, &SUPPORTED_GAMES)
    }

    fn initialize(&mut self, context: &mut PluginContext<'_>) -> PluginResult {
        context.subscribe_event(TelemetryEventKind::Started)?;
        context.subscribe_event(TelemetryEventKind::FrameEnd)?;
        context.subscribe_event_optional(TelemetryEventKind::Gameplay)?;

        context.subscribe(channels::truck::SPEED)?;
        context.subscribe_optional(channels::truck::NAVIGATION_SPEED_LIMIT)?;
        context.subscribe_with_flags(
            channels::truck::ENGINE_RPM,
            ChannelFlags::EACH_FRAME,
        )?;
        context.subscribe_at(channels::truck::WHEEL_ROTATION, SdkIndex::ZERO)?;
        context.subscribe_trailer(
            channels::trailer::CONNECTED,
            TrailerIndex::ALL[1],
        )?;

        Ok(())
    }
}
```

The different index concepts remain distinct:

- `subscribe(channel)` subscribes to a scalar channel;
- `subscribe_at(channel, sdk_index)` selects an SDK array index such as a wheel or selector;
- `subscribe_trailer(channel, trailer_index)` selects the trailer number encoded in names from `trailer.0.*` through `trailer.9.*`;
- `subscribe_trailer_at(channel, trailer_index, sdk_index)` selects an indexed channel for a specific trailer;
- every group has `_with_flags` variants for explicitly selecting delivery flags such as `EACH_FRAME` and `NO_VALUE`;
- every channel index domain has a matching explicit `subscribe*_optional` family; optional declarations tolerate only `NotFound` and `UnsupportedType`, retain their product-side default when skipped, and never weaken malformed-descriptor, duplicate, lifecycle, or other SDK errors;
- `subscribe_event_optional(event)` skips an event introduced after the negotiated API and tolerates only `Unsupported` or `NotFound` from event registration;
- `Channel::requesting<U>()` explicitly selects the value representation requested from the SDK. The framework rejects a required representation which postdates the negotiated API before registration (`i64` requires Telemetry API 1.01), while SCS remains authoritative for channel-specific conversions. An optional newer representation is skipped.

`SdkIndex::new` rejects only the scalar `SCS_U32_NIL` sentinel. `TrailerIndex::new`
validates the official SDK 1.14 range of `0..10`, and `TrailerIndex::ALL`
provides all ten statically valid values. Configuration callbacks expose
`TrailerConfigurationId`, preserving the difference between legacy `trailer`
and numbered `trailer.0`; a legacy identity is not silently rewritten into
`TrailerIndex::ZERO`.

Enum-like SDK strings use a typed-known/raw-unknown pair of APIs.
`ConfigurationEvent::shifter_type`, `ConfigurationEvent::job_market`, and
`GameplayEvent::fine_offence` return a known SDK 1.14 enum when possible. The
generic `string` and `string_owned` accessors remain available beside them, so
a future game value is preserved verbatim rather than discarded. The same
catalog enums implement `FromStr`, and `GameInfo::supports` applies the detected
game kind and schema to any `GameSchemaAvailability` value.

`TelemetryPlugin::initialize` has no default implementation. If an empty plugin makes no explicit subscriptions, the runtime issues zero event or channel registrations to the SDK. Duplicate subscriptions return `AlreadyRegistered` before calling the SDK, while subscriptions attempted during a callback or shutdown return `NotNow`.

Explicit intent does not push resource management into the product. The runtime commits registrations only after the plugin returns successfully. An expected capability absence skips only its optional declaration. Every required failure and every non-capability optional failure rolls the completed prefix back in reverse order. Normal shutdown follows the same reverse-order rule, and committed counts include only registrations which actually succeeded.

## Example plugin boundary

`examples/telemetry-plugin/` is a real in-game probe and the framework's application-boundary example. It depends only on `scs-sdk-plugin`, explicitly subscribes to six event classes and eight channels, then uses typed callbacks to update a snapshot and record job configuration and gameplay events.

Source in this directory targets:

- zero `unsafe`;
- zero raw pointers;
- zero handwritten foreign ABI;
- zero C string types or literals;
- zero access to `scs-sdk-sys` or `::sys`;
- zero access to the macro-hygiene implementation in `scs_sdk_plugin::__private`.

Use the same repository check as CI to ensure this boundary has not regressed:

```bash
scripts/check-plugin-boundary.sh
```

The script audits both Rust source and `Cargo.toml`, preventing the application from depending directly on `scs-sdk-sys` or using the macro-only, doc-hidden `__private` module to reach the raw ABI. The wrapper and runtime still contain necessary `unsafe` blocks with explicit Safety contracts. Making those blocks disappear cosmetically would hide FFI preconditions rather than improve the boundary.

## Loader fallback E2E example

`examples/telemetry-fallback-plugin/` is a separate manual real-ETS probe for
the loader rule documented by SCS: the game tries Telemetry API versions from
newest to oldest and retries only after `scs_telemetry_init` returns
`SCS_RESULT_unsupported`.

The probe declares API 1.00 as its compatibility minimum so both SDK 1.14
attempts reach product initialization. It then deliberately rejects 1.01 with
`SdkError::Unsupported`, accepts exactly 1.00, and registers only two API-1.00
events plus the scalar truck-speed channel. This is test behavior, not a
compatibility pattern for ordinary plugins.

The expected `game.log.txt` evidence is, in order:

1. `[scs-sdk-fallback-example] requesting loader retry` for API 1.01 with
   `result=unsupported`;
2. `[scs-sdk-fallback-example] rejected attempt cleaned` for API 1.01;
3. `[scs-sdk-fallback-example] accepted loader fallback` for API 1.00;
4. framework initialization with `events=2 channels=1`;
5. `[scs-sdk-fallback-example] fallback callbacks confirmed` under API 1.00,
   carrying both `frame_end_seen=true` and a decoded `speed_metres_per_second`
   value so event and channel delivery are independently evidenced;
6. a clean fallback-session shutdown.

The normal and fallback examples must be installed one at a time. Their macOS
installers verify the selected artifact first, then remove only the exact
alternate and legacy example filenames so the game log has one negotiation
sequence.

## Workspace

```text
crates/scs-sdk-sys/             SDK 1.14 raw x86-64 ABI
crates/scs-sdk/                 no_std typed wrapper and complete catalogs
crates/scs-sdk-plugin/          safe plugin lifecycle framework
crates/scs-sdk-plugin-macros/   SCS entry-point proc macro
  tests/fixtures/export-plugin/ isolated macro compile-pass/fail cdylib workspace
examples/telemetry-plugin/      safe Rust in-game example cdylib
examples/telemetry-fallback-plugin/
                                manual real-ETS API fallback E2E cdylib
scripts/                        Windows/Linux/macOS builds and artifact verification
third-party/scs_sdk_1_14/       original official SDK distribution and license
tmp/                            local investigations, log conclusions, and design notes
```

Product applications, bridges, web interfaces, dispatch logic, save-game integration, and other end-user components are intentionally outside this SDK workspace.

## Development environment

The repository pins Rust `1.85.0` in `rust-toolchain.toml` and declares the `rustfmt` and `clippy` components plus the Windows GNU, Linux GNU, and macOS x86-64 targets.

Base requirements:

- rustup;
- Cargo;
- Bash;
- `file`;
- nightly Miri for provenance and lifecycle verification.

Install Miri with:

```bash
rustup toolchain install nightly-2026-04-12 \
  --profile minimal \
  --component miri \
  --component rust-src
```

`rust-src` is required to build the Miri sysroot. The pinned date matches CI, while rustup automatically selects the host triple for the current machine.

Cross-compiling for Windows x64 requires MinGW-w64:

```text
x86_64-w64-mingw32-gcc
x86_64-w64-mingw32-objdump
```

Linux x86-64 cross-compilation uses Zig and `cargo-zigbuild`:

```bash
cargo install cargo-zigbuild --version 0.23.0 --locked
```

Linux artifact verification additionally requires an `nm` implementation that can read an ELF dynamic symbol table. The script searches in this order:

1. the explicit path in `$NM`;
2. `x86_64-linux-gnu-nm`;
3. `llvm-nm`;
4. native `nm` on a Linux host.

For example, a Homebrew environment can install the required tools with:

```bash
brew install mingw-w64 zig
```

## Quality gates

Complete local verification:

```bash
cargo fmt --all -- --check
scripts/check-license-copies.sh
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
```

Test coverage includes:

- byte-identical Apache-2.0 and MIT license copies in every publishable crate;
- every SDK result code and channel flag;
- item-by-item raw-name, order, indexing-mode, and duplicate checks across the 107/60/15 catalogs;
- every primitive and geometry tagged-union decoder;
- avoiding reads from an inactive union member for incorrect or unknown tags;
- avoiding reads from uninitialized ABI padding;
- compile-fail doctests proving that `SdkCall` cannot escape and implements neither `Send` nor `Sync`;
- independent compilation, strict Clippy, and safe-source auditing for the passing proc-macro consumer;
- an exact E0277 trait-bound failure when the `TelemetryPlugin` implementation is missing;
- both loader-visible SCS exports in the Windows PE, Linux ELF, and macOS Mach-O fixtures;
- no broken intra-doc links under workspace-wide rustdoc with `-Dwarnings`;
- owned game metadata and the Rust string boundary;
- scalar, indexed, and multi-trailer subscription naming;
- explicit event subscriptions and zero registrations for an empty plugin;
- duplicate and invalid-phase subscription rejection;
- channel and event dispatch;
- reverse rollback after partial initialization and reverse unregistration during shutdown;
- stale-generation callback rejection;
- stable context provenance and leak-free destruction.
- exact fallback-probe policy: reject API 1.01, accept API 1.00, and keep the
  accepted subscription surface compatible with API 1.00.

The workspace uses strict Clippy configuration, in particular rejecting casts that may truncate or lose a sign. Non-test builds additionally reject `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, and `unreachable`.

## Continuous integration

`.github/workflows/rust.yml` retains read-only repository permissions, cancellation of superseded commits on the same branch, path filtering, fixed timeouts, and independent Rust caches. The workspace divides its foundation boundaries into seven parallel gates:

| Job | Verification |
| --- | --- |
| `Format, Clippy, and boundaries` | rustfmt, shell syntax, the safe application boundary, macro compile-pass/fail fixtures, workspace-wide Clippy, and strict rustdoc |
| `Workspace tests` | all workspace unit tests and doctests |
| `Miri (scs-sdk)` | typed values, unions, padding, scope, and catalogs under Miri |
| `Miri (scs-sdk-plugin)` | runtime strict provenance, context lifetimes, and stale-generation behavior |
| `Windows x86-64 plugin` | MinGW release DLLs for both the example and isolated macro fixture, PE32+/x86-64 format, and both dynamic SCS exports |
| `Linux x86-64 plugin (glibc 2.17)` | Zig release shared objects for both the example and isolated macro fixture, ELF/x86-64 format, and both dynamic SCS exports |
| `macOS x86-64 plugin` | release dynamic libraries for the normal example, fallback E2E probe, and isolated macro fixture; Mach-O/x86-64 format, signing, and exact SCS export sets |

CI pins:

```text
Rust/MSRV:          1.85.0
Miri:               nightly-2026-04-12
Zig:                0.16.0
cargo-zigbuild:     0.23.0
Linux glibc floor:  2.17
```

The Windows, Linux, and macOS jobs upload plugin artifacts that have passed format and export checks and retain them for seven days. The macOS job additionally uploads the manual fallback E2E artifact under a distinct name. The workflow runs on pushes to `master`, pull requests targeting `master`, and manual dispatch. It runs automatically only when the SDK foundation, examples, build scripts, toolchain, or workflow itself changes; standalone edits to the README or later web directories do not trigger the complete Miri and cross-platform build suite.

## Building and verification

### Windows x64

```bash
scripts/build-windows-plugin.sh
```

Artifact:

```text
target/x86_64-pc-windows-gnu/release/scs_sdk_telemetry_example.dll
```

The script checks:

- PE32+ DLL format;
- x86-64 architecture;
- that the PE export tables contain exactly `scs_telemetry_init` and
  `scs_telemetry_shutdown`, with no additional named or ordinal-only export.

An existing artifact can also be verified independently:

```bash
scripts/verify-windows-plugin.sh PATH_TO_DLL
```

### Linux x86-64

```bash
scripts/build-linux-plugin.sh
```

The build uses a glibc 2.17 baseline:

```text
x86_64-unknown-linux-gnu.2.17
```

Artifact:

```text
target/x86_64-unknown-linux-gnu/release/libscs_sdk_telemetry_example.so
```

The script checks:

- ELF 64-bit LSB shared-object format;
- x86-64 architecture;
- that the defined dynamic-export set is exactly `scs_telemetry_init` and
  `scs_telemetry_shutdown`.

An existing artifact can also be verified independently:

```bash
scripts/verify-linux-plugin.sh PATH_TO_SHARED_OBJECT
```

### macOS x86-64

```bash
scripts/build-macos-plugin.sh
```

The current macOS ETS2 executable is x86-64, including on Apple Silicon where it runs through Rosetta. The build therefore selects `x86_64-apple-darwin` explicitly instead of using the host architecture.

Artifact:

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_example.dylib
```

The script checks:

- Mach-O 64-bit dynamically linked shared-library format;
- x86-64 architecture;
- a valid embedded code signature; local and CI builds use an ad-hoc identity;
- that the defined external-symbol set is exactly `_scs_telemetry_init` and
  `_scs_telemetry_shutdown`, using Mach-O's leading-underscore C ABI spelling.

The ad-hoc signature gives local builds a verifiable code directory but is not Developer ID signing or notarization. A public release pipeline should replace it with a project-owned Developer ID signature and notarized distribution archive.

An existing artifact can also be verified independently:

```bash
scripts/verify-macos-plugin.sh PATH_TO_DYNAMIC_LIBRARY
```

### macOS loader fallback E2E

Build the intentionally version-rejecting probe separately from the normal
example:

```bash
scripts/build-macos-fallback-plugin.sh
```

Artifact:

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_fallback_example.dylib
```

The build applies the same x86-64, code-signing, and exact-export verification
as the normal plugin. Install it as the sole example probe with:

```bash
scripts/install-macos-fallback-plugin.sh
```

Return to the normal six-event/eight-channel example with:

```bash
scripts/install-macos-plugin.sh
```

### Isolated proc-macro fixture

Check only the passing/failing compilation contracts, formatting, Clippy, and safe source:

```bash
scripts/check-plugin-macro-fixtures.sh
```

Build the Windows fixture and verify its real PE exports:

```bash
scripts/build-windows-plugin-macro-fixture.sh
```

Build the Linux glibc 2.17 fixture and verify its real ELF exports:

```bash
scripts/build-linux-plugin-macro-fixture.sh
```

Build the macOS x86-64 fixture and verify its real Mach-O exports:

```bash
scripts/build-macos-plugin-macro-fixture.sh
```

Fixture artifacts are written under `target/plugin-macro-fixtures/`. They exist only to verify the proc-macro consumer contract and are not distributed as example plugins. Install the platform-specific example artifact listed earlier in this section into the game.

## Installing into ETS2

Place the Windows DLL at:

```text
bin/win_x64/plugins/scs_sdk_telemetry_example.dll
```

Place the Linux shared object at:

```text
bin/linux_x64/plugins/libscs_sdk_telemetry_example.so
```

Place the macOS dynamic library at:

```text
<ETS2 installation>/Euro Truck Simulator 2.app/Contents/MacOS/plugins/libscs_sdk_telemetry_example.dylib
```

For the default Steam library under the current user, `<ETS2 installation>` is normally `~/Library/Application Support/Steam/steamapps/common/Euro Truck Simulator 2`. SCS discovers plugins from the `plugins` directory beside the game executable; this is separate from the user-data directory containing profiles and logs.

The repository installer removes a downloaded artifact's quarantine attribute, applies an ad-hoc signature to a private copy, verifies it, and then writes it to that directory:

```bash
scripts/install-macos-plugin.sh
```

Writing into another application bundle is controlled by macOS App Management. If installation reports `Operation not permitted`, allow the terminal under **System Settings -> Privacy & Security -> App Management**, restart the terminal, and run the installer again. The installer deliberately does not re-sign the ETS2 application: doing so would replace SCS Software's Developer ID and notarized application signature.

Do not install multiple telemetry plugins that implement the same probe at the same time, because each will register its own channels and produce duplicate logs. ETS2 may display a confirmation prompt the first time it loads a third-party SDK plugin.

The Windows game log is normally located at:

```text
Documents/Euro Truck Simulator 2/game.log.txt
```

The macOS game log is normally located at:

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

The current probe uses this log prefix:

```text
[scs-sdk-example]
```

## License

Rust code authored for this project is licensed, at your option, under either of:

- [Apache License, Version 2.0](LICENSE-APACHE);
- [MIT License](LICENSE-MIT).

`third-party/scs_sdk_1_14/` originates from SCS Software and is governed by the separate license text distributed with the SDK. That license permits use, modification, and distribution while requiring copies or substantial portions of the software to retain the SCS Software copyright and license notice.
