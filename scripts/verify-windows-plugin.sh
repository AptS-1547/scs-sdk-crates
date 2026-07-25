#!/usr/bin/env bash

set -euo pipefail

plugin_path="${1:?usage: verify-windows-plugin.sh PATH_TO_DLL}"
objdump_bin="${OBJDUMP:-x86_64-w64-mingw32-objdump}"

if [[ ! -f "$plugin_path" ]]; then
  printf 'Plugin does not exist: %s\n' "$plugin_path" >&2
  exit 1
fi

file_output="$(file "$plugin_path")"
if [[ "$file_output" != *"PE32+ executable (DLL)"* || "$file_output" != *"x86-64"* ]]; then
  printf 'Unexpected plugin format: %s\n' "$file_output" >&2
  exit 1
fi

export_table="$($objdump_bin -p "$plugin_path")"
for symbol in scs_telemetry_init scs_telemetry_shutdown; do
  if [[ "$export_table" != *"$symbol"* ]]; then
    printf 'Missing required export: %s\n' "$symbol" >&2
    exit 1
  fi
done

# A telemetry plugin has exactly the two loader entry points. Merely finding
# both names would miss an accidental third export from macro hygiene or an
# application-owned ABI function. PE records ordinal-only and named entries in
# separate counts, so validate both tables before accepting the artifact.
export_address_count="$({ printf '%s\n' "$export_table" | awk '
  /^[[:space:]]*Export Address Table[[:space:]]+[[:xdigit:]]+[[:space:]]*$/ {
    print $NF
    exit
  }
'; } || true)"
named_export_count="$({ printf '%s\n' "$export_table" | awk '
  /^[[:space:]]*\[Name Pointer\/Ordinal\] Table[[:space:]]+[[:xdigit:]]+[[:space:]]*$/ {
    print $NF
    exit
  }
'; } || true)"
if [[ -z "$export_address_count" || -z "$named_export_count"
  || $((16#$export_address_count)) -ne 2
  || $((16#$named_export_count)) -ne 2 ]]; then
  printf 'Unexpected PE export count: addresses=%s names=%s\n' \
    "${export_address_count:-missing}" "${named_export_count:-missing}" >&2
  exit 1
fi

printf 'Verified Windows x64 telemetry plugin: %s\n' "$plugin_path"
