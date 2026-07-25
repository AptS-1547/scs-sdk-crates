#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-apple-darwin"
plugin="$repo_root/target/$target/release/libscs_sdk_telemetry_fallback_example.dylib"

# This is a manual loader-negotiation fixture for the x86-64 macOS game. It is
# intentionally a separate artifact from the general telemetry example so an
# ordinary E2E never rejects the loader's newest supported API.
cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-telemetry-fallback-example \
  --target "$target" \
  --locked \
  --release

codesign --force --sign - "$plugin"

"$repo_root/scripts/verify-macos-plugin.sh" "$plugin"
printf 'macOS telemetry fallback E2E plugin: %s\n' "$plugin"
