#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-unknown-linux-gnu"
target_root="${CARGO_TARGET_DIR:-$repo_root/target}"
plugin="$target_root/$target/release/libscs_sdk_input_semantical_example.so"

cargo zigbuild \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-input-semantical-example \
  --target "$target.2.17" \
  --locked \
  --release

"$repo_root/scripts/verify-linux-plugin.sh" \
  "$plugin" scs_input_init scs_input_shutdown
printf 'Linux semantical input plugin: %s\n' "$plugin"
