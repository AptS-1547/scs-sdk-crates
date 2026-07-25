#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
allow_dirty=false

if [[ "${1:-}" == "--allow-dirty" ]]; then
  allow_dirty=true
  shift
fi
if [[ $# -ne 0 ]]; then
  printf '%s\n' 'usage: check-crate-packages.sh [--allow-dirty]' >&2
  exit 1
fi

metadata="$(cargo metadata \
  --manifest-path "$repo_root/Cargo.toml" \
  --locked \
  --no-deps \
  --format-version 1)"
version="$(printf '%s\n' "$metadata" | jq -r \
  '.packages[] | select(.name == "scs-sdk-sys") | .version')"
# Cargo's command-line registry patches intentionally differ from the normal
# workspace graph. Give package normalization its own target directory so those
# fingerprints and cached metadata can never be reused by tests or rustdoc.
package_target_directory="$repo_root/target/release-package-check"

"$repo_root/scripts/check-release-version.sh" "v$version"

public_crates=(
  scs-sdk-sys
  scs-sdk-plugin-macros
  scs-sdk
  scs-sdk-plugin
)
required_files=(
  Cargo.toml
  Cargo.toml.orig
  README.md
  README.zh.md
  LICENSE-APACHE
  LICENSE-MIT
  LICENSE-SCS-SDK-2013
  LICENSE-SCS-SDK-2016
)

package_args=(--locked --no-verify)
if [[ "$allow_dirty" == true ]]; then
  package_args+=(--allow-dirty)
fi

for crate in "${public_crates[@]}"; do
  crate_package_args=("${package_args[@]}")
  # Bash 3.2 treats expansion of an empty array as an unbound variable under
  # `set -u`, so keep one explicit empty sentinel for dependency-free crates.
  internal_dependencies=("")

  # During the first release, dependent public crates do not exist in the
  # registry yet. Command-line patches let Cargo perform its real packaging and
  # manifest normalization against the audited workspace sources without
  # writing release-only patch policy into the repository manifest.
  case "$crate" in
    scs-sdk)
      crate_package_args+=(
        --config "patch.crates-io.scs-sdk-sys.path=\"$repo_root/crates/scs-sdk-sys\""
      )
      internal_dependencies=(scs-sdk-sys)
      ;;
    scs-sdk-plugin)
      crate_package_args+=(
        --config "patch.crates-io.scs-sdk-sys.path=\"$repo_root/crates/scs-sdk-sys\""
        --config "patch.crates-io.scs-sdk.path=\"$repo_root/crates/scs-sdk\""
        --config "patch.crates-io.scs-sdk-plugin-macros.path=\"$repo_root/crates/scs-sdk-plugin-macros\""
      )
      internal_dependencies=(scs-sdk-sys scs-sdk scs-sdk-plugin-macros)
      ;;
  esac

  CARGO_TARGET_DIR="$package_target_directory" cargo package \
    --manifest-path "$repo_root/Cargo.toml" \
    --package "$crate" \
    "${crate_package_args[@]}"

  archive="$package_target_directory/package/$crate-$version.crate"
  prefix="$crate-$version"
  if [[ ! -s "$archive" ]]; then
    printf 'Cargo package archive is missing or empty: %s\n' "$archive" >&2
    exit 1
  fi

  archive_entries="$(tar -tzf "$archive")"
  for required in "${required_files[@]}"; do
    if ! grep -Fxq "$prefix/$required" <<<"$archive_entries"; then
      printf 'Package %s is missing %s.\n' "$crate" "$required" >&2
      exit 1
    fi
  done

  if grep -Fxq "$prefix/AGENTS.md" <<<"$archive_entries"; then
    printf 'Package %s unexpectedly contains repository-only AGENTS.md.\n' \
      "$crate" >&2
    exit 1
  fi

  if ! grep -Eq "^$prefix/src/.+\.rs$" <<<"$archive_entries"; then
    printf 'Package %s contains no Rust source files.\n' "$crate" >&2
    exit 1
  fi

  normalized_manifest="$(tar -xOzf "$archive" "$prefix/Cargo.toml")"
  if [[ "$normalized_manifest" != *'repository = "https://github.com/AptS-1547/scs-sdk-crates"'* \
    || "$normalized_manifest" != *"documentation = \"https://docs.rs/$crate\""* ]]; then
    printf 'Package %s has incomplete normalized release metadata.\n' "$crate" >&2
    exit 1
  fi

  for dependency in "${internal_dependencies[@]}"; do
    if [[ -z "$dependency" ]]; then
      continue
    fi
    dependency_block="[dependencies.$dependency]"
    dependency_version="$(printf '%s\n' "$normalized_manifest" | awk \
      -v section="$dependency_block" '
        $0 == section { in_section = 1; next }
        in_section && /^\[/ { exit }
        in_section && /^version = / {
          value = $0
          sub(/^version = "/, "", value)
          sub(/"$/, "", value)
          print value
          exit
        }
      ')"
    if [[ "$dependency_version" != "=$version" ]]; then
      printf 'Package %s did not normalize %s to exact version %s.\n' \
        "$crate" "$dependency" "$version" >&2
      exit 1
    fi
  done

  printf 'Verified Cargo package: %s\n' "$archive"
done
