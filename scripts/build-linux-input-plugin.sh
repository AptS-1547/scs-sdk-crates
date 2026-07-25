#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-unknown-linux-gnu"
plugin="$repo_root/target/$target/release/libscs_sdk_input_example.so"

cargo zigbuild \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-input-example \
  --target "$target.2.17" \
  --locked \
  --release

"$repo_root/scripts/verify-linux-plugin.sh" \
  "$plugin" scs_input_init scs_input_shutdown
printf 'Linux input plugin: %s\n' "$plugin"
