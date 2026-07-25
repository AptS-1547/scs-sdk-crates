#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-pc-windows-gnu"
target_root="${CARGO_TARGET_DIR:-$repo_root/target}"
plugin="$target_root/$target/release/scs_sdk_input_semantical_example.dll"

cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-input-semantical-example \
  --target "$target" \
  --locked \
  --release

"$repo_root/scripts/verify-windows-plugin.sh" \
  "$plugin" scs_input_init scs_input_shutdown
printf 'Windows semantical input plugin: %s\n' "$plugin"
