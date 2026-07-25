#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
default_plugin="$repo_root/target/x86_64-apple-darwin/release/libscs_sdk_telemetry_fallback_example.dylib"
steam_common="$HOME/Library/Application Support/Steam/steamapps/common"
default_game_macos="$steam_common/Euro Truck Simulator 2/Euro Truck Simulator 2.app/Contents/MacOS"
plugin_source="${1:-$default_plugin}"
game_macos="${ETS2_MACOS_DIR:-$default_game_macos}"
plugin_dir="$game_macos/plugins"
plugin_destination="$plugin_dir/libscs_sdk_telemetry_fallback_example.dylib"
normal_example="$plugin_dir/libscs_sdk_telemetry_example.dylib"
legacy_example="$plugin_dir/libets2_dispatch_telemetry_rust.dylib"

if [[ ! -f "$plugin_source" ]]; then
  printf 'Plugin does not exist: %s\n' "$plugin_source" >&2
  printf '%s\n' 'Build it first with scripts/build-macos-fallback-plugin.sh.' >&2
  exit 1
fi

if [[ ! -x "$game_macos/eurotrucks2" ]]; then
  printf 'ETS2 macOS executable was not found: %s\n' "$game_macos/eurotrucks2" >&2
  printf '%s\n' 'Set ETS2_MACOS_DIR to the game app Contents/MacOS directory.' >&2
  exit 1
fi

# Preserve the source artifact and perform quarantine/signing work on a private
# copy. This mirrors the normal installer while keeping the two probes' names
# and cleanup rules explicit.
staging_dir="$(mktemp -d "${TMPDIR:-/tmp}/scs-sdk-fallback-install.XXXXXX")"
cleanup() {
  rm -rf "$staging_dir"
}
trap cleanup EXIT
staged_plugin="$staging_dir/libscs_sdk_telemetry_fallback_example.dylib"
cp "$plugin_source" "$staged_plugin"
xattr -d com.apple.quarantine "$staged_plugin" 2>/dev/null || true
codesign --force --sign - "$staged_plugin"

"$repo_root/scripts/verify-macos-plugin.sh" "$staged_plugin"

if ! mkdir -p "$plugin_dir" || ! cp -f "$staged_plugin" "$plugin_destination"; then
  printf '%s\n' 'Failed to write inside the ETS2 application bundle.' >&2
  printf '%s\n' 'Allow this terminal under System Settings -> Privacy & Security -> App Management, then retry.' >&2
  exit 1
fi
chmod 755 "$plugin_destination"
xattr -d com.apple.quarantine "$plugin_destination" 2>/dev/null || true
"$repo_root/scripts/verify-macos-plugin.sh" "$plugin_destination"

# The two example artifacts intentionally exercise different negotiation
# behavior. Keeping both installed would make the log ambiguous, so switch to
# the fallback probe only after its destination has passed verification.
rm -f "$normal_example" "$legacy_example"
printf 'Installed macOS telemetry fallback E2E plugin: %s\n' "$plugin_destination"
