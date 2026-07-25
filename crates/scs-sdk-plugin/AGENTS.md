# scs-sdk-plugin Instructions

This file supplements the repository-level AGENTS.md for scs-sdk-plugin. This
crate owns the safe application framework and the audited runtime boundary.

## Ownership

- Own TelemetryPlugin, PluginContext, PluginMetadata, owned GameInfo, safe event
  payloads, explicit subscription APIs, runtime state, and callback dispatch.
- Own every lifecycle mechanism shared by plugins: initialization, registration
  commit, reverse rollback, reverse shutdown, reinitialization, panic containment,
  poison recovery, generation isolation, and foreign context retention.
- Do not put product telemetry state, dispatcher behavior, shared-memory protocol,
  web transport, or save-game logic in this crate.
- Application crates should need only scs-sdk-plugin in their manifest. Re-export
  the typed SDK layer deliberately; do not require direct scs-sdk-sys access.

## Application Safety Boundary

- All normal TelemetryPlugin implementations must be writable in safe Rust.
  Public plugin hooks and context methods must not expose raw pointers, CStr,
  CString, extern callback functions, sys unions, or unsafe preconditions.
- Keep initialization subscriptions explicit. Do not infer an event subscription
  from an implemented hook and do not subscribe all catalog entries automatically.
- Empty initialization means zero SDK registrations. Duplicate, malformed, or
  wrong-phase subscriptions fail before invoking the SDK.
- Keep scalar channels, SDK-indexed channels, trailer channels, and indexed
  trailer channels as distinct methods with explicit flags variants.
- Accept `SdkIndex` and `TrailerIndex` in their respective public subscription
  methods instead of bare `u32`. Expose `TrailerConfigurationId` on
  configuration callbacks so legacy `trailer` and numbered `trailer.0` remain
  observably different.
- Copy game identity and other data whose foreign lifetime ends after init. For
  callback-only payloads, preserve a borrow tied to the hook invocation.
- PluginMetadata is required product identity. Validate it before registration and
  retain stable startup, detected-game, initialization, and shutdown logs.

## Runtime Invariants

- The runtime lifecycle state machine must reject invalid init/reinit/shutdown
  transitions deterministically.
- Registration is a transaction. Defer commit until plugin initialization returns
  successfully; on failure, unregister the completed prefix in reverse order.
- Shutdown unregisters channels and events in reverse order and invokes the plugin
  shutdown hook according to the documented lifecycle contract.
- Each registration context must have a stable allocation. Store Arc handles
  without creating competing unique references to the foreign-visible pointee.
- Generation numbers isolate delayed callbacks from a prior session. A stale
  context must never dispatch into the current plugin instance.
- If SDK unregistration fails, keep the corresponding context and runtime backlink
  alive. Logical inactivity is not proof that SCS discarded the pointer.
- Catch every panic before it crosses the loader, event callback, channel callback,
  initialization, or shutdown ABI boundary.
- Recover poisoned mutex state deliberately and keep lifecycle invariants intact;
  do not replace poison recovery with unwrap or expect.
- Do not hold or construct an SDK call outside a direct game callback scope. Do
  not move callback handling to a worker thread.
- Keep runtime logging bounded and lifecycle-focused. Product probe lines belong
  in the example or product plugin, not this framework.

## Unsafe Review

- Unsafe is permitted only for the FFI/runtime work this crate owns. Keep it out
  of ordinary public application APIs.
- Every pointer dereference must identify which allocation owns the pointee, why
  it is still live, which generation it belongs to, and what synchronization
  prevents invalid concurrent access.
- Never shorten retired-context lifetime based on a successful logical state
  transition alone. Only a successful SDK unregistration releases the foreign
  reference contract.
- Any change to Runtime storage, Arc ownership, callback context casts, global
  state, rollback, or shutdown requires a focused regression test and strict
  provenance Miri.

## Public API and Compatibility

- Treat scs-sdk::TelemetryApi as the sole owner of audited API adapters and its
  supported-version list. Do not repeat that whitelist in the runtime; consume
  the wrapper result and translate it into framework diagnostics and SCS results.
- Keep wrapper capability separate from product requirements. Every product
  declares PluginCompatibility explicitly, and the runtime validates its minimum
  API and per-game schema requirements before product initialization.
- Accept later schema minors only within the product-declared major. Reject a
  lower minimum, a different major, undeclared games, duplicate declarations,
  and ambiguous Game::Other declarations with precise typed errors.
- Enforce API-gated framework capabilities using official header evidence.
  Gameplay event subscription requires API 1.01 even though the wrapper can
  safely parse the shared 1.00 initialization layout.
- Re-export scs_sdk::Event as the application-facing TelemetryEventKind name;
  do not maintain a second event enum, raw discriminator match, or capability
  table in the framework. TelemetryEvent remains separate because it carries
  validated callback payloads rather than registration intent.
- Keep required and optional intent visible in method names. Optional channel
  declarations tolerate only NotFound and UnsupportedType; optional event
  declarations tolerate only Unsupported and NotFound. Do not turn arbitrary
  SDK failures into absence, and keep required rollback unchanged.
- Check Event and ValueType minimum API metadata before SDK calls. Skip a newer
  optional capability, reject a newer required capability, and count only
  registrations which actually committed.
- Combine each descriptor's per-game schema availability with detected
  `GameInfo` before calling SCS. Required capabilities reject initialization;
  optional capabilities remain visible to duplicate auditing but are skipped
  before foreign registration. Numbered trailer names additionally require the
  official multi-trailer schema capability.
- Preserve Telemetry API and game telemetry/schema versions as separate fields.
- Route descriptor, association, and value capability queries through
  `GameInfo::minimum_schema_for` and `GameInfo::supports`; this is the canonical
  detected-game policy and must not be copied into a second match tree.
- High-level parsers such as `shifter_type`, `job_market`, and `fine_offence`
  return known catalog values only. Keep the generic `string` and
  `string_owned` accessors beside them so future unknown text remains visible.
- Retain unknown game IDs in Game::Other with their owned textual ID and name.
- Errors should preserve the SCS result and enough context for a useful game-log
  diagnostic without exposing internal pointer details.
- Public changes must remain usable through export_plugin! and the isolated
  consumer fixture. Avoid hidden paths that only work inside this workspace.

## Tests and Fixtures

- Runtime tests must cover success, empty subscriptions, duplicates, wrong phase,
  partial registration failure, reverse rollback, shutdown, failed unregistration,
  panic containment, reinitialization, and stale callbacks.
- Do not use timing assumptions for lifecycle tests. Drive fake SDK callbacks and
  registration results deterministically.
- Preserve crates/scs-sdk-plugin/tests/fixtures/export-plugin as an independent
  consumer workspace. Its pass and missing-trait packages are part of this crate's
  public compatibility contract.
- Do not weaken expected compile-fail diagnostics to accept unrelated failures.

## Validation

After changes, run at minimum:

    cargo fmt --all -- --check
    scripts/check-plugin-boundary.sh
    scripts/check-plugin-macro-fixtures.sh
    cargo test --locked -p scs-sdk-plugin
    cargo clippy --locked -p scs-sdk-plugin --all-targets -- -D warnings
    RUSTDOCFLAGS=-Dwarnings cargo doc --locked -p scs-sdk-plugin --no-deps
    MIRIFLAGS=-Zmiri-strict-provenance cargo +nightly-2026-04-12 miri test \
      --locked -p scs-sdk-plugin

For changes to exports, macro integration, crate names, or release linkage, build
and verify the independent fixture and example on Windows, Linux, and macOS.
