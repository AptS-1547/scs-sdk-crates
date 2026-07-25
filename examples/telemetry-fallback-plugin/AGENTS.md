# Telemetry Fallback E2E Example Instructions

This file supplements the repository-level AGENTS.md for the
scs-sdk-telemetry-fallback-example package. This package is a deliberately
specialized real-ETS loader negotiation probe, not a normal product example.

## Purpose

- Exercise the official loader rule which retries older Telemetry API versions
  only after `scs_telemetry_init` returns `SCS_RESULT_unsupported`.
- Reject API 1.01 intentionally, accept exactly API 1.00, register only
  1.00-compatible capabilities, and emit unambiguous game-log evidence for both
  attempts and the accepted callback session.
- Keep this policy isolated from examples/telemetry-plugin. The normal example
  should continue accepting the newest audited API and exercising the complete
  SDK 1.14 telemetry surface.
- Remain a manual E2E fixture. Do not turn intentional version rejection into a
  default framework policy or a recommendation for product plugins.

## Safety and Dependencies

- Keep handwritten source under `#![forbid(unsafe_code)]` and free of raw
  pointers, external ABI declarations, C string types, scs-sdk-sys, `::sys`, and
  `scs_sdk_plugin::__private` access.
- Depend directly only on scs-sdk-plugin and use its `sdk` re-export.
- Use `PluginContext::telemetry_api_version` and `PluginError` for negotiation;
  ABI inspection and custom exported functions do not belong in this example.

## Behavior Contract

- `ACCEPTED_FALLBACK_API` remains `TelemetryApiVersion::V1_00` while SDK 1.14's
  current API is 1.01. Changing either side requires a new real-game test plan.
- The compatibility minimum remains API 1.00 so the framework admits the 1.00
  retry. The initialize hook owns the intentional exact-version rejection.
- Return `SdkError::Unsupported` for every attempt other than 1.00. A different
  result stops the loader retry sequence according to the official header.
- Subscribe only API 1.00 capabilities. Gameplay events and signed 64-bit value
  requests would invalidate the accepted fallback session.
- Keep the `[scs-sdk-fallback-example]` markers stable. Manual validation relies
  on their order and embedded API values. The callback confirmation must be
  one-shot and require both a frame-end event and a decoded `truck.speed` value;
  registration counts alone are not channel-delivery evidence.
- Install this artifact by itself. The fallback installer removes only the exact
  normal-example and legacy-example filenames after verifying the new artifact.

## Validation

Run at minimum:

    cargo fmt --all -- --check
    scripts/check-plugin-boundary.sh
    cargo test --locked -p scs-sdk-telemetry-fallback-example
    cargo clippy --locked -p scs-sdk-telemetry-fallback-example --all-targets -- -D warnings
    scripts/build-macos-fallback-plugin.sh

For the real-game E2E, confirm that game.log.txt contains, in order:

1. rejection of Telemetry API 1.01 with `result=unsupported`;
2. cleanup of the rejected attempt;
3. acceptance of Telemetry API 1.00;
4. framework initialization with two events and one channel;
5. one callback confirmation proving both frame-end and `truck.speed` delivery
   under API 1.00;
6. clean fallback-session shutdown.
