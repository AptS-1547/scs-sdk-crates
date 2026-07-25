#!/usr/bin/env bash

set -euo pipefail

asset_directory="${1:?usage: verify-release-assets.sh ASSET_DIRECTORY RELEASE_TAG archives|complete}"
release_tag="${2:?usage: verify-release-assets.sh ASSET_DIRECTORY RELEASE_TAG archives|complete}"
mode="${3:?usage: verify-release-assets.sh ASSET_DIRECTORY RELEASE_TAG archives|complete}"

expected=(
  "scs-sdk-crates-$release_tag-linux-x86_64-glibc-2.17.tar.gz"
  "scs-sdk-crates-$release_tag-macos-x86_64.tar.gz"
  "scs-sdk-crates-$release_tag-windows-x86_64.zip"
)

case "$mode" in
  archives)
    ;;
  complete)
    expected+=(
      checksums.txt
      checksums.txt.pem
      checksums.txt.sig
    )
    ;;
  *)
    printf 'Unknown release asset verification mode: %s\n' "$mode" >&2
    exit 1
    ;;
esac

expected_list="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-expected-assets.XXXXXX")"
actual_list="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-actual-assets.XXXXXX")"
trap 'rm -f "$expected_list" "$actual_list"' EXIT

printf '%s\n' "${expected[@]}" | LC_ALL=C sort >"$expected_list"
find "$asset_directory" -maxdepth 1 -type f -exec basename {} \; \
  | LC_ALL=C sort >"$actual_list"

if ! diff -u "$expected_list" "$actual_list"; then
  printf 'Release asset set does not match mode %s.\n' "$mode" >&2
  exit 1
fi

for asset in "${expected[@]}"; do
  if [[ ! -s "$asset_directory/$asset" ]]; then
    printf 'Release asset is missing or empty: %s\n' "$asset" >&2
    exit 1
  fi
done

printf 'Verified %d release assets in %s mode.\n' "${#expected[@]}" "$mode"
