# `scs-sdk-plugin-macros`

[中文](README.zh.md) | **English**

`scs-sdk-plugin-macros` is the deliberately narrow procedural-macro layer for
exporting safe Rust plugins through the SCS Telemetry and Input loader ABIs. It
owns two independent macros:

```rust
scs_sdk_plugin::export_plugin!(Plugin::default());
scs_sdk_plugin::export_input_plugin!(InputPlugin::default());
```

Most applications should use the re-export from
[`scs-sdk-plugin`](https://crates.io/crates/scs-sdk-plugin) rather than depending on this proc-macro
crate directly.

This is an independent community crate and is not affiliated with or endorsed
by SCS Software.

## Expansion contract

Each macro parses its input as exactly one ordinary Rust expression. The
expression may be a constructor, struct literal, or another expression whose
result implements `TelemetryPlugin` for `export_plugin!` or `InputPlugin` for
`export_input_plugin!`.

For each initialization attempt which passes the framework's ABI, pointer,
version, and lifecycle validation, the expression is evaluated exactly once.
The generated factory coerces its result into the framework plugin trait object,
so a missing `TelemetryPlugin` implementation fails at compile time rather than
at the game ABI boundary.

`export_plugin!` creates:

- one process-lifetime `Runtime` static whose address remains stable while the
  dynamic library is loaded;
- `scs_telemetry_init`; and
- `scs_telemetry_shutdown`.

`export_input_plugin!` independently creates:

- one process-lifetime `InputRuntime` static;
- `scs_input_init`; and
- `scs_input_shutdown`.

Each macro therefore contributes exactly the two loader-visible exports for its
SCS interface. The generated entry points preserve `extern "system"`, the raw
ABI parameter and result types, and the required symbol names. Raw pointers,
unsafe calls, symbol attributes, and ABI documentation are generated inside the
framework boundary; handwritten application source remains ordinary safe Rust.

Invoke each macro at most once in one plugin `cdylib`. A Telemetry and an Input
invocation may coexist in the same library because their runtime statics and
four loader symbols are deliberately independent. Repeating the same macro
would define its fixed runtime and symbols twice.

## Dependency hygiene

The generated code resolves absolute paths through the consumer's direct
`scs-sdk-plugin` dependency. It does not depend on local imports, type aliases,
or implementation details from the invoking source file.

This proc-macro crate does not depend back on `scs-sdk-plugin`, because that
would create a Cargo dependency cycle:

```text
scs-sdk-plugin -> scs-sdk-plugin-macros
       ^                    |
       +--------------------+  forbidden cycle
```

Instead, the generated tokens refer to the public framework contract and its
documented macro-hygiene path. Runtime behavior remains implemented and audited
in `scs-sdk-plugin`.

The proc-macro process itself has only the standard syntax/quoting toolchain:

```text
proc-macro2
quote
syn
```

It contains no SDK adapter, platform linker, global plugin state, or product
dependency.

## What the macro does not own

The macros do not parse Telemetry or Input API versions, validate initialization
pointers, decide compatibility, register subscriptions or devices, dispatch
callbacks, perform rollback or context retirement, interpret SDK values, or
implement product behavior. They generate fixed ABI surfaces and delegate all
lifecycle decisions to the framework `Runtime` or `InputRuntime`.

Keeping this crate small matters: changing generated ABI tokens affects every
consumer even when their application source is unchanged.

## Independent consumer fixtures

An ignored proc-macro doctest is not considered sufficient proof. The repository
keeps an isolated consumer workspace under
[`crates/scs-sdk-plugin/tests/fixtures/export-plugin`](https://github.com/AptS-1547/scs-sdk-crates/tree/master/crates/scs-sdk-plugin/tests/fixtures/export-plugin):

- the Telemetry pass fixture implements `TelemetryPlugin` in safe source and
  expands `export_plugin!(Plugin::default())` as a real `cdylib`;
- the Input pass fixture implements `InputPlugin` in safe source and expands
  `export_input_plugin!(Plugin::default())`;
- the combined pass fixture invokes both macros and must retain exactly all four
  SCS loader exports; and
- the two missing-trait fixtures must fail specifically with Rust error `E0277`
  for the missing `TelemetryPlugin` or `InputPlugin` bound.

Every fixture depends only on the public `scs-sdk-plugin` crate and keeps its
handwritten application source safe.

Release fixture builds then inspect final linked and stripped dynamic libraries,
not merely macro parsing:

| Platform | Artifact verification |
| --- | --- |
| Windows x86-64 | PE architecture and the exact two- or four-export set. |
| Linux x86-64 | ELF architecture and the exact two- or four-export set. |
| macOS x86-64 | Mach-O architecture, signature, and the exact two- or four-export set. |

This catches path-hygiene, trait-bound, LTO, symbol visibility, calling
convention, and final-link regressions across the real public dependency
boundary.

## Validation

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo test --locked -p scs-sdk-plugin-macros
cargo clippy --locked -p scs-sdk-plugin-macros --all-targets -- -D warnings
RUSTDOCFLAGS=-Dwarnings \
  cargo doc --locked -p scs-sdk-plugin-macros --no-deps
scripts/check-plugin-macro-fixtures.sh
scripts/check-plugin-boundary.sh
```

Any expansion or export change also requires:

```bash
scripts/build-windows-plugin-macro-fixture.sh
scripts/build-linux-plugin-macro-fixture.sh
scripts/build-macos-plugin-macro-fixture.sh
```

Each build script verifies the finished Telemetry, Input, and combined platform
artifacts and rejects missing or additional SCS exports.

## License

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

SCS loader symbol names, ABI contracts, and related documentation derived from
SDK 1.0 through 1.14 retain both original SCS Software notices:
[LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013) for SDK 1.0-1.5 and
[LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016) for SDK 1.6-1.14. The
[official SDK archive](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
remains third-party material and is not relicensed under the workspace license.
