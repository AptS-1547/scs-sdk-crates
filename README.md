<div align="center">

# SCS SDK Rust Crates

**Build native SCS telemetry and input-device plugins in safe, typed Rust.**

Complete public Telemetry and Input SDK 1.14 coverage · audited FFI and lifecycle boundaries · verified Windows, Linux, and macOS artifacts

**English** · [简体中文](README.zh.md)

</div>

`scs-sdk-crates` is a reusable Rust foundation for the public telemetry and
input-device interfaces in **SCS SDK 1.14**. It turns the official C ABI into
typed `no_std` bindings, adds safe API-specific plugin runtimes, and provides
real plugin examples that prove the application boundary and final artifacts.

The workspace is implemented entirely in Rust. Plugin authors do not need a C
or C++ shim, CMake, bindgen, raw pointers, handwritten exports, or application
`unsafe`.

> [!IMPORTANT]
> This repository covers the complete public **Telemetry API 1.00/1.01** and
> **Input API 1.00** interfaces present in SCS SDK 1.14. Coverage claims remain
> scoped to those audited public interfaces rather than unspecified future SDK
> additions.

> [!NOTE]
> This is an independent community project and is not affiliated with or
> endorsed by SCS Software. The official files under
> [`third-party/scs_sdk_1_14/`](third-party/scs_sdk_1_14/) remain the ABI and
> constant source of truth. See [Third-Party Notices](THIRD_PARTY_NOTICES.md).

## Why this workspace exists

| Need | What this repository provides |
| --- | --- |
| Auditable SDK coverage | Header-ordered raw catalogs and typed catalogs for all 107 channels, 6 configuration IDs, 60 configuration attributes, 6 gameplay events, and 15 gameplay attributes. |
| Safe application code | Independent `TelemetryPlugin` and `InputPlugin` APIs with typed values, indices, game identity, compatibility, and explicit registration. |
| Shared runtime correctness | Telemetry transactions plus Input device lifetime handling, panic containment, stable callback contexts, poison recovery, and stale-callback isolation. |
| Honest artifact proof | Release scripts inspect PE, ELF, and Mach-O format, x86-64 architecture, and the exact API-specific export set after linking and stripping. |
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

Continue with the [safe plugin framework guide](crates/scs-sdk-plugin/), the
[real telemetry example](examples/telemetry-plugin/), or the
[generic input-device example](examples/input-plugin/). The isolated
[semantical input fixture](examples/input-semantical-plugin/) demonstrates
direct game-mix routing without a binding UI step.

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

