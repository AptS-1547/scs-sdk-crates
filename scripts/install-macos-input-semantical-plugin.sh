#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_root="${CARGO_TARGET_DIR:-$repo_root/target}"
default_plugin="$target_root/x86_64-apple-darwin/release/libscs_sdk_input_semantical_example.dylib"
steam_common="$HOME/Library/Application Support/Steam/steamapps/common"
default_game_macos="$steam_common/Euro Truck Simulator 2/Euro Truck Simulator 2.app/Contents/MacOS"
plugin_source="${1:-$default_plugin}"
game_macos="${ETS2_MACOS_DIR:-$default_game_macos}"
plugin_dir="$game_macos/plugins"
plugin_destination="$plugin_dir/libscs_sdk_input_semantical_example.dylib"
generic_destination="$plugin_dir/libscs_sdk_input_example.dylib"

if [[ ! -f "$plugin_source" ]]; then
  printf 'Plugin does not exist: %s\n' "$plugin_source" >&2
  printf '%s\n' 'Build it first with scripts/build-macos-input-semantical-plugin.sh.' >&2
  exit 1
fi

if [[ ! -x "$game_macos/eurotrucks2" ]]; then
  printf 'ETS2 macOS executable was not found: %s\n' "$game_macos/eurotrucks2" >&2
  printf '%s\n' 'Set ETS2_MACOS_DIR to the game app Contents/MacOS directory.' >&2
  exit 1
fi

# Keep the release artifact immutable. Quarantine removal and signing happen on
# a private copy so the build checksum continues to identify the release bytes.
staging_dir="$(mktemp -d "${TMPDIR:-/tmp}/scs-sdk-input-semantical-install.XXXXXX")"
cleanup() {
  rm -rf "$staging_dir"
}
trap cleanup EXIT
staged_plugin="$staging_dir/libscs_sdk_input_semantical_example.dylib"
cp "$plugin_source" "$staged_plugin"
xattr -d com.apple.quarantine "$staged_plugin" 2>/dev/null || true
codesign --force --sign - "$staged_plugin"

"$repo_root/scripts/verify-macos-plugin.sh" \
  "$staged_plugin" scs_input_init scs_input_shutdown

# Generic and semantical fixtures export the same Input loader entry points.
# Install and verify the selected artifact before removing only the exact known
# generic fixture. Telemetry plugins remain untouched.
if ! mkdir -p "$plugin_dir" || ! cp -f "$staged_plugin" "$plugin_destination"; then
  printf '%s\n' 'Failed to write inside the ETS2 application bundle.' >&2
  printf '%s\n' 'Allow this terminal under System Settings -> Privacy & Security -> App Management, then retry.' >&2
  exit 1
fi
chmod 755 "$plugin_destination"
xattr -d com.apple.quarantine "$plugin_destination" 2>/dev/null || true

"$repo_root/scripts/verify-macos-plugin.sh" \
  "$plugin_destination" scs_input_init scs_input_shutdown
rm -f "$generic_destination"
printf 'Installed macOS semantical input E2E plugin: %s\n' "$plugin_destination"
