# `scs-sdk-sys`

[中文](README.zh.md) | **English**

`scs-sdk-sys` is the dependency-free, `no_std`, handwritten raw ABI layer for
the public **SCS Telemetry SDK 1.14** interface on 64-bit game processes. It
mirrors the C declarations used by Euro Truck Simulator 2 and American Truck
Simulator without adding wrapper policy, plugin lifecycle, or product logic.

The authoritative source is the official header distribution preserved under
[`third-party/scs_sdk_1_14/`](../../third-party/scs_sdk_1_14/). Rust definitions
are reviewed against those files for layout, numeric values, function
signatures, calling conventions, and NUL-terminated byte constants.

> This crate covers the public **telemetry** interface from SDK 1.14. The SDK's
> input-device API is not implemented by this workspace yet.

This is an independent community crate and is not affiliated with or endorsed
by SCS Software.

## Design constraints

- **Pure Rust:** no C or C++ shim, CMake, generated bindings, bindgen, or Clang
  is needed to compile the crate.
- **Dependency-free:** the crate uses only `core` and declares no third-party
  Rust dependencies.
- **`no_std`:** raw bindings do not depend on an operating-system runtime or
  allocation support.
- **x86-64 ABI contract:** a compile-time error rejects targets whose pointer
  width is not 64 bits. The supported plugin artifacts are Windows x86-64,
  Linux x86-64, and macOS x86-64.
- **Header-shaped data:** the crate preserves C field order, signedness,
  widths, constants, and catalog ordering rather than making the API ergonomic.

## Modules

| Module | Raw responsibility |
| --- | --- |
| `constants` | SDK result codes, version constants, log levels, flags, and shared ABI scalar types. |
| `value` | `repr(C)` tagged values, vectors, Euler angles, placements, named values, and the value union. |
| `telemetry` | Initialization structures, function pointers, callback types, event payload layouts, and plugin entry-point types. |
| `channels` | Common, truck, trailer, and job channel byte-string constants plus ordered catalogs. |
| `configuration` | Configuration IDs and attribute constants. |
| `gameplay` | Gameplay event IDs and attribute constants. |
| `games` | Official ETS2 and ATS identifiers and version constants. |

The raw SDK 1.14 telemetry inventory is:

| Surface | Count |
| --- | ---: |
| Channels | 107 |
| Configuration IDs | 6 |
| Configuration attributes | 60 |
| Gameplay event IDs | 6 |
| Gameplay attributes | 15 |

Raw `ALL` catalogs retain header order and raw metadata. Higher layers add
typed descriptors and association catalogs; this crate does not infer those
policies from application behavior.

## ABI and unsafe boundary

Public foreign layouts use `#[repr(C)]`, and callbacks and SDK function
pointers use `extern "system"` so the correct platform calling convention is
selected. Compile-time size, alignment, and offset assertions guard structures
crossing the game/plugin boundary.

Some raw declarations are intentionally awkward:

- tagged values contain a C union, so reading a member remains `unsafe` here;
- padding which SCS is not required to initialize is represented with
  `MaybeUninit<u32>` rather than being read, compared, or formatted;
- foreign strings and catalog names remain raw NUL-terminated byte strings;
- optional function pointers and opaque callback contexts retain their foreign
  pointer representation.

These choices preserve the actual ABI instead of inventing validity guarantees.
Tag checking, lifetime-bound borrowing, typed value decoding, and ergonomic
errors belong in [`scs-sdk`](../scs-sdk/).

## Intended users

This crate is primarily for authors auditing or extending the typed wrapper and
runtime. Ordinary plugin crates should depend on
[`scs-sdk-plugin`](../scs-sdk-plugin/) and use its `sdk` re-export. Application
plugins should not read unions, handle raw pointers, import raw SCS types, or
declare loader entry points themselves.

## Validation

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk-sys
cargo clippy --locked -p scs-sdk-sys --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-sys --no-deps
```

Changes to ABI layout, padding, unions, catalogs, or version constants must also
exercise the safe interpretation layer:

```bash
cargo test --locked -p scs-sdk
cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
```

Exported callback or initialization-layout changes additionally require all
applicable Windows, Linux, and macOS artifact build and export-verification
scripts.

## License

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

ABI declarations, constants, identifiers, catalogs, schema-history metadata,
and related documentation derived from SDK 1.0 through 1.14 retain both
original SCS Software notices: [LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013)
for SDK 1.0-1.5 and [LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016) for SDK
1.6-1.14. The
[official SDK archive](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
remains third-party material and is not relicensed under the workspace license.
