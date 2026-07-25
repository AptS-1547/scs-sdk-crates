# Telemetry Example Instructions

This file supplements the repository-level AGENTS.md for the
scs-sdk-telemetry-example package. This package is both user-facing sample code
and a real ETS2 end-to-end probe of the framework boundary.

## Purpose and Scope

- Demonstrate how a product plugin uses only scs-sdk-plugin and ordinary safe
  Rust to declare metadata, subscribe explicitly, receive typed callbacks, keep
  product state, log telemetry, and shut down.
- Keep enough real behavior to exercise all six telemetry event kinds, typed
  channel updates, a snapshot, job configuration, gameplay events, and clean
  lifecycle transitions in ETS2.
- Do not turn this example back into ETS2 Dispatch. Bridge, protocol, web,
  dispatcher, save-game, persistence, and task-acceptance behavior belong in the
  separate product repository.
- Prefer clear representative code over generic abstraction. The example should
  teach framework usage by reading like a small plugin, not like another runtime.

## Hard Safety Boundary

- Keep every Rust source file free of unsafe code, raw pointers, extern ABI
  declarations, no_mangle attributes, CStr, CString, scs-sdk-sys dependencies,
  sys module access, and scs_sdk_plugin::__private access.
- Depend directly only on scs-sdk-plugin for SDK functionality. Use its sdk
  re-export for typed channels, configuration attributes, and gameplay values.
- Do not weaken scripts/check-plugin-boundary.sh. Add #![forbid(unsafe_code)] when
  it is compatible with the generated macro boundary and preserve the existing
  source audit in all cases.
- Do not add global mutable state or an application mutex to duplicate runtime
  serialization. Keep state inside the TelemetryPlugin value.

## Example Behavior

- Plugin metadata remains explicit and generic: SCS SDK Telemetry Example. Do not
  reintroduce ETS2 Dispatch product names, filenames, or log prefixes.
- Declare PluginCompatibility explicitly. This example requires Telemetry API
  1.01 for gameplay events and ETS2 schema 1.14 for documented gameplay support;
  keep those requirements synchronized with its subscriptions.
- Use the [scs-sdk-example] prefix for example-owned messages. Framework lifecycle
  identity remains under [scs-sdk-plugin].
- Declare every event and channel subscription in initialize. Removing a line
  should visibly remove that capability; do not infer or bulk-register behavior.
- Use `SdkIndex` for SDK array slots and `TrailerIndex` for numbered trailer
  namespaces. Do not pass a bare `u32` as an SDK or trailer index, and do not
  treat legacy `trailer` configuration callbacks as if they were `trailer.0`.
- Keep the current registration surface intentional: 6 event kinds and 8 channels
  unless an E2E coverage change justifies and documents a new count.
- Preserve the distinction between latest channel values and frame-end snapshot
  logging. Do not assume SCS callback ordering that the SDK does not guarantee.
- Preserve units in names and conversion constants. SDK speed is metres per second;
  display conversion to kilometres per hour must stay explicit.
- When demonstrating enum-like SDK strings, log or otherwise retain both the
  high-level known enum and the generic raw string. A future unknown value must
  remain diagnosable rather than disappearing behind `None`.
- Keep log volume bounded. Probe output is rate-limited by the SCS real-time
  timestamp and must handle timer restart without underflow or a stale throttle
  anchor.
- Handle missing, unknown, or temporarily unavailable navigation and job values
  without panicking. Do not fabricate product defaults that look authoritative.
- The current example intentionally accepts ETS2 and reports other games as
  unsupported. If ATS support is added, audit semantic assumptions and E2E it
  rather than merely deleting the game check.
- Reset product state on initialize and shutdown so reinitialization cannot leak a
  previous game session's snapshot.

## Documentation and Readability

- Keep comments generous and educational. Explain why subscriptions, snapshots,
  units, throttling, and lifecycle resets exist, not just what each line does.
- Application code is the public ergonomics test. If normal behavior requires a
  raw workaround, fix the owning framework/wrapper layer instead of hiding the
  workaround here.
- Keep package, library, artifact, installer, CI artifact, and README names in
  sync when this example is renamed.

## Validation

After changes, run at minimum:

    cargo fmt --all -- --check
    scripts/check-plugin-boundary.sh
    cargo test --locked -p scs-sdk-telemetry-example
    cargo clippy --locked -p scs-sdk-telemetry-example --all-targets -- -D warnings
    scripts/check-plugin-macro-fixtures.sh

For behavior, subscription, export, package, or artifact changes, build and verify
the Windows DLL, Linux shared object, and macOS dylib. For an actual installation
change, inspect the current live game.log.txt and confirm the new library name,
plugin metadata, [scs-sdk-example] prefix, 6/8 registration counts, clean shutdown,
and absence of the legacy plugin.
