#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plugin_source="${1:-$repo_root/apps/plugin-rust}"

if [[ ! -d "$plugin_source" || ! -f "$plugin_source/Cargo.toml" ]]; then
  printf 'Plugin crate does not exist or has no Cargo.toml: %s\n' "$plugin_source" >&2
  exit 1
fi

# Product code must stay on the safe framework side of the architecture. This
# list intentionally checks both direct FFI syntax and the common ways raw SDK
# types could leak upward. The framework's doc-hidden `__private` module exists
# solely so the proc-macro expansion can reach its runtime and ABI types; using
# that path in handwritten plugin code would bypass the public safe API even if
# the source did not spell an `unsafe` block itself. Framework crates have their
# own audited unsafe boundary and are outside this application-only check.

# Collect source paths with NUL delimiters so spaces and other shell metacharacters
# in a checkout path cannot change which files are scanned. A plugin crate with no
# Rust source would make this boundary check meaningless, so treat that state as a
# configuration error instead of reporting a vacuous success.
rust_sources=()
while IFS= read -r -d '' source; do
  rust_sources+=("$source")
done < <(find "$plugin_source" -type f -name '*.rs' -print0)

if [[ ${#rust_sources[@]} -eq 0 ]]; then
  printf 'Plugin crate has no Rust source files: %s\n' "$plugin_source" >&2
  exit 1
fi

# Use a POSIX ERE word boundary rather than ripgrep's `\b`. GitHub's hosted
# runner image does not guarantee ripgrep is installed, while `grep` is part of
# every supported build environment. More importantly, distinguish grep's
# "no match" status (1) from an actual scan failure (>1); a missing or unreadable
# input must fail closed rather than being mistaken for a clean application.
forbidden_rust_pattern='CStr|CString|c"|(^|[^[:alnum:]_])unsafe([^[:alnum:]_]|$)|extern[[:space:]]+"(C|system)"|no_mangle|ScsContext|ScsString|\*const|\*mut|scs_sdk_sys|::sys([^[:alnum:]_]|$)|scs_sdk_plugin::__private'
boundary_violation=false
for source in "${rust_sources[@]}"; do
  if matches="$(grep -nE "$forbidden_rust_pattern" "$source")"; then
    while IFS= read -r match; do
      printf '%s:%s\n' "$source" "$match"
    done <<<"$matches"
    boundary_violation=true
  else
    status=$?
    if [[ $status -ne 1 ]]; then
      printf 'Failed to scan Rust source with grep: %s\n' "$source" >&2
      exit "$status"
    fi
  fi
done

if [[ "$boundary_violation" == true ]]; then
  printf '%s\n' 'Safe plugin boundary violation: raw ABI detail found in application source.' >&2
  exit 1
fi

# A dormant direct sys dependency is still architectural drift even when the
# current source has not imported it yet. The application should reach SDK
# capabilities through scs-sdk-plugin and its typed re-export only.
if matches="$(grep -nE 'scs-sdk-sys|scs_sdk_sys' "$plugin_source/Cargo.toml")"; then
  printf '%s\n' "$matches"
  printf '%s\n' 'Safe plugin boundary violation: direct scs-sdk-sys dependency found.' >&2
  exit 1
else
  status=$?
  if [[ $status -ne 1 ]]; then
    printf 'Failed to scan plugin manifest with grep: %s\n' "$plugin_source/Cargo.toml" >&2
    exit "$status"
  fi
fi

printf 'Verified safe Rust plugin source: %s\n' "$plugin_source"
