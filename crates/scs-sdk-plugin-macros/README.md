# `scs-sdk-plugin-macros`

[中文](README.zh.md) | **English**

`scs-sdk-plugin-macros` is the deliberately narrow procedural-macro layer for
exporting a safe Rust plugin through the SCS telemetry loader ABI. It owns one
macro:

```rust
scs_sdk_plugin::export_plugin!(Plugin::default());
```

Most applications should use the re-export from
[`scs-sdk-plugin`](../scs-sdk-plugin/) rather than depending on this proc-macro
crate directly.

This is an independent community crate and is not affiliated with or endorsed
by SCS Software.

## Expansion contract

`export_plugin!` parses its input as exactly one ordinary Rust expression. The
expression may be a constructor, struct literal, or another expression whose
result implements `TelemetryPlugin`.

For each initialization attempt which passes the framework's ABI, pointer,
version, and lifecycle validation, the expression is evaluated exactly once.
The generated factory coerces its result into the framework plugin trait object,
so a missing `TelemetryPlugin` implementation fails at compile time rather than
at the game ABI boundary.

The expansion creates:

- one process-lifetime `Runtime` static whose address remains stable while the
  dynamic library is loaded;
- `scs_telemetry_init`; and
- `scs_telemetry_shutdown`.

Those are exactly the two loader-visible exports required by SCS. The generated
entry points preserve `extern "system"`, the raw ABI parameter and result types,
and the required symbol names. Raw pointers, unsafe calls, symbol attributes,
and ABI documentation are generated inside the framework boundary; handwritten
application source remains ordinary safe Rust.

Invoke the macro exactly once in one plugin `cdylib`. A second invocation would
attempt to define the same fixed runtime and loader symbols again.

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

The macro does not parse Telemetry API versions, validate initialization
pointers, decide compatibility, register subscriptions, dispatch callbacks,
perform rollback, interpret SDK values, or implement product behavior. It
generates the fixed ABI surface and delegates all lifecycle decisions to the
framework `Runtime`.

Keeping this crate small matters: changing generated ABI tokens affects every
consumer even when their application source is unchanged.

## Independent consumer fixtures

An ignored proc-macro doctest is not considered sufficient proof. The repository
keeps an isolated consumer workspace under
[`../scs-sdk-plugin/tests/fixtures/export-plugin`](../scs-sdk-plugin/tests/fixtures/export-plugin/):

- the pass fixture depends only on the public `scs-sdk-plugin` crate, implements
  `TelemetryPlugin` in safe source, and expands
  `export_plugin!(Plugin::default())` as a real `cdylib`;
- the missing-trait fixture uses the same public boundary but omits the trait
  implementation, and must fail specifically with Rust error `E0277` for the
  missing `TelemetryPlugin` bound.

Release fixture builds then inspect final linked and stripped dynamic libraries,
not merely macro parsing:

| Platform | Artifact verification |
| --- | --- |
| Windows x86-64 | PE shared library architecture and both loader exports. |
| Linux x86-64 | ELF shared object architecture and both loader exports. |
| macOS x86-64 | Mach-O dynamic library architecture and both loader exports. |

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

Each build script verifies the finished platform artifact and both SCS exports.

## License

Workspace-authored Rust code is available under either
[Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

SCS loader symbol names, ABI contracts, and related documentation derived from
SDK 1.0 through 1.14 retain both original SCS Software notices:
[LICENSE-SCS-SDK-2013](LICENSE-SCS-SDK-2013) for SDK 1.0-1.5 and
[LICENSE-SCS-SDK-2016](LICENSE-SCS-SDK-2016) for SDK 1.6-1.14. The
[official SDK archive](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
remains third-party material and is not relicensed under the workspace license.
