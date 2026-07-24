#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="$repo_root/crates/scs-sdk-plugin/tests/fixtures/export-plugin"
target_dir="$repo_root/target/plugin-macro-fixtures"
target="x86_64-pc-windows-gnu"
plugin="$target_dir/$target/release/scs_sdk_plugin_export_fixture.dll"

# Build the isolated consumer crate as an actual cdylib. This separately proves
# that the public macro expansion survives Windows linking and that no root
# workspace dependency is needed by the application fixture.
cargo build \
  --manifest-path "$fixture_root/Cargo.toml" \
  --package scs-sdk-plugin-export-fixture \
  --target-dir "$target_dir" \
  --target "$target" \
  --locked \
  --release

# Reuse the production verifier: the fixture only passes when the PE dynamic
# export table contains both loader entry points with their unmangled names.
"$repo_root/scripts/verify-windows-plugin.sh" "$plugin"
printf 'Windows plugin macro fixture: %s\n' "$plugin"
