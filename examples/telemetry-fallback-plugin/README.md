# SCS Telemetry Loader Fallback E2E Probe

[中文](README.zh.md) | **English**

This crate is a deliberately specialized real-ETS fixture for the SCS
telemetry loader's API-version fallback rule. It is not a normal compatibility
strategy and not a product plugin.

The probe intentionally rejects Telemetry API 1.01 with `Unsupported`, accepts
exactly API 1.00, and then proves that the accepted 1.00 session can deliver
both an event callback and a typed channel callback through the safe plugin
framework.

## Contract under test

The official loader tries API versions from newest to oldest and continues only
when `scs_telemetry_init` returns `SCS_RESULT_unsupported`.

For SDK 1.14's two audited telemetry APIs, this fixture expects:

| Attempt | Product result | Expected loader behavior |
| --- | --- | --- |
| Telemetry API 1.01 | `SdkError::Unsupported` | Clean the rejected attempt and retry an older API. |
| Telemetry API 1.00 | `Ok(())` | Commit registrations and keep the plugin active. |

The complete sequence is:

```text
API 1.01
  -> product initialize
  -> SCS_RESULT_unsupported
  -> attempt-local product shutdown
  -> runtime returns to retryable state
API 1.00
  -> product initialize
  -> two events and one channel committed
  -> SCS_RESULT_ok
```

The plugin declares API 1.00 as its compatibility minimum. That declaration is
intentional: both the initial 1.01 attempt and the eventual 1.00 retry must
reach product initialization, where the probe applies its exact-version policy.

## Accepted API 1.00 surface

The successful attempt registers only API-1.00-compatible capabilities:

| Kind | Capability | Evidence |
| --- | --- | --- |
| Event | `Started` | Logs each accepted session start. |
| Event | `FrameEnd` | Sets the event side of the strict callback proof. |
| Channel | `truck.speed` | Decodes a real `f32` value and sets the channel side of the strict callback proof. |

The initialized framework line must report:

```text
events=2 channels=1
```

Gameplay events and signed 64-bit values are deliberately absent because those
representations require Telemetry API 1.01.

## Strict callback proof

Registration success does not by itself prove that the game delivered a
callback. The probe therefore records two observations independently:

```text
frame_end_seen == true
latest_speed_metres_per_second.is_some()
```

It emits the confirmation exactly once, after both conditions are true:

```text
[scs-sdk-fallback-example] fallback callbacks confirmed \
telemetry_api=1.0 frame_end_seen=true speed_metres_per_second=0
```

SCS does not promise whether the changed channel value or `FrameEnd` arrives
first. The probe checks readiness after both callback paths, so either order is
valid. A stopped-truck value of `0.0` is still real channel-delivery evidence;
only `None` means that no speed value has been observed yet.

Very small signed values near zero may be produced by floating-point or physics
state. For example, `-0.000000631284 m/s` is about
`-0.00000227 km/h`; it proves value delivery but is not meaningful evidence of
reverse driving.

## Expected real-game log

The live ETS2 log is:

```text
~/Library/Application Support/Euro Truck Simulator 2/game.log.txt
```

A successful run contains this sequence.

### 1. Correct library loaded

```text
loading 'libscs_sdk_telemetry_fallback_example' '.../libscs_sdk_telemetry_fallback_example.dylib'
```

### 2. API 1.01 rejected deliberately

```text
[scs-sdk-plugin] detected ... telemetry_api=1.1 telemetry_schema=...
[scs-sdk-fallback-example] requesting loader retry \
rejected_telemetry_api=1.1 accepted_telemetry_api=1.0 result=unsupported
<ERROR> plugin initialization failed: fallback E2E intentionally rejects telemetry API 1.1; retry 1.0
[scs-sdk-fallback-example] rejected attempt cleaned telemetry_api=1.1
```

The initialization-error line is expected evidence. A different result would
stop the official loader's retry sequence.

### 3. API 1.00 accepted

