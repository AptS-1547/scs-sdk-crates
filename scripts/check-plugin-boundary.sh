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
forbidden_rust_pattern='CStr|CString|c"|\bunsafe\b|extern[[:space:]]+"(C|system)"|no_mangle|ScsContext|ScsString|\*const|\*mut|scs_sdk_sys|::sys\b|scs_sdk_plugin::__private'

if rg -n --glob '*.rs' "$forbidden_rust_pattern" "$plugin_source"; then
  printf '%s\n' 'Safe plugin boundary violation: raw ABI detail found in application source.' >&2
  exit 1
fi

# A dormant direct sys dependency is still architectural drift even when the
# current source has not imported it yet. The application should reach SDK
# capabilities through scs-sdk-plugin and its typed re-export only.
if rg -n 'scs-sdk-sys|scs_sdk_sys' "$plugin_source/Cargo.toml"; then
  printf '%s\n' 'Safe plugin boundary violation: direct scs-sdk-sys dependency found.' >&2
  exit 1
fi

printf 'Verified safe Rust plugin source: %s\n' "$plugin_source"
