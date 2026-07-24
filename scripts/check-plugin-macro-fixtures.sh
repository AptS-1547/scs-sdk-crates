#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="$repo_root/crates/scs-sdk-plugin/tests/fixtures/export-plugin"
manifest="$fixture_root/Cargo.toml"
target_dir="$repo_root/target/plugin-macro-fixtures/check"
diagnostics="$target_dir/missing-trait.stderr"

# The fixture workspace is intentionally excluded from the repository's main
# workspace, so root-level fmt/clippy/test commands do not discover it. Run its
# own formatter explicitly before any compilation checks.
cargo fmt \
  --manifest-path "$manifest" \
  --all \
  -- \
  --check

# Both sources model application crates and therefore share the same source
# boundary as examples/telemetry-plugin. The failing fixture may omit the
# lifecycle trait implementation, but it still has no reason to mention raw ABI
# syntax, C strings, pointers, or scs-sdk-sys.
"$repo_root/scripts/check-plugin-boundary.sh" "$fixture_root/pass"
"$repo_root/scripts/check-plugin-boundary.sh" "$fixture_root/missing-trait"

# Compile and lint the successful consumer in isolation. `--locked` makes its
# dedicated Cargo.lock part of the fixture contract rather than silently
# resolving a different syn/quote/framework graph in CI.
cargo check \
  --manifest-path "$manifest" \
  --package scs-sdk-plugin-export-fixture \
  --target-dir "$target_dir" \
  --locked

cargo clippy \
  --manifest-path "$manifest" \
  --package scs-sdk-plugin-export-fixture \
  --target-dir "$target_dir" \
  --locked \
  --all-targets \
  -- \
  -D warnings

mkdir -p "$target_dir"

# A compile-fail fixture is only useful when it fails for the intended reason.
# Capture color-free diagnostics and require both E0277 and the missing
# TelemetryPlugin bound; a syntax error, missing dependency, or broken path must
# not accidentally count as a passing negative test.
if CARGO_TERM_COLOR=never cargo check \
  --manifest-path "$manifest" \
  --package scs-sdk-plugin-missing-trait-fixture \
  --target-dir "$target_dir" \
  --locked \
  >"$diagnostics" 2>&1; then
  printf '%s\n' 'Missing-trait fixture unexpectedly compiled successfully.' >&2
  exit 1
fi

# Check each required diagnostic independently and distinguish a missing match
# from a grep execution error. This keeps the negative fixture self-contained on
# a clean hosted runner and prevents a missing search tool from being reported as
# a Rust type-checking regression.
require_diagnostic() {
  local pattern="$1"
  local status

  if grep -Eq "$pattern" "$diagnostics"; then
    return
  else
    status=$?
  fi

  if [[ $status -eq 1 ]]; then
    printf '%s\n' 'Missing-trait fixture failed for an unexpected reason:' >&2
  else
    printf 'Failed to inspect fixture diagnostics with grep (status %d).\n' "$status" >&2
  fi
  cat "$diagnostics" >&2
  exit 1
}

require_diagnostic 'error\[E0277\]'
require_diagnostic 'trait bound .*TelemetryPlugin.*not satisfied'

printf '%s\n' 'Verified plugin macro compile-pass and missing-trait fixtures.'
