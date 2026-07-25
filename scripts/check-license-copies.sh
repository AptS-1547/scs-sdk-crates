#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
published_crates=(
  scs-sdk-sys
  scs-sdk
  scs-sdk-plugin
  scs-sdk-plugin-macros
)
license_files=(
  LICENSE-APACHE
  LICENSE-MIT
  LICENSE-SCS-SDK-2013
  LICENSE-SCS-SDK-2016
)

# The historical archive series contains two distinct original copyright
# notices: SDK 1.0-1.5 use the 2013 notice, while SDK 1.6-1.14 use the 2016
# notice. Preserve both exact texts instead of silently replacing the notice
# attached to the earlier headers used for schema-history research.
if ! cmp -s \
  "$repo_root/third-party/scs_sdk_history/licenses/LICENSE-SCS-SDK-2013" \
  "$repo_root/LICENSE-SCS-SDK-2013"; then
  printf '%s\n' 'LICENSE-SCS-SDK-2013 does not match the preserved historical notice.' >&2
  exit 1
fi

if ! cmp -s \
  "$repo_root/third-party/scs_sdk_history/licenses/LICENSE-SCS-SDK-2016" \
  "$repo_root/third-party/scs_sdk_1_14/sdk_license.txt"; then
  printf '%s\n' 'The preserved 2016 notice does not match the vendored SDK 1.14 license.' >&2
  exit 1
fi

if ! cmp -s \
  "$repo_root/third-party/scs_sdk_history/licenses/LICENSE-SCS-SDK-2016" \
  "$repo_root/LICENSE-SCS-SDK-2016"; then
  printf '%s\n' 'LICENSE-SCS-SDK-2016 does not match the preserved historical notice.' >&2
  exit 1
fi

# Each public crate is an independently distributed Cargo package. Cargo does
# not automatically copy workspace-root license files into a nested package,
# so every public crate keeps byte-identical release copies beside its manifest.
# The root files remain the package-copy authority; the SCS root copy is itself
# checked against the official vendored text above.
for crate in "${published_crates[@]}"; do
  crate_root="$repo_root/crates/$crate"

  for license in "${license_files[@]}"; do
    authoritative="$repo_root/$license"
    packaged="$crate_root/$license"

    if [[ ! -f "$packaged" ]]; then
      printf 'Published crate is missing %s: %s\n' "$license" "$crate_root" >&2
      exit 1
    fi

    if ! cmp -s "$authoritative" "$packaged"; then
      printf 'Published crate has a stale %s copy: %s\n' "$license" "$crate_root" >&2
      printf 'Refresh it from the repository root before packaging.\n' >&2
      exit 1
    fi
  done
done

printf 'Verified workspace and both historical SCS SDK license notices for %d public crates.\n' \
  "${#published_crates[@]}"
