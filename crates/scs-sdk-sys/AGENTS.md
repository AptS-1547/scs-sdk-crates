# scs-sdk-sys Instructions

This file supplements the repository-level AGENTS.md for the scs-sdk-sys crate.
Read the official headers under third-party/scs_sdk_1_14/ before changing ABI
definitions or constants.

## Ownership

- This crate is the dependency-free, no_std, x86-64 raw ABI layer for the public
  SCS Telemetry SDK 1.14 headers.
- Mirror C layout, numeric values, function signatures, calling conventions, and
  header-defined byte strings. Do not add plugin lifecycle, owned application
  values, string conversion, logging policy, or ergonomic registration APIs here.
- Keep this crate usable without std, alloc, bindgen, Clang, CMake, or a C/C++
  shim.
- The current target contract is 64-bit SCS game processes. Do not claim or add
  32-bit support without auditing every size, alignment, offset, and callback ABI.

## ABI Rules

- Every repr(C) struct and union must match the official header exactly. Preserve
  field order, integer width, signedness, alignment, and padding behavior.
- Use core FFI types or explicitly sized Rust primitives only when their ABI
  meaning is established by the header.
- Represent padding with MaybeUninit when SCS is not required to initialize it.
  Do not derive or implement behavior that reads, compares, hashes, or formats
  such bytes.
- Keep union access unsafe at this layer. Tag validation and safe decoding belong
  in scs-sdk.
- Function pointer aliases must preserve the SDK calling convention and pointer
  mutability. Do not turn a possibly absent pointer into a guaranteed callable
  function unless the header guarantees it.
- Raw C strings and catalog names are NUL-terminated byte strings. Do not convert
  them to str in this crate or silently repair malformed data.
- Keep compile-time size, alignment, and offset assertions close to the ABI types
  they protect. Add assertions whenever a new externally passed layout is added.

## Constants and Catalogs

- Preserve the exact spelling, value, and header order of SDK constants.
- Raw ALL catalogs must be derived from the same declarations, not maintained as
  a semantically different second inventory.
- Keep channel families separate: common, truck, trailer, and job. Preserve the
  distinction between scalar and indexed channels in the data consumed by the
  typed wrapper.
- Keep Telemetry API versions, game telemetry/schema versions, and public game
  versions distinct. Do not infer support policy in this raw crate.
- Keep the header-shaped SCS_GAME_ID_* constants as the byte-string source of
  truth. Ergonomic games::*::GAME_ID paths may alias those constants, but must
  not repeat the foreign byte strings as independent declarations.
- The current telemetry inventory is 107 channels, 6 configuration IDs,
  60 configuration attributes, 6 gameplay events, and 15 gameplay attributes.
  Any count change requires official-header evidence and synchronized upper-layer
  tests and documentation.
- Do not add input-device declarations piecemeal. Input support needs a complete
  separately reviewed coverage plan across all layers.

## Unsafe and Documentation

- Unsafe declarations are expected at the foreign boundary, but unsafe operations
  still require the smallest possible block and a precise SAFETY explanation.
- Document the originating SDK header, ABI purpose, units, sentinel values, and
  validity rules for non-obvious public declarations.
- Avoid convenience impls such as Default, Eq, or Debug when they would imply that
  every bit pattern, union member, or padding byte is initialized and meaningful.

## Validation

After changes, run at minimum:

    cargo fmt --all -- --check
    cargo test --locked -p scs-sdk-sys
    cargo clippy --locked -p scs-sdk-sys --all-targets -- -D warnings
    RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-sys --no-deps

For ABI, padding, union, catalog, or version changes, also run the scs-sdk tests
and Miri because that crate exercises the safe interpretation of these bindings:

    cargo test --locked -p scs-sdk
    cargo +nightly-2026-04-12 miri test --locked -p scs-sdk

If an exported callback or initialization layout changes, run all platform plugin
build and export-verification scripts before claiming completion.
