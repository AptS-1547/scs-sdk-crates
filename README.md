<div align="center">

# SCS SDK Rust Crates

**Build native SCS telemetry plugins in safe, typed Rust.**

Complete Telemetry SDK 1.14 coverage · audited FFI and lifecycle boundaries · verified Windows, Linux, and macOS artifacts

**English** · [简体中文](README.zh.md)

</div>

`scs-sdk-crates` is a reusable Rust foundation for the public **SCS Telemetry
SDK 1.14** interface. It turns the official C ABI into typed `no_std` bindings,
adds a safe plugin runtime, and provides a real ETS2 plugin that proves the
application boundary and final native artifacts.

The workspace is implemented entirely in Rust. Plugin authors do not need a C
or C++ shim, CMake, bindgen, raw pointers, handwritten exports, or application
`unsafe`.

> [!IMPORTANT]
> This repository covers the complete public **telemetry** interface in SCS SDK
> 1.14—not the entire SCS SDK. The input-device API is outside the currently
> implemented scope.

> [!NOTE]
> This is an independent community project and is not affiliated with or
> endorsed by SCS Software. The official files under
> [`third-party/scs_sdk_1_14/`](third-party/scs_sdk_1_14/) remain the ABI and
> constant source of truth. See [Third-Party Notices](THIRD_PARTY_NOTICES.md).

## Why this workspace exists

| Need | What this repository provides |
| --- | --- |
| Auditable SDK coverage | Header-ordered raw catalogs and typed catalogs for all 107 channels, 6 configuration IDs, 60 configuration attributes, 6 gameplay events, and 15 gameplay attributes. |
| Safe application code | A `TelemetryPlugin` API with typed channels, events, values, indices, game identity, and compatibility declarations. |
| Shared runtime correctness | Transactional registration, reverse rollback and shutdown, panic containment, stable callback contexts, poison recovery, and stale-callback isolation. |
| Honest artifact proof | Release scripts inspect PE, ELF, and Mach-O format, x86-64 architecture, and the exact two SCS loader exports after linking and stripping. |
| Real game evidence | A safe example runs in ETS2 with six event classes and eight channels; a separate probe verifies the loader's documented API fallback sequence. |

The result is deliberately a foundation rather than a product plugin. Web
bridges, dispatch logic, persistence, save-game handling, and user interfaces
belong in downstream repositories.

## Start here

The repository example is the canonical integration path while the crates are
developed together in this workspace:

```bash
git clone https://github.com/AptS-1547/scs-sdk-crates.git
cd scs-sdk-crates

cargo test --workspace --locked
scripts/check-plugin-boundary.sh
```

A minimal plugin depends only on `scs-sdk-plugin`. Its handwritten source can
forbid unsafe code at the application boundary:

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

The macro generates exactly the two loader-visible entry points:

```text
scs_telemetry_init
scs_telemetry_shutdown
```

Continue with the [safe plugin framework guide](crates/scs-sdk-plugin/) or the
[real telemetry example](examples/telemetry-plugin/).

## Proof before promises

The primary example is not a hidden product crate or a compile-only snippet. It
is a safe application-boundary fixture built as a real `cdylib`, checked by CI,
and exercised inside ETS2.

Its startup sequence reports framework identity, negotiated versions, detected
game identity, and committed subscriptions:

```text
[scs-sdk-plugin] starting plugin name="SCS SDK Telemetry Example" version="0.1.0" framework_version="0.1.0"
[scs-sdk-plugin] detected game_display_name="Euro Truck Simulator 2 1.60.1.7s" game_id="eut2" telemetry_api=1.1 telemetry_schema=1.19
[scs-sdk-plugin] initialized plugin name="SCS SDK Telemetry Example" version="0.1.0" events=6 channels=8
```

During gameplay it decodes typed snapshots, job configuration, and all six SDK
1.14 gameplay payload shapes. One representative snapshot looks like this:

```text
[scs-sdk-example] probe speed=85.3km/h rpm=1485 gear=16 \
position=(27077.895,2.983,-8572.040) heading=0.7134 \
navigation_distance=705.64km navigation_time=39887s \
speed_limit=80.0km/h cargo_damage=0.000
```

The separate
[`telemetry-fallback-plugin`](examples/telemetry-fallback-plugin/) deliberately
rejects Telemetry API 1.01 with `Unsupported`, accepts exactly 1.00, and proves
that the SCS loader retries the older API before delivering event and channel
callbacks. It remains isolated from the normal example so the two negotiation
contracts cannot be confused.

## Four layers, one direction

