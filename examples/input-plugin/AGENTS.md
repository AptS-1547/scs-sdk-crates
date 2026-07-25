# Input Example Instructions

This file supplements the repository-level AGENTS.md for the
scs-sdk-input-example package.

## Purpose

- Demonstrate the public InputPlugin API using only safe Rust and the
  scs-sdk-plugin dependency.
- Keep the device deterministic and self-contained. Real hardware, network
  protocols, product bridges, and dispatch behavior belong outside this repo.
- Register every device and optional activity callback explicitly.

## Safety Boundary

- Keep application source under #![forbid(unsafe_code)].
- Do not use raw pointers, extern ABI declarations, C strings, scs-sdk-sys,
  framework-private modules, or handwritten exports.
- Keep device identity and input indices strongly typed.

## Behavior

- Preserve one generic device with one float axis and one bool button unless an
  explicit E2E coverage change justifies more.
- Keep the float axis inside `InputAxisValue`'s finite inclusive -1.0 through
  1.0 contract. Out-of-range research belongs in preserved E2E evidence, not a
  bypass feature in the final safe example.
- Reset the per-frame event cursor only when first_in_frame is present.
- Return None after the two events so SCS receives SCS_RESULT_not_found.
- Keep logging bounded to lifecycle, activation transitions, and exactly one
  float/bool/exhaustion evidence sequence per activation.

## Validation

Run:

    cargo fmt --all -- --check
    scripts/check-plugin-boundary.sh
    cargo test --locked -p scs-sdk-input-example
    cargo clippy --locked -p scs-sdk-input-example --all-targets -- -D warnings

For export or artifact changes, run all three input-plugin build scripts.
Before release, install the macOS fixture and preserve a real ETS2 log proving
load, registration, activation, both typed events, exhaustion, shutdown, and
unload.
