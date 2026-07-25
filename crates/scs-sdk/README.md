# `scs-sdk`

[中文](README.zh.md) | **English**

`scs-sdk` is the safe, typed, `no_std` interpretation of
[`scs-sdk-sys`](https://crates.io/crates/scs-sdk-sys) for the public telemetry and input interfaces in SCS SDK 1.14
interface. It turns the header-shaped ABI into typed Rust values, descriptors,
catalogs, version domains, and callback-scoped SDK operations without taking
ownership of plugin lifecycle or product state.

> Telemetry and Input version domains and call capabilities remain independent.

This is an independent community crate and is not affiliated with or endorsed
by SCS Software.

## Layer responsibility

This layer owns:

- audited decoding from raw SCS values into typed Rust representations;
- strongly typed Telemetry API and per-game telemetry schema versions;
- channel, configuration, gameplay, event, and attribute descriptors;
- borrowed callback views whose lifetime cannot exceed the callback data;
- scoped registration, unregistration, and logging calls back into SCS; and
- input-device declarations, input indices/values/flags, init-only device
  registration, and callback-scoped input logging; and
- complete enumerable catalogs with official capability history.

It deliberately does **not** own exported loader symbols, process-global
runtime state, callback-context allocation, registration transactions, panic
containment, or a product's telemetry state. Those mechanisms belong in
[`scs-sdk-plugin`](https://crates.io/crates/scs-sdk-plugin) or the application.

The crate remains `no_std` and does not add `std` or `alloc` merely for owned
strings or collections. A higher layer copies foreign data when it must outlive
the direct callback scope.

## API and version model

`TelemetryApi` is the single audited entry point for adapting SCS
initialization structures. Its exact supported list is:

```text
TelemetryApiVersion::V1_00
TelemetryApiVersion::V1_01
```

Unknown raw versions remain representable for diagnostics, but are not silently
reinterpreted as the newest known layout. `TelemetryApiVersion` describes the
plugin ABI negotiated with the loader; `GameSchemaVersion` describes one
game’s telemetry descriptor schema. They are separate strong types and must not
be confused with an SDK archive suffix or a public game patch version.

`InputApi` separately accepts only `InputApiVersion::V1_00`.
`InputGameVersion` is distinct from both telemetry version types.
`InputInitCall` alone can register a device, while `InputCall` and
`InputSession` keep later callbacks and shutdown scoped to direct SCS calls.

Key scoped types include:

- `TelemetryApi<'a>`: validated initialization data and the selected ABI
  adapter;
- `TelemetrySession`: callback registration and logging functions captured from
  the validated API;
- `SdkCall<'scope>`: a non-escaping call capability available only while SCS is
  directly invoking the plugin on the game main thread; and
- `ScopedLogger<'scope>`: bounded temporary C-string construction for one
  permitted SDK call scope.

`SdkCall` is deliberately non-storable, non-`Send`, and non-`Sync`. The safe API
does not suggest that SDK calls may be retained or moved to a worker thread.
Every official non-success result maps to a distinct `SdkError` variant instead
of losing information in a generic failure.

## Typed values and callback views

`ValueRef<'a>` checks the raw SCS value tag before reading the corresponding C
union member. Unknown or mismatched tags produce `None`; inactive union members
are never inspected. Borrowed strings and named values remain tied to the
foreign callback lifetime.

Rust-owned geometry types—`FVector`, `DVector`, `Euler`, `FPlacement`, and
`DPlacement`—copy only meaningful fields. They do not copy ABI padding which
SCS is not required to initialize.

`InputAxisValue` models the normalized float domain used by game input axes.
Its constructor preserves every finite value from -1.0 through 1.0, including
negative zero, and rejects NaN, infinities, and finite out-of-range values.
`InputValue::Float` carries this strong type rather than an arbitrary `f32`, so
safe application code cannot send a value which the game interprets as an
invalid neutral input.

`NamedValues<'a>` implements the SDK sentinel iteration contract for
configuration and gameplay payloads. It terminates at the foreign sentinel and
validates every entry independently rather than scanning arbitrary memory.

## Descriptors, catalogs, and indices

Typed descriptors preserve the expected value representation:

```rust
use scs_sdk::{Attribute, Channel};
```

`Channel<T>` and `Attribute<T>` are used for decoding and registration, while
`AnyChannel` and `AnyAttribute` retain names, value types, indexing metadata,
and availability for enumeration and diagnostics.

Three index domains remain deliberately distinct:

| Type | Meaning |
| --- | --- |
| `SdkIndex` | Explicit SCS array slot attached to an indexed value. |
| `TrailerIndex` | Numbered trailer namespace such as `trailer.0`. |
| `TrailerConfigurationId` | Callback identity distinguishing legacy `trailer` from numbered trailer configurations. |

Application APIs should not replace these domains with an ambiguous bare
integer.

The complete typed telemetry inventory is:

| Surface | Count |
| --- | ---: |
| Channels | 107 |
| Configuration IDs | 6 |
| Configuration attributes | 60 |
| Configuration-to-attribute associations | 71 |
| Gameplay events | 6 |
| Gameplay attributes | 15 |
| Gameplay-to-attribute associations | 21 |

Documented enum-like strings also have closed known-value catalogs:

| Catalog | Known values |
| --- | ---: |
| `configuration::ShifterType` | 4 |
| `configuration::JobMarket` | 5 |
| `gameplay::FineOffence` | 14 |

Each closed catalog exposes `ALL`, `COUNT`, `as_str`, `FromStr`, and per-value
schema availability. A failed known-value parse does not destroy the original
text: generic string access remains available so future additive SDK values can
still be logged and preserved.

## Capability history

Every built-in channel, configuration descriptor, gameplay descriptor, and
association carries `GameSchemaAvailability` with independently researched
ETS2 and ATS minima. These values come from official historical SDK headers and
their changelog comments—not from the SDK archive number or Telemetry API.

Representation-level and event-level API history stays separate. For example,
signed 64-bit values and gameplay events require Telemetry API 1.01, while
individual descriptors may also require a later game schema.

The plugin framework combines this metadata with detected `GameInfo` before it
asks SCS to register a capability.

## Raw escape hatch

The underlying ABI crate is deliberately available as:

```rust
pub use scs_sdk_sys as sys;
```

This exists for wrapper and runtime work, not as the normal application path.
Ordinary product plugins should use the safe descriptors and the
`scs-sdk-plugin::sdk` re-export instead of importing `sys` types.

## Validation

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk
cargo clippy --locked -p scs-sdk --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk --no-deps
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
```

Public API, descriptor, registration, version, or value changes must also prove
that the framework still exposes the wrapper without leaking raw details:

```bash
cargo test --locked -p scs-sdk-plugin
scripts/check-plugin-boundary.sh
```

## License

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

Typed descriptors, constants, identifiers, catalogs, schema-history metadata,
and related documentation derived from SDK 1.0 through 1.14 retain both
original SCS Software notices: [LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013)
for SDK 1.0-1.5 and [LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016) for SDK
1.6-1.14. The
[official SDK archive](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
remains third-party material and is not relicensed under the workspace license.