examples/input-plugin and examples/input-semantical-plugin use the same layers
through the independent InputPlugin runtime. scs-sdk-plugin-macros generates
the two exports for each selected API.
```

| Layer | Owns | Does not own |
| --- | --- | --- |
| [`scs-sdk-sys`](crates/scs-sdk-sys/) | Raw function pointers, unions, structures, constants, catalogs, layout assertions, and ABI-required `unsafe`. | Typed application policy or plugin lifecycle. |
| [`scs-sdk`](crates/scs-sdk/) | Typed values and descriptors, version domains, catalog enumeration, tagged-union decoding, and callback-scoped calls into SCS. | Global runtime state, product state, or exported symbols. |
| [`scs-sdk-plugin`](crates/scs-sdk-plugin/) | Safe plugin lifecycle, explicit registration, compatibility checks, callback dispatch, rollback, shutdown, and foreign-context ownership. | Networking, storage, dispatch, UI, or save-game features. |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/) | Expansion of safe constructors into telemetry and/or input loader exports. | Runtime policy or product behavior. |
| [`examples/telemetry-plugin`](examples/telemetry-plugin/) | A real safe plugin and end-to-end boundary fixture. | Product functionality. |
| [`examples/input-plugin`](examples/input-plugin/) | A safe generic input device with typed bool/float events. | Hardware integration or product functionality. |
| [`examples/input-semantical-plugin`](examples/input-semantical-plugin/) | An isolated safe semantical device that drives the official `light` bool mix directly. | Generic bindings or product functionality. |

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

## Typed coverage of Input SDK 1.00

Input support is a separate API surface rather than an extension of
`TelemetryPlugin`:

| Surface | Coverage |
| --- | --- |
| API versions | Input API 1.00 plus distinct ETS2/ATS input game versions |
| Device classes | Generic and semantical |
| Device shape | 1 to 400 explicitly declared bool/float inputs |
| Callback flags | First in frame and first after activation, with unknown bits preserved |
| Lifecycle | Init-only registration, optional activity callback, repeated next-event polling, automatic pre-shutdown unregistration |
| Exports | `scs_input_init` and `scs_input_shutdown` |

`InputIndex`, `InputDeviceId`, `InputAxisValue`, `InputValue`, and
`InputEventFlags` keep callback domains explicit. `InputAxisValue` accepts only
finite normalized positions in the inclusive -1.0 through 1.0 interval; it
rejects invalid values instead of silently clamping them. The runtime validates
device/input names, per-device index bounds, registered value types, panic
containment, and stale contexts from partially failed initialization. The safe
example and isolated macro fixtures contain no application `unsafe`.

The independent semantical fixture follows the official SDK sample's `light`
bool input. A fresh controls file references `semantical.light?0`, so the game
activates and consumes the device without a user binding. Its deterministic
false/true cycle provides direct real-game evidence for the second device
class without mixing that behavior into the generic artifact.

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
| Windows Input | x86-64 GNU | `scripts/build-windows-input-plugin.sh` | `scs_sdk_input_example.dll` |
| Windows Semantical Input | x86-64 GNU | `scripts/build-windows-input-semantical-plugin.sh` | `scs_sdk_input_semantical_example.dll` |
| Linux | x86-64, glibc 2.17 floor via Zig | `scripts/build-linux-plugin.sh` | `libscs_sdk_telemetry_example.so` |
| Linux Input | x86-64, glibc 2.17 floor via Zig | `scripts/build-linux-input-plugin.sh` | `libscs_sdk_input_example.so` |
| Linux Semantical Input | x86-64, glibc 2.17 floor via Zig | `scripts/build-linux-input-semantical-plugin.sh` | `libscs_sdk_input_semantical_example.so` |
| macOS | x86-64, including Apple Silicon through Rosetta | `scripts/build-macos-plugin.sh` | `libscs_sdk_telemetry_example.dylib` |
| macOS Input | x86-64, including Apple Silicon through Rosetta | `scripts/build-macos-input-plugin.sh` | `libscs_sdk_input_example.dylib` |
| macOS Semantical Input | x86-64, including Apple Silicon through Rosetta | `scripts/build-macos-input-semantical-plugin.sh` | `libscs_sdk_input_semantical_example.dylib` |

Verification checks the native format, x86-64 architecture, and exact dynamic
export set. macOS builds additionally receive and verify an ad-hoc signature
for local loading; this is not Developer ID signing or notarization.

For installation paths, live log checks, and expected runtime markers, use the
[example's platform and ETS2 validation guide](examples/telemetry-plugin/README.md#build-and-verify).

## Releases and crates.io publication

Pushing a tag that exactly matches the workspace version, such as `v0.1.0`,
starts [the release workflow](.github/workflows/release.yml). The tagged commit
must be contained in `origin/master`; tags pointing at an unmerged commit are
rejected before any publication starts.

The workflow runs the complete quality, package, Miri, and platform gates, then
publishes the four crates in dependency order:

```text
scs-sdk-sys -> scs-sdk
scs-sdk-plugin-macros -> scs-sdk-plugin
scs-sdk + scs-sdk-sys + scs-sdk-plugin-macros -> scs-sdk-plugin
```

The `publish-crates` job uses the GitHub environment `crates-io` and expects its
`CARGO_REGISTRY_TOKEN` secret. Exact crate versions already visible
on crates.io are skipped, so a failed workflow can resume without attempting a
duplicate publication. Newly published dependencies are polled through Cargo's
registry index before their dependents are published.

If a run fails after an irreversible publication step, keep the existing tag
fixed and resume the complete workflow from the current default-branch workflow
definition:

```bash
gh workflow run release.yml --ref master -f release_tag=v0.1.0
```

The recovery run still checks out and builds the exact tagged source. Only its
workflow helpers come from `master`, allowing a CI-only repair without moving a
tag or attempting a manual duplicate `cargo publish`.

Only after every crate is visible does the workflow prepare a draft GitHub
Release, upload all assets, compare the remote asset list with the expected
list, and publish the draft. Stable tags become the latest release; semantic
prerelease tags such as `v0.2.0-rc.1` remain prereleases.

Each release contains three platform archives:

```text
scs-sdk-crates-v0.1.0-windows-x86_64.zip
scs-sdk-crates-v0.1.0-linux-x86_64-glibc-2.17.tar.gz
scs-sdk-crates-v0.1.0-macos-x86_64.tar.gz
```

Every archive contains the verified Telemetry, Generic Input, and Semantical
Input example libraries plus the English/Chinese READMEs, workspace licenses,
preserved SCS SDK notices, and third-party notices. The two Input examples are
mutually exclusive installation fixtures: install at most one of them at a
time. The Telemetry example may coexist with the selected Input example.

Release archives are covered by `checksums.txt`, which is signed through GitHub
Actions keyless OIDC with Sigstore cosign. Verify a normal tag-triggered release
with the exact tag and workflow identity:

```bash
TAG=v0.1.0