```text
examples/telemetry-plugin
        │ safe TelemetryPlugin API
        ▼
scs-sdk-plugin          lifecycle, registration, callbacks, runtime
        │ typed SDK operations
        ▼
scs-sdk                 no_std values, descriptors, catalogs, decoding
        │ raw ABI
        ▼
scs-sdk-sys             no_std x86-64 C ABI definitions

scs-sdk-plugin-macros   generates the two exported loader entry points
```

| Layer | Owns | Does not own |
| --- | --- | --- |
| [`scs-sdk-sys`](crates/scs-sdk-sys/) | Raw function pointers, unions, structures, constants, catalogs, layout assertions, and ABI-required `unsafe`. | Typed application policy or plugin lifecycle. |
| [`scs-sdk`](crates/scs-sdk/) | Typed values and descriptors, version domains, catalog enumeration, tagged-union decoding, and callback-scoped calls into SCS. | Global runtime state, product state, or exported symbols. |
| [`scs-sdk-plugin`](crates/scs-sdk-plugin/) | Safe plugin lifecycle, explicit registration, compatibility checks, callback dispatch, rollback, shutdown, and foreign-context ownership. | Networking, storage, dispatch, UI, or save-game features. |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/) | Expansion of a safe constructor expression into the two SCS loader exports. | Runtime policy or an application ABI surface. |
| [`examples/telemetry-plugin`](examples/telemetry-plugin/) | A real safe plugin and end-to-end boundary fixture. | Product functionality. |

The Cargo dependency direction remains:

```text
scs-sdk-sys ← scs-sdk ← scs-sdk-plugin ← application
                                     ↖ scs-sdk-plugin-macros
```

## Typed coverage of Telemetry SDK 1.14

The inventory is derived from the official headers and checked for names,
counts, ordering, duplicates, value types, associations, and indexed/scalar
behavior.

| Surface | Count | Typed entry point |
| --- | ---: | --- |
| Channels | **107** | `channels::ALL` |
| Configuration IDs | **6** | `configuration::ids::ALL` |
| Configuration attributes | **60** | `configuration::attributes::ALL` |
| Configuration associations | **71** | `configuration::associations::ALL` |
| Gameplay events | **6** | `gameplay::events::ALL` |
| Gameplay attributes | **15** | `gameplay::attributes::ALL` |
| Gameplay associations | **21** | `gameplay::associations::ALL` |
| H-shifter values | **4** | `ShifterType::ALL` |
| Job-market values | **5** | `JobMarket::ALL` |
| Fine-offence values | **14** | `FineOffence::ALL` |

Coverage also includes every public telemetry ABI value type, result code,
delivery flag, initialization structure, callback payload, game ID, Telemetry
API version, and ETS2/ATS telemetry game-version constant in SDK 1.14.

Typed access does not erase future data. Known enum-like strings have closed
catalogs with `ALL`, `COUNT`, `as_str`, `FromStr`, and schema availability,
while generic string access keeps an unknown future value available verbatim.

## Contracts kept explicit

- **Subscriptions are declared, not inferred.** Implementing a callback does
  not register it, and the framework does not subscribe an entire catalog.
- **Index domains stay distinct.** `SdkIndex`, `TrailerIndex`, and
  `TrailerConfigurationId` cannot be silently interchanged; legacy `trailer`
  remains different from numbered `trailer.0`.
- **Required and optional registrations stay different.** Optional channels
  tolerate only `NotFound` and `UnsupportedType`; optional events tolerate only
  `Unsupported` and `NotFound`. Other failures preserve transactional rollback.
- **Version domains stay distinct.** The SDK archive suffix, negotiated
  `TelemetryApiVersion`, per-game `GameSchemaVersion`, and public game version
  are not interchangeable.
- **Future ABIs are rejected deliberately.** Unknown raw versions remain
  diagnosable but are not reinterpreted as the newest known layout.
- **Game identity remains lossless.** ETS2 and ATS are typed variants; an
  unknown game ID remains an owned `Other` value rather than being collapsed.
- **SDK calls remain scoped.** `SdkCall` is non-storable, non-`Send`, and
  non-`Sync`, matching SCS's callback-scope and main-thread requirements.

Capability history is also explicit. Every descriptor and association carries
independently researched ETS2 and ATS schema minima from the official SDK 1.0
through 1.14 headers. API-level requirements—such as Telemetry API 1.01 for
gameplay events and signed 64-bit values—remain separate from per-game schema
availability.

## Runtime safety model

The framework centralizes the lifecycle mechanisms that downstream plugins
should not rewrite:

