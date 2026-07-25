#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="x86_64-apple-darwin"
plugin="$repo_root/target/$target/release/libscs_sdk_input_example.dylib"

cargo build \
  --manifest-path "$repo_root/Cargo.toml" \
  --package scs-sdk-input-example \
  --target "$target" \
  --locked \
  --release

codesign --force --sign - "$plugin"

"$repo_root/scripts/verify-macos-plugin.sh" \
  "$plugin" scs_input_init scs_input_shutdown
printf 'macOS input plugin: %s\n' "$plugin"