```text
[scs-sdk-plugin] detected ... telemetry_api=1.0 telemetry_schema=...
[scs-sdk-fallback-example] accepted loader fallback \
telemetry_api=1.0 expected_telemetry_api=1.0
[scs-sdk-plugin] initialized ... events=2 channels=1
```

The game telemetry schema remains its own version domain. A modern ETS2 build
may use API 1.00 while still reporting a newer schema such as 1.19.

### 4. Both callback paths delivered

```text
[scs-sdk-fallback-example] fallback session started telemetry_api=1.0
[scs-sdk-fallback-example] fallback callbacks confirmed \
telemetry_api=1.0 frame_end_seen=true speed_metres_per_second=...
```

### 5. Clean shutdown

```text
[scs-sdk-fallback-example] fallback session shutdown \
telemetry_api=1.0 callbacks_confirmed=true
[scs-sdk-plugin] shutdown complete \
plugin name="SCS SDK Telemetry Fallback E2E" version="..."
unloaded 'libscs_sdk_telemetry_fallback_example'
```

## Safety boundary

All handwritten source in this crate is safe Rust and carries:

```rust
#![forbid(unsafe_code)]
```

It depends directly only on `scs-sdk-plugin` and reaches SDK types through the
framework's public `sdk` re-export. It contains no raw pointers, handwritten ABI
exports, C string handling, direct sys-crate access, or macro-private access.

Verify that boundary from the repository root:

```bash
scripts/check-plugin-boundary.sh examples/telemetry-fallback-plugin
```

## Build and verify on macOS

Run from the repository root:

```bash
scripts/build-macos-fallback-plugin.sh
```

Artifact:

```text
target/x86_64-apple-darwin/release/libscs_sdk_telemetry_fallback_example.dylib
```

The script:

1. builds the `x86_64-apple-darwin` release cdylib;
2. applies an ad-hoc signature;
3. verifies the Mach-O shared-library format;
4. verifies the x86-64 architecture;
5. verifies the embedded signature; and
6. verifies that the external export set is exactly
   `_scs_telemetry_init` and `_scs_telemetry_shutdown`.

The target remains x86-64 on Apple Silicon because the current macOS ETS2
process loads x86-64 plugins through Rosetta.

## Install on macOS

Exit ETS2 completely, then run:

```bash
scripts/install-macos-fallback-plugin.sh
```

The installer verifies the new artifact and installed destination before
removing only these exact alternate filenames:

```text
libscs_sdk_telemetry_example.dylib
libets2_dispatch_telemetry_rust.dylib
```

The normal and fallback examples are mutually exclusive. Keeping only one
probe installed gives the game log one unambiguous lifecycle and negotiation
sequence.

Return to the normal example with:

```bash
scripts/install-macos-plugin.sh
```

See [`../telemetry-plugin/README.md`](../telemetry-plugin/README.md) for the
normal six-event/eight-channel example.

## Development checks

At minimum, run:

```bash
cargo fmt --all -- --check
scripts/check-plugin-boundary.sh examples/telemetry-fallback-plugin
cargo test --locked -p scs-sdk-telemetry-fallback-example
cargo clippy --locked -p scs-sdk-telemetry-fallback-example --all-targets -- -D warnings
scripts/build-macos-fallback-plugin.sh
```

The unit tests cover both the exact API-1.00 acceptance policy and the strict,
one-shot dual-callback readiness condition. Real ETS2 testing remains necessary
for loader negotiation and callback delivery evidence.

## Non-goals

This fixture should not grow into:

- a recommendation to reject the loader's newest audited API in normal plugins;
- a product compatibility policy;
- a bridge, network service, dispatcher, or persistence layer; or
- a replacement for the normal telemetry example.

Its job is narrow: prove the official loader fallback sequence and the accepted
API-1.00 callback paths.

## License

Workspace-authored Rust code is licensed under **MIT OR Apache-2.0**. The
official SCS SDK files under `third-party/` remain subject to SCS's own license.
