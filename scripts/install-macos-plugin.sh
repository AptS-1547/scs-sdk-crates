#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
default_plugin="$repo_root/target/x86_64-apple-darwin/release/libets2_dispatch_telemetry_rust.dylib"
steam_common="$HOME/Library/Application Support/Steam/steamapps/common"
default_game_macos="$steam_common/Euro Truck Simulator 2/Euro Truck Simulator 2.app/Contents/MacOS"
plugin_source="${1:-$default_plugin}"
game_macos="${ETS2_MACOS_DIR:-$default_game_macos}"
plugin_dir="$game_macos/plugins"
plugin_destination="$plugin_dir/libets2_dispatch_telemetry_rust.dylib"

if [[ ! -f "$plugin_source" ]]; then
  printf 'Plugin does not exist: %s\n' "$plugin_source" >&2
  printf '%s\n' 'Build it first with scripts/build-macos-plugin.sh.' >&2
  exit 1
fi

if [[ ! -x "$game_macos/eurotrucks2" ]]; then
  printf 'ETS2 macOS executable was not found: %s\n' "$game_macos/eurotrucks2" >&2
  printf '%s\n' 'Set ETS2_MACOS_DIR to the game app Contents/MacOS directory.' >&2
  exit 1
fi

# Work on a private copy so installation never mutates a release artifact the
# user may want to checksum later. Downloaded archives can carry quarantine;
# remove it before signing because changing extended attributes after copying
# can otherwise trigger a first-load Gatekeeper assessment.
staging_dir="$(mktemp -d "${TMPDIR:-/tmp}/ets2-dispatch-install.XXXXXX")"
cleanup() {
  rm -rf "$staging_dir"
}
trap cleanup EXIT
staged_plugin="$staging_dir/libets2_dispatch_telemetry_rust.dylib"
cp "$plugin_source" "$staged_plugin"
xattr -d com.apple.quarantine "$staged_plugin" 2>/dev/null || true
codesign --force --sign - "$staged_plugin"

"$repo_root/scripts/verify-macos-plugin.sh" "$staged_plugin"

# macOS App Management protects writes inside another application's bundle. A
# terminal denied here must be enabled under Privacy & Security -> App
# Management; changing ownership or re-signing the game application is neither
# necessary nor desirable.
if ! mkdir -p "$plugin_dir" || ! cp -f "$staged_plugin" "$plugin_destination"; then
  printf '%s\n' 'Failed to write inside the ETS2 application bundle.' >&2
  printf '%s\n' 'Allow this terminal under System Settings -> Privacy & Security -> App Management, then retry.' >&2
  exit 1
fi
chmod 755 "$plugin_destination"

xattr -d com.apple.quarantine "$plugin_destination" 2>/dev/null || true
"$repo_root/scripts/verify-macos-plugin.sh" "$plugin_destination"
printf 'Installed macOS telemetry plugin: %s\n' "$plugin_destination"
