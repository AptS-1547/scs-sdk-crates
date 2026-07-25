# Repository Instructions

This file applies to the entire repository. Read the current code, manifests,
scripts, tests, and relevant SDK headers before changing anything. Preserve
existing patterns unless there is a concrete correctness reason to change them.

## Project Identity

- This repository is the reusable scs-sdk-crates foundation. It is not the ETS2
  Dispatch product repository.
- Product plugins, web applications, bridges, dispatch logic, save-game logic,
  and other end-user features belong in separate repositories.
- The current implemented scope is complete coverage of the public SCS
  Telemetry SDK 1.14 interface. Do not describe the whole SCS SDK as covered:
  the input-device API is possible future scope and is not implemented yet.
- The official SDK files in third-party/scs_sdk_1_14/ are the ABI and constant
  source of truth. Do not derive contracts from third-party Rust crates or
  existing product behavior when the official headers answer the question.
- The codebase is pure Rust. Do not introduce a C or C++ shim, CMake, or a
  build-time bindgen/Clang dependency without an explicit architectural decision.

## Repository Boundaries

The reusable workspace consists of these layers:

    scs-sdk-sys             raw no_std x86-64 C ABI definitions
            |
    scs-sdk                 safe no_std typed SDK wrapper and catalogs
            |
    scs-sdk-plugin          safe plugin lifecycle/runtime/framework
            |
    scs-sdk-plugin-macros   exported-entry-point proc macro
            |
    examples/telemetry-plugin
                            real safe application-boundary example

Dependency and ownership rules:

- scs-sdk-sys mirrors the official ABI. Raw pointers, unions, FFI function
  pointers, and carefully justified unsafe are expected only because this is the
  raw layer. Keep it dependency-free and no_std.
- scs-sdk owns typed values, descriptors, catalogs, version types, decoding,
  and scoped calls back into the game. Keep it no_std. It must not own plugin
  lifecycle or product state.
- scs-sdk-plugin owns lifecycle state, registration transactions, callbacks,
  panic containment, context lifetime, rollback, shutdown, and stale-callback
  isolation. Product crates must not reimplement these mechanisms.
- scs-sdk-plugin-macros generates the two SCS loader entry points. Macro
  expansions must reference the public framework contract and must not require
  application authors to handle ABI details.
- examples/telemetry-plugin is an example and an end-to-end boundary fixture,
  not a hidden product crate. Keep its state and behavior useful for real ETS2
  validation, but do not put bridge, web, dispatcher, or save-game features in it.
- Do not create dependency cycles or let higher-level concerns leak into lower
  layers. The intended direction is sys -> sdk -> plugin -> application.

The following untracked paths may contain product work reserved by the user:

    apps/bridge/
    apps/web/
    assets/
    crates/dispatcher/
    crates/protocol/
    crates/savegame/
    crates/telemetry/
    fixtures/

Do not delete, move, edit, format, add to the workspace, or stage these paths
unless the user explicitly changes their ownership. Treat any unrelated
worktree changes as user-owned and preserve them.

## Safety Contract

- Application plugin source must remain safe Rust. It must not contain unsafe,
  raw pointers, handwritten external ABI declarations, CStr, CString, raw SCS
  sys types, or direct access to macro internals.
- The example must continue to pass scripts/check-plugin-boundary.sh. Do not
  weaken or bypass that script to make a change pass.
- Necessary unsafe belongs in the smallest audited wrapper/runtime boundary.
  Every unsafe operation must have a concrete SAFETY comment covering pointer
  validity, layout, lifetime, aliasing, thread, and callback assumptions that
  actually apply.
- Do not hide an unsafe precondition behind a safe public API. If soundness
  cannot be established locally, stop and redesign the ownership or lifetime
  model.
- Never read inactive tagged-union members. Validate the SCS value tag first.
- Do not read ABI padding that the SDK is not required to initialize. Preserve
  MaybeUninit where it represents that contract.
- Calls back into the SDK must stay inside the game's permitted callback scope
  and main thread. Do not make SdkCall storable, escapable, Send, or Sync.
