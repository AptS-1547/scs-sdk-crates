# Semantical Input Example Instructions

This file supplements the repository-level AGENTS.md for the
scs-sdk-input-semantical-example package.

## Purpose

- Provide an isolated real-game fixture for `InputDeviceType::Semantical`.
- Preserve the official SDK 1.14 sample's `light` bool input name so the event
  maps directly to the `semantical.light?0` mix in a fresh controls file.
- Keep the fixture deterministic, self-contained, and independent from the
  generic binding example.

## Safety Boundary

- Keep application source under `#![forbid(unsafe_code)]`.
- Depend only on the public `scs-sdk-plugin` application API.
- Do not use raw pointers, external ABI declarations, C strings,
  `scs-sdk-sys`, framework-private modules, or handwritten exports.

## Behavior

- Register exactly one semantical device with exactly one bool input named
  `light`.
- Keep registration explicit in `InputPlugin::initialize`; do not infer the
  device or mix from callbacks.
- Toggle the value deterministically every 60 input frames so a real ETS2 run
  can observe both states without relying on wall-clock time.
- Reset the per-frame event cursor only for `first_in_frame` requests.
- Return `None` after the one event so SCS receives `SCS_RESULT_not_found`.
- Bound logs to lifecycle, activity, the first false value, the first true
  value, and one exhaustion marker per activation.

## Validation

Run:

    cargo fmt --all -- --check
    scripts/check-plugin-boundary.sh examples/input-semantical-plugin
    cargo test --locked -p scs-sdk-input-semantical-example
    cargo clippy --locked -p scs-sdk-input-semantical-example --all-targets -- -D warnings

For export or artifact changes, run all three semantical input build scripts.
Before release, install only this Input fixture and preserve one real ETS2 log
proving load, registration, automatic activation, both bool states, exhaustion,
shutdown, and unload.
