#!/usr/bin/env bash

set -euo pipefail

plugin_path="${1:?usage: verify-linux-plugin.sh PATH_TO_SHARED_OBJECT [EXPECTED_EXPORT ...]}"
shift
if [[ $# -eq 0 ]]; then
  expected_symbols=(scs_telemetry_init scs_telemetry_shutdown)
else
  expected_symbols=("$@")
fi

if [[ ! -f "$plugin_path" ]]; then
  printf 'Plugin does not exist: %s\n' "$plugin_path" >&2
  exit 1
fi

# `file` validates the artifact itself instead of trusting Cargo's target
# directory name. SCS loads an x86-64 ELF shared object on Linux; a relocatable
# object, executable, archive, or host-architecture Mach-O file is not a valid
# plugin even if it happens to use the expected `.so` suffix.
file_output="$(file "$plugin_path")"
if [[ "$file_output" != *"ELF 64-bit LSB shared object"* || "$file_output" != *"x86-64"* ]]; then
  printf 'Unexpected plugin format: %s\n' "$file_output" >&2
  exit 1
fi

# Prefer a target-prefixed GNU nm when cross-checking from macOS. A native GNU
# nm is sufficient on Linux, while llvm-nm is a portable explicit fallback.
# `NM=/path/to/nm` remains available for unusual toolchain layouts.
if [[ -n "${NM:-}" ]]; then
  nm_bin="$NM"
elif command -v x86_64-linux-gnu-nm >/dev/null 2>&1; then
  nm_bin="x86_64-linux-gnu-nm"
elif command -v llvm-nm >/dev/null 2>&1; then
  nm_bin="llvm-nm"
elif [[ "$(uname -s)" == "Linux" ]] && command -v nm >/dev/null 2>&1; then
  nm_bin="nm"
else
  printf '%s\n' 'No ELF-capable nm found; set NM to GNU nm or llvm-nm.' >&2
  exit 1
fi

# Only the dynamic, defined symbol table matters to the game loader. Searching
# all strings or the ordinary object symbol table could accept an export name
# that is present as debug data but unavailable through dlsym(3).
dynamic_symbols="$($nm_bin -D --defined-only "$plugin_path")"
symbol_names="$(printf '%s\n' "$dynamic_symbols" | awk '{print $NF}' | LC_ALL=C sort)"
for symbol in "${expected_symbols[@]}"; do
  if [[ $'\n'"$symbol_names"$'\n' != *$'\n'"$symbol"$'\n'* ]]; then
    printf 'Missing required dynamic export: %s\n' "$symbol" >&2
    exit 1
  fi
done


# Keep the external ABI surface closed. A presence-only check would accept an
# accidentally exported Rust or application symbol.
expected_symbol_names="$(printf '%s\n' "${expected_symbols[@]}" | LC_ALL=C sort)"
if [[ "$symbol_names" != "$expected_symbol_names" ]]; then
  printf 'Unexpected defined dynamic exports:\n%s\n' "$symbol_names" >&2
  exit 1
fi

printf 'Verified Linux x86-64 plugin with %d exports: %s\n' \
  "${#expected_symbols[@]}" "$plugin_path"
