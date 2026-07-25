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
)

# Each public crate is an independently distributed Cargo package. Cargo does
# not automatically copy workspace-root license files into a nested package,
# so every publishable crate keeps byte-identical release copies beside its
# manifest. The root files remain authoritative; this check prevents the
# package copies from silently drifting when either license text is updated.
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

printf 'Verified license copies for %d published crates.\n' "${#published_crates[@]}"
