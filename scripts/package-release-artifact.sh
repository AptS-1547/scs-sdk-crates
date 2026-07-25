#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
platform="${1:?usage: package-release-artifact.sh PLATFORM RELEASE_TAG OUTPUT_DIRECTORY}"
release_tag="${2:?usage: package-release-artifact.sh PLATFORM RELEASE_TAG OUTPUT_DIRECTORY}"
output_directory="${3:?usage: package-release-artifact.sh PLATFORM RELEASE_TAG OUTPUT_DIRECTORY}"

"$repo_root/scripts/check-release-version.sh" "$release_tag"
mkdir -p "$output_directory"
output_directory="$(cd "$output_directory" && pwd)"

case "$platform" in
  windows)
    archive_base="scs-sdk-crates-$release_tag-windows-x86_64"
    archive_name="$archive_base.zip"
    plugins=(
      "$repo_root/target/x86_64-pc-windows-gnu/release/scs_sdk_telemetry_example.dll"
      "$repo_root/target/x86_64-pc-windows-gnu/release/scs_sdk_input_example.dll"
      "$repo_root/target/x86_64-pc-windows-gnu/release/scs_sdk_input_semantical_example.dll"
    )
    build_scripts=(
      build-windows-plugin.sh
      build-windows-input-plugin.sh
      build-windows-input-semantical-plugin.sh
    )
    ;;
  linux)
    archive_base="scs-sdk-crates-$release_tag-linux-x86_64-glibc-2.17"
    archive_name="$archive_base.tar.gz"
    plugins=(
      "$repo_root/target/x86_64-unknown-linux-gnu/release/libscs_sdk_telemetry_example.so"
      "$repo_root/target/x86_64-unknown-linux-gnu/release/libscs_sdk_input_example.so"
      "$repo_root/target/x86_64-unknown-linux-gnu/release/libscs_sdk_input_semantical_example.so"
    )
    build_scripts=(
      build-linux-plugin.sh
      build-linux-input-plugin.sh
      build-linux-input-semantical-plugin.sh
    )
    ;;
  macos)
    archive_base="scs-sdk-crates-$release_tag-macos-x86_64"
    archive_name="$archive_base.tar.gz"
    plugins=(
      "$repo_root/target/x86_64-apple-darwin/release/libscs_sdk_telemetry_example.dylib"
      "$repo_root/target/x86_64-apple-darwin/release/libscs_sdk_input_example.dylib"
      "$repo_root/target/x86_64-apple-darwin/release/libscs_sdk_input_semantical_example.dylib"
    )
    build_scripts=(
      build-macos-plugin.sh
      build-macos-input-plugin.sh
      build-macos-input-semantical-plugin.sh
    )
    ;;
  *)
    printf 'Unsupported release platform: %s\n' "$platform" >&2
    exit 1
    ;;
esac

for build_script in "${build_scripts[@]}"; do
  "$repo_root/scripts/$build_script"
done

staging_root="$(mktemp -d "${TMPDIR:-/tmp}/scs-sdk-release.XXXXXX")"
trap 'rm -rf "$staging_root"' EXIT
archive_root="$staging_root/$archive_base"
mkdir -p "$archive_root"

documentation_files=(
  README.md
  README.zh.md
  LICENSE-APACHE
  LICENSE-MIT
  LICENSE-SCS-SDK-2013
  LICENSE-SCS-SDK-2016
  THIRD_PARTY_NOTICES.md
  THIRD_PARTY_NOTICES.zh.md
)

for documentation in "${documentation_files[@]}"; do
  install -m 0644 "$repo_root/$documentation" "$archive_root/$documentation"
done
for plugin in "${plugins[@]}"; do
  if [[ ! -s "$plugin" ]]; then
    printf 'Verified plugin is missing or empty: %s\n' "$plugin" >&2
    exit 1
  fi
  install -m 0755 "$plugin" "$archive_root/$(basename "$plugin")"
done

# The archive is intentionally a complete platform bundle: one Telemetry
# example plus both alternative Input fixtures and all redistribution notices.
# Keeping a fixed file count makes accidental stale or missing assets visible.
actual_file_count="$(find "$archive_root" -type f | wc -l | tr -d ' ')"
expected_file_count=$((${#documentation_files[@]} + ${#plugins[@]}))
if [[ "$actual_file_count" -ne "$expected_file_count" ]]; then
  printf 'Release bundle has %s files, expected %s.\n' \
    "$actual_file_count" "$expected_file_count" >&2
  exit 1
fi

case "$platform" in
  windows)
    (
      cd "$staging_root"
      zip -9 -r "$archive_name" "$archive_base"
    )
    ;;
  linux | macos)
    tar -C "$staging_root" -czf "$staging_root/$archive_name" "$archive_base"
    ;;
esac

if [[ ! -s "$staging_root/$archive_name" ]]; then
  printf 'Release archive is missing or empty: %s\n' "$staging_root/$archive_name" >&2
  exit 1
fi

# Create in the fresh staging directory first, then replace the destination in
# one move. In particular, `zip` updates an existing archive in place and could
# otherwise retain stale entries from an earlier local release attempt.
mv -f "$staging_root/$archive_name" "$output_directory/$archive_name"

printf 'Packaged release archive: %s\n' "$output_directory/$archive_name"