- Callback context allocations must have stable addresses and valid provenance.
  Moving an Arc handle must not be confused with moving its pointee.
- Registration is transactional: partial failure rolls back completed work in
  reverse order, and normal shutdown unregisters in reverse order.
- Contain panics before every foreign ABI boundary. Preserve mutex-poison
  recovery, generation checks, retired-context retention after failed
  unregistration, and stale callback isolation.
- Changes to unsafe code, FFI layout, callback ownership, or runtime lifecycle
  require targeted tests and Miri validation. Do not ship a short-term patch in
  place of a complete soundness model.

## Explicit API Design

- Prefer explicit declarations over inference. A plugin subscribes to each event
  and channel in TelemetryPlugin::initialize; the framework must not infer
  subscriptions from implemented callbacks or automatically subscribe catalogs.
- Keep scalar SDK indices, indexed channel indices, and trailer indices distinct
  in both types and method names. Do not merge them into an ambiguous integer API.
- Use `SdkIndex` for SDK array slots, `TrailerIndex` for the numbered trailer
  namespace, and `TrailerConfigurationId` for callback identity. Application
  code must not use a bare `u32` as one of these domains, and legacy `trailer`
  must remain distinguishable from numbered `trailer.0`.
- Version negotiation must use strong version types and explicit supported
  variants. Keep Telemetry API version, telemetry game/schema version, and public
  game version separate.
- scs-sdk::TelemetryApi is the single source of truth for audited ABI adapters.
  Framework code consumes that result and must not maintain a second API-version
  whitelist. PluginCompatibility separately describes product requirements.
- scs-sdk::Event is the single typed event-identifier catalog. Framework APIs
  may re-export it under a lifecycle-oriented name, but must not duplicate its
  variants, raw discriminator mapping, or official capability metadata.
- Keep required and optional registrations explicit. Optional channels may
  tolerate only NotFound and UnsupportedType; optional events may tolerate only
  Unsupported and NotFound. Every other failure preserves transactional
  rollback, and committed counts include only successful registrations.
- Enforce official API-level capability history before SDK registration:
  gameplay events and signed 64-bit values require Telemetry API 1.01. A newer
  optional capability is skipped; a newer required capability rejects init.
- Reject unsupported versions deliberately. Do not silently reinterpret future
  ABI versions as the newest known layout.
- Preserve unknown game IDs as an owned Other value instead of collapsing them
  into ETS2 or ATS.
- Keep product metadata explicit through PluginMetadata. Runtime logs should
  identify plugin name/version, framework version, detected game identity, API
  version, schema version, and committed subscription counts.
- Use typed Channel<T>, Attribute<T>, event IDs, flags, and owned Rust values in
  public APIs. Type-erased catalogs are for enumeration and diagnostics, not an
  excuse to discard decoding types.
- Treat documented enum-like strings as versioned value catalogs. A complete
  catalog exposes `ALL`, `COUNT`, `as_str`, `FromStr`, and schema availability.
  High-level known-value decoders must coexist with generic string access so a
  future unknown value remains available verbatim.
- Use `GameInfo::minimum_schema_for` and `GameInfo::supports` as the canonical
  framework query for `GameSchemaAvailability`; do not grow parallel
  game-kind/schema matching policies in registration or callback code.
- Avoid speculative abstractions. Add one when it removes demonstrated
  duplication, establishes a safety invariant, or matches the existing layering.

## Complete Coverage Standard

Complete coverage requires auditable evidence, not a large list of constants.
For changes to SDK coverage:

- Compare against the official SDK 1.14 headers.
- Preserve every public telemetry ABI type, result code, version constant, event,
  channel, configuration ID/attribute, gameplay event/attribute, game ID, and
  game-version constant within the declared scope.
- Preserve header ordering in raw ALL catalogs and type/value/index metadata in
  high-level catalogs.
- Descriptor availability belongs to the per-game telemetry schema. Derive
  ETS2 and ATS minima from official historical SDK headers and their changelog
  comments; never substitute the SDK archive suffix or Telemetry API version.
  Keep semantic-fix versions distinct from the version which first introduced
  a descriptor.
