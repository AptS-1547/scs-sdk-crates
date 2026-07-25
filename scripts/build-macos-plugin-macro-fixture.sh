#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="$repo_root/crates/scs-sdk-plugin/tests/fixtures/export-plugin"
target_dir="$repo_root/target/plugin-macro-fixtures"
target="x86_64-apple-darwin"
plugin="$target_dir/$target/release/libscs_sdk_plugin_export_fixture.dylib"
input_plugin="$target_dir/$target/release/libscs_sdk_input_plugin_export_fixture.dylib"
combined_plugin="$target_dir/$target/release/libscs_sdk_combined_plugin_export_fixture.dylib"

# Build the isolated safe-Rust consumer as a real Mach-O cdylib. This proves
# that the public macro expansion links on macOS independently of the product
# crate and without relying on dependencies from the root workspace.
cargo build \
  --manifest-path "$fixture_root/Cargo.toml" \
  --package scs-sdk-plugin-export-fixture \
  --package scs-sdk-input-plugin-export-fixture \
  --package scs-sdk-combined-plugin-export-fixture \
  --target-dir "$target_dir" \
  --target "$target" \
  --locked \
  --release

# Match the product artifact's local signing path. This keeps verification of
# the isolated consumer honest: the exact post-signing Mach-O presented to the
# loader must retain both proc-macro-generated exports.
codesign --force --sign - "$plugin"
codesign --force --sign - "$input_plugin"
codesign --force --sign - "$combined_plugin"

# Apply the same architecture and exported-symbol contract as the product
# artifact. Passing a compile-only fixture would miss Mach-O export regressions
# introduced by LTO, stripping, or proc-macro changes.
"$repo_root/scripts/verify-macos-plugin.sh" "$plugin"
"$repo_root/scripts/verify-macos-plugin.sh" "$input_plugin" scs_input_init scs_input_shutdown
"$repo_root/scripts/verify-macos-plugin.sh" "$combined_plugin" scs_telemetry_init scs_telemetry_shutdown scs_input_init scs_input_shutdown
printf 'macOS plugin macro fixture: %s\n' "$plugin"
