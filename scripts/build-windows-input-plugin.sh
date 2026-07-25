#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-pc-windows-gnu"
plugin="$repo_root/target/$target/release/scs_sdk_input_example.dll"

cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-input-example \
  --target "$target" \
  --locked \
  --release

"$repo_root/scripts/verify-windows-plugin.sh" \
  "$plugin" scs_input_init scs_input_shutdown
printf 'Windows input plugin: %s\n' "$plugin"