- Add or update tests for names, counts, ordering, duplicate detection, value
  types, and indexed/scalar behavior.
- Keep the documented inventory synchronized with code. The current telemetry
  inventory is 107 channels, 6 configuration IDs, 60 configuration attributes,
  71 configuration-to-attribute associations, 6 gameplay events, 15 gameplay
  attributes, and 21 gameplay-to-attribute associations.
- Do not call input-device support implemented until its raw ABI, safe wrapper,
  framework contract, tests, examples, documentation, and cross-platform build
  implications have all been handled.

## Rust Style

- Use the toolchain and MSRV pinned by rust-toolchain.toml; do not raise them
  accidentally through syntax or dependency updates.
- Keep the workspace lints enabled. In particular, deny
  clippy::cast_possible_truncation and clippy::cast_sign_loss.
- In non-test code, avoid unwrap, expect, panic, unreachable, unimplemented, and
  todo. Return or propagate a typed error instead.
- Do not apply an application-level forbid(unsafe_code) policy to the raw FFI and
  audited runtime crates. The goal is contained and reviewed unsafe code, not
  pretending the foreign boundary does not exist. Application fixtures should
  use #![forbid(unsafe_code)] where practical.
- Use checked or explicit conversions when narrowing or changing signedness.
- Keep code ASCII unless an existing file or user-facing fixture requires other
  text.
- Write extensive useful documentation. Public items should explain semantics,
  units, SDK origin, lifetime/phase restrictions, index meaning, error behavior,
  and safety invariants. Internal comments should explain non-obvious ABI,
  ownership, provenance, rollback, and concurrency reasoning. Do not add comments
  that merely restate the next line of code.
- Keep English and Chinese README content synchronized when behavior, commands,
  paths, artifact names, or support claims change.

## Macro Contract

- export_plugin!(Plugin::default()) must generate exactly the two loader-visible
  exports scs_telemetry_init and scs_telemetry_shutdown.
- The calling application should need only a normal safe constructor expression
  whose type implements TelemetryPlugin.
- Preserve the independent consumer fixture under
  crates/scs-sdk-plugin/tests/fixtures/export-plugin/.
- The pass fixture must compile as a real cdylib, retain both exports after
  release linking/stripping, and keep application source safe.
- The missing-trait fixture must fail for the expected TelemetryPlugin bound,
  not for an unrelated path, dependency, or syntax error.
- Do not replace this fixture with an ignored doctest. It exists specifically to
  validate expansion across the real public dependency boundary.

## Platforms and Artifacts

- The supported plugin artifact targets are Windows x86-64, Linux x86-64, and
  macOS x86-64. SCS documents all three platforms.
- macOS ETS2 currently loads an x86-64 plugin, including on Apple Silicon through
  Rosetta. Do not replace the target with arm64 merely because the host is arm64.
- Linux release builds retain the documented glibc 2.17 floor and use the
  repository Zig/cargo-zigbuild flow.
- A successful cargo build is insufficient. Verify file format, x86-64
  architecture, and both loader-visible dynamic exports using repository scripts.
- Keep build, verification, installer, CI artifact, README, and Cargo library
  names synchronized when an example or package is renamed.
- The macOS installer may clear quarantine and ad-hoc sign the plugin itself. Do
  not re-sign the ETS2 application bundle or replace SCS's Developer ID identity.
- Avoid installing two plugin filenames side by side. The installer may remove
  only the exact known legacy example filename after the new artifact verifies.
- Game Archive Packer is for SCS game/mod archives; it is not the packaging format
  for native telemetry DLL, shared-object, or dylib plugins.

## Licensing

- Workspace-authored Rust code is licensed under MIT OR Apache-2.0. Keep package
  manifests, repository license files, and README statements consistent.
- The official SCS SDK distribution remains third-party material under its own
  license. Do not imply that files under third-party/scs_sdk_1_14/ were
  relicensed under the workspace license.
- Preserve upstream license and attribution files when updating vendored SDK
  material or incorporating code from another project.
- Check license compatibility before adding a dependency or copying an
  implementation. A crate being publicly downloadable does not make its source
  interchangeable with this workspace.

## Continuous Integration

