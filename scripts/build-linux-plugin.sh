#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-unknown-linux-gnu"
plugin="$repo_root/target/$target/release/libets2_dispatch_telemetry_rust.so"

cargo zigbuild \
  --manifest-path "$repo_root/Cargo.toml" \
  --package ets2-dispatch-telemetry-rust \
  --target "$target.2.17" \
  --locked \
  --release

"$repo_root/scripts/verify-linux-plugin.sh" "$plugin"
printf 'Linux Rust plugin: %s\n' "$plugin"
