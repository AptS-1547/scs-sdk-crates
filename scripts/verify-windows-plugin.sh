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

printf 'Verified Windows x64 telemetry plugin: %s\n' "$plugin_path"
