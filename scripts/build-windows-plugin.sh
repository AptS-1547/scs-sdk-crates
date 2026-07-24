#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-pc-windows-gnu"
plugin="$repo_root/target/$target/release/ets2_dispatch_telemetry_rust.dll"

cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package ets2-dispatch-telemetry-rust \
  --target "$target" \
  --locked \
  --release

"$repo_root/scripts/verify-windows-plugin.sh" "$plugin"
printf 'Rust plugin: %s\n' "$plugin"
