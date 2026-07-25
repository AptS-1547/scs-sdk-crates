#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="$repo_root/crates/scs-sdk-plugin/tests/fixtures/export-plugin"
target_dir="$repo_root/target/plugin-macro-fixtures"
target="x86_64-unknown-linux-gnu"
plugin="$target_dir/$target/release/libscs_sdk_plugin_export_fixture.so"
input_plugin="$target_dir/$target/release/libscs_sdk_input_plugin_export_fixture.so"
combined_plugin="$target_dir/$target/release/libscs_sdk_combined_plugin_export_fixture.so"

# Match the real Linux plugin's glibc 2.17 floor so fixture success exercises
# the same Zig linker path, not merely a native host-only macro expansion.
cargo zigbuild \
  --manifest-path "$fixture_root/Cargo.toml" \
  --package scs-sdk-plugin-export-fixture \
  --package scs-sdk-input-plugin-export-fixture \
  --package scs-sdk-combined-plugin-export-fixture \
  --target-dir "$target_dir" \
  --target "$target.2.17" \
  --locked \
  --release

# The ELF verifier inspects the defined dynamic symbol table, proving both
# entry points are loader-visible after LTO and symbol stripping.
"$repo_root/scripts/verify-linux-plugin.sh" "$plugin"
"$repo_root/scripts/verify-linux-plugin.sh" "$input_plugin" scs_input_init scs_input_shutdown
"$repo_root/scripts/verify-linux-plugin.sh" "$combined_plugin" scs_telemetry_init scs_telemetry_shutdown scs_input_init scs_input_shutdown
printf 'Linux plugin macro fixture: %s\n' "$plugin"