- Keep CI repository permissions read-only unless a narrowly scoped publishing
  workflow later requires more.
- Preserve branch concurrency cancellation, explicit job timeouts, locked Cargo
  resolution, pinned tool versions, and relevant path filters.
- Keep formatting/boundary checks, workspace tests, Miri, and the three platform
  artifact builds as distinct gates so a failure identifies the broken contract.
- CI must verify the telemetry example and the independent macro fixture as real
  release dynamic libraries, including architecture and both dynamic exports.
- Do not replace a platform job with host-only cargo check, and do not upload an
  artifact that has not passed the repository verification script.
- When changing pinned Rust, nightly Miri, Zig, cargo-zigbuild, target, or glibc
  versions, update CI, scripts, toolchain files, and both READMEs together and
  explain the compatibility reason.

## Required Workflow

Before editing:

1. Inspect the relevant code, tests, official headers, scripts, and current diff.
2. Establish which layer owns the behavior.
3. Confirm that unrelated or untracked product work will remain untouched.
4. For ambiguous architecture decisions, ask the user instead of guessing.

While editing:

- Keep changes scoped to the requested behavior.
- Add tests with the implementation, scaled to the FFI and lifecycle risk.
- Preserve explicit contracts and public documentation.
- Never resolve a failing check by weakening a safety assertion, boundary audit,
  compile-fail expectation, export check, or warning policy.

Run the relevant subset after each logical batch. Before declaring foundation
work complete, run the full local gate where required toolchains are present:

    cargo fmt --all -- --check
    bash -n scripts/*.sh
    scripts/check-plugin-boundary.sh
    scripts/check-plugin-macro-fixtures.sh
    cargo test --workspace --locked
    cargo clippy --workspace --all-targets --locked -- -D warnings
    RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
    cargo +nightly-2026-04-12 miri test --locked -p scs-sdk
    MIRIFLAGS=-Zmiri-strict-provenance cargo +nightly-2026-04-12 miri test \
      --locked -p scs-sdk-plugin
    git diff --check
    git diff --cached --check

For release artifact or export changes, also run the applicable platform scripts:

    scripts/build-windows-plugin.sh
    scripts/build-linux-plugin.sh
    scripts/build-macos-plugin.sh

If a toolchain or platform check was not run, report that fact explicitly. Do not
claim cross-platform completion based only on compilation for the host platform.

## Git and External Crates

- The GitHub repository is AptS-1547/scs-sdk-crates. The local checkout directory
  may still be named ets2-dispatch; do not infer repository identity from it.
- Do not commit, push, publish crates, create releases, or rename packages unless
  the user explicitly asks.
- Check exact crate-name availability and metadata with
  cargo info --registry crates-io NAME. Do not infer availability from an HTTP
  error or a fuzzy search result.
- Similar crates are references, not sources of truth. Distinguish raw ABI
  bindings, shared-memory reader clients, fixed shared-memory plugin products,
  and a reusable safe plugin framework when comparing projects.
- Current names intentionally describe four separate public responsibilities:
  scs-sdk-sys, scs-sdk, scs-sdk-plugin, and scs-sdk-plugin-macros. Keep those
  boundaries visible rather than collapsing them for download-count optics.

## Real ETS2 Validation

- Treat the live ETS2 log at
  ~/Library/Application Support/Euro Truck Simulator 2/game.log.txt as the
  current runtime source. Files copied into Downloads/ or tmp/ are snapshots and
  may be older.
- Confirm runtime identity through all of: loaded library name, framework startup
  metadata, product log prefix, subscription counts, shutdown metadata, and
  unloaded library name.
- Check for legacy prefixes or a second legacy dylib before declaring a renamed
  plugin installed correctly.
- Preserve meaningful E2E evidence such as API/schema negotiation, event/channel
  counts, callbacks, job configuration, gameplay events, and clean shutdown.
- A displayed speed of -0.0 alone is signed floating-point zero. Confirm reverse
  movement using materially negative speed together with a negative engine gear.
- ETS2 rewrites game.log.txt each run. Copy evidence promptly when a durable
  fixture or investigation record is required.
