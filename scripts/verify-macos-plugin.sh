#!/usr/bin/env bash

set -euo pipefail

plugin_path="${1:?usage: verify-macos-plugin.sh PATH_TO_DYLIB [EXPECTED_EXPORT ...]}"
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

# The currently shipped macOS ETS2 executable is x86-64, including on Apple
# Silicon hosts where Steam starts it through Rosetta. Verify the actual Mach-O
# slice instead of accepting a host-native arm64 library merely because Cargo
# placed it below a directory with a plausible name.
file_output="$(file "$plugin_path")"
if [[ "$file_output" != *"Mach-O"*
  || "$file_output" != *"64-bit dynamically linked shared library"*
  || "$file_output" != *"x86_64"* ]]; then
  printf 'Unexpected plugin format: %s\n' "$file_output" >&2
  exit 1
fi

# `codesign --verify` checks the embedded code directory against the final
# bytes. The build scripts use an ad-hoc identity for local/CI artifacts; a
# future release pipeline may replace it with Developer ID signing and
# notarization without changing this verifier.
if ! codesign --verify --strict --verbose=2 "$plugin_path"; then
  printf 'Plugin has no valid code signature: %s\n' "$plugin_path" >&2
  exit 1
fi

# Apple nm prints Mach-O C symbols with their leading object-file underscore.
# `-g` limits the result to external symbols, `-U` removes undefined imports,
# and `-j` prints only names. Together they check the symbols the game loader
# can resolve rather than accepting names which occur only in strings or debug
# information. `NM` remains configurable for nonstandard Xcode installations.
nm_bin="${NM:-nm}"
defined_external_symbols="$($nm_bin -gjU "$plugin_path" | LC_ALL=C sort)"
expected_external_symbols_array=()
for symbol in "${expected_symbols[@]}"; do
  symbol="_$symbol"
  expected_external_symbols_array+=("$symbol")
  if [[ $'\n'"$defined_external_symbols"$'\n' != *$'\n'"$symbol"$'\n'* ]]; then
    printf 'Missing required external export: %s\n' "${symbol#_}" >&2
    exit 1
  fi
done


# The leading underscore is Mach-O's C-symbol spelling. The expected set still
# remains closed after translating every caller-supplied C symbol.
expected_external_symbols="$(
  printf '%s\n' "${expected_external_symbols_array[@]}" | LC_ALL=C sort
)"
if [[ "$defined_external_symbols" != "$expected_external_symbols" ]]; then
  printf 'Unexpected defined external exports:\n%s\n' "$defined_external_symbols" >&2
  exit 1
fi

printf 'Verified macOS x86-64 plugin with %d exports: %s\n' \
  "${#expected_symbols[@]}" "$plugin_path"
