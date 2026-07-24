#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-apple-darwin"
plugin="$repo_root/target/$target/release/libets2_dispatch_telemetry_rust.dylib"

# Build the architecture used by the macOS game executable, not the build
# machine's native architecture. This distinction matters on Apple Silicon:
# an ordinary host build produces arm64 code, while the current ETS2 binary is
# x86-64 and therefore requires an x86-64 plugin in the same process.
cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package ets2-dispatch-telemetry-rust \
  --target "$target" \
  --locked \
  --release

# A locally built Mach-O has no code signature. ETS2 deliberately disables
# library validation for third-party plugins, but Gatekeeper can still prompt
# when an unsigned dylib is first loaded. An ad-hoc signature provides the
# kernel-verifiable code directory needed for local builds without pretending
# that the artifact has a Developer ID or notarization ticket.
codesign --force --sign - "$plugin"

"$repo_root/scripts/verify-macos-plugin.sh" "$plugin"
printf 'macOS Rust plugin: %s\n' "$plugin"
