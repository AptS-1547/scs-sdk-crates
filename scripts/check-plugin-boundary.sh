#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plugin_source="$repo_root/apps/plugin-rust"

# Product code must stay on the safe framework side of the architecture. This
# list intentionally checks both direct FFI syntax and the common ways raw SDK
# types could leak upward. Framework crates have their own audited unsafe
# boundary and are outside the scope of this application-only check.
forbidden_rust_pattern='CStr|CString|c"|\bunsafe\b|extern[[:space:]]+"(C|system)"|no_mangle|ScsContext|ScsString|\*const|\*mut|scs_sdk_sys|::sys\b'

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

printf '%s\n' 'Verified safe Rust boundary for apps/plugin-rust.'