- initialization and SDK registration form one transaction;
- a partial failure rolls completed work back in reverse order;
- normal shutdown unregisters in reverse order;
- panics are contained before every foreign ABI boundary;
- mutex poison is recovered deliberately;
- callback contexts have stable allocation addresses and valid provenance;
- failed unregistration retains foreign-visible contexts rather than freeing
  memory the SDK may still reference; and
- session generations isolate delayed callbacks from later plugin instances.

Unsafe operations remain in the smallest audited FFI and runtime boundaries.
The wrapper never reads an inactive tagged-union member or ABI padding that SCS
is not required to initialize. The callback ownership model is tested with Miri
strict provenance.

## Build verified native plugins

Run the platform script from the repository root. Each script builds the safe
example and then validates the final artifact rather than trusting Cargo's
target directory or filename extension.

| Platform | Target and compatibility | Command | Artifact |
| --- | --- | --- | --- |
| Windows | x86-64 GNU | `scripts/build-windows-plugin.sh` | `scs_sdk_telemetry_example.dll` |
| Linux | x86-64, glibc 2.17 floor via Zig | `scripts/build-linux-plugin.sh` | `libscs_sdk_telemetry_example.so` |
| macOS | x86-64, including Apple Silicon through Rosetta | `scripts/build-macos-plugin.sh` | `libscs_sdk_telemetry_example.dylib` |

Verification checks the native format, x86-64 architecture, and exact dynamic
export set. macOS builds additionally receive and verify an ad-hoc signature
for local loading; this is not Developer ID signing or notarization.

For installation paths, live log checks, and expected runtime markers, use the
[example's platform and ETS2 validation guide](examples/telemetry-plugin/#build-and-verify).

## Repository map

```text
crates/scs-sdk-sys/             raw no_std x86-64 ABI
crates/scs-sdk/                 safe no_std typed wrapper and catalogs
crates/scs-sdk-plugin/          safe lifecycle/runtime/framework
crates/scs-sdk-plugin-macros/   exported-entry-point proc macro
examples/telemetry-plugin/      real safe application-boundary plugin
examples/telemetry-fallback-plugin/
                                manual loader fallback E2E probe
scripts/                        boundary, build, install, and artifact checks
third-party/scs_sdk_1_14/       official SDK 1.14 distribution
third-party/scs_sdk_history/    official SDK 1.0–1.14 history and notices
```

| Read next | Use it for |
| --- | --- |
| [`scs-sdk-plugin`](crates/scs-sdk-plugin/) | Writing a safe plugin and understanding lifecycle guarantees. |
| [Telemetry example](examples/telemetry-plugin/) | Seeing explicit subscriptions, typed callbacks, build artifacts, and real ETS2 validation. |
| [`scs-sdk`](crates/scs-sdk/) | Typed descriptors, values, indices, versions, schema history, and decoding. |
| [`scs-sdk-sys`](crates/scs-sdk-sys/) | Auditing the raw ABI and official header mapping. |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/) | Reviewing the exported-entry-point contract and independent consumer fixtures. |
| [Fallback E2E probe](examples/telemetry-fallback-plugin/) | Reproducing SCS loader negotiation from API 1.01 to 1.00. |

## Development and CI

The workspace pins Rust `1.85.0` and keeps formatting/boundary checks,
workspace tests, Miri, and all three platform artifact builds as separate CI
gates. The independent proc-macro consumer fixture must compile as a real
`cdylib`, preserve both exports after release linking, and fail with the
expected trait-bound diagnostic when `TelemetryPlugin` is missing.

<details>
<summary><strong>Run the complete local foundation gate</strong></summary>

```bash
cargo fmt --all -- --check
bash -n scripts/*.sh
scripts/check-license-copies.sh
scripts/check-plugin-boundary.sh
scripts/check-plugin-macro-fixtures.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
MIRIFLAGS=-Zmiri-strict-provenance \
  cargo +nightly-2026-04-12 miri test --locked -p scs-sdk-plugin
git diff --check
git diff --cached --check
```

Changes to release artifacts or exports also require the applicable Windows,
Linux, and macOS build scripts above. A host-only Cargo build is not
cross-platform artifact proof.

</details>

## License and attribution

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

Official SDK files and SCS-derived ABI declarations, constants, identifiers,
catalogs, schema-history metadata, and related documentation retain SCS
Software's notices. SDK 1.0–1.5 use
[the 2013 notice](LICENSE-SCS-SDK-2013), while SDK 1.6–1.14 use
[the 2016 notice](LICENSE-SCS-SDK-2016). These materials are not relicensed
under the workspace license.

See [Third-Party Notices](THIRD_PARTY_NOTICES.md) for the complete attribution
and independence statement.