cosign verify-blob checksums.txt \
  --bundle checksums.txt.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity \
    "https://github.com/AptS-1547/scs-sdk-crates/.github/workflows/release.yml@refs/tags/$TAG"

sha256sum -c checksums.txt
```

A manually resumed release is signed by the default-branch workflow identity
instead. Use the exact `--certificate-identity` printed in that release's notes;
for a recovery dispatched from `master`, it ends in `@refs/heads/master`.

On macOS, use `shasum -a 256 -c checksums.txt` for the final hash check.

## Repository map

```text
crates/scs-sdk-sys/             raw no_std x86-64 ABI
crates/scs-sdk/                 safe no_std typed wrapper and catalogs
crates/scs-sdk-plugin/          safe lifecycle/runtime/framework
crates/scs-sdk-plugin-macros/   exported-entry-point proc macro
examples/input-plugin/          safe generic input-device plugin
examples/input-semantical-plugin/
                                direct semantical light-mix E2E plugin
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
| [Input example](examples/input-plugin/) | Seeing explicit device registration and frame-scoped bool/float event generation. |
| [Semantical input example](examples/input-semantical-plugin/) | Proving direct `semantical.light?0` routing without controller binding. |
| [`scs-sdk`](crates/scs-sdk/) | Typed descriptors, values, indices, versions, schema history, and decoding. |
| [`scs-sdk-sys`](crates/scs-sdk-sys/) | Auditing the raw ABI and official header mapping. |
| [`scs-sdk-plugin-macros`](crates/scs-sdk-plugin-macros/) | Reviewing the exported-entry-point contract and independent consumer fixtures. |
| [Fallback E2E probe](examples/telemetry-fallback-plugin/) | Reproducing SCS loader negotiation from API 1.01 to 1.00. |

## Development and CI

The workspace pins Rust `1.85.0` and keeps formatting/boundary checks,
workspace tests, Miri, and all three platform artifact builds as separate CI
gates. The independent proc-macro consumer fixture must compile as a real
`cdylib`, preserve the exact telemetry/input export sets after release
linking, support a combined four-export artifact, and fail with the expected
trait-bound diagnostic when either plugin trait is missing.

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
