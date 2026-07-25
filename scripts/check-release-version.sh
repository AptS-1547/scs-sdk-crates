#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
release_tag="${1:?usage: check-release-version.sh RELEASE_TAG}"

if [[ ! "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  printf 'Release tag is not a v-prefixed semantic version: %s\n' "$release_tag" >&2
  exit 1
fi

metadata="$(cargo metadata \
  --manifest-path "$repo_root/Cargo.toml" \
  --locked \
  --no-deps \
  --format-version 1)"

public_crates=(
  scs-sdk-sys
  scs-sdk
  scs-sdk-plugin-macros
  scs-sdk-plugin
)

workspace_version="$(printf '%s\n' "$metadata" | jq -r \
  '.packages[] | select(.name == "scs-sdk-sys") | .version')"

if [[ -z "$workspace_version" || "$workspace_version" == "null" ]]; then
  printf '%s\n' 'Could not determine the public workspace version.' >&2
  exit 1
fi

expected_tag="v$workspace_version"
if [[ "$release_tag" != "$expected_tag" ]]; then
  printf 'Release tag %s does not match workspace version %s.\n' \
    "$release_tag" "$workspace_version" >&2
  exit 1
fi

# A synchronized workspace version is part of the public compatibility model.
# Verify every independently published crate instead of assuming that Cargo's
# workspace inheritance remained intact after a manifest edit.
for crate in "${public_crates[@]}"; do
  crate_version="$(printf '%s\n' "$metadata" | jq -r \
    --arg crate "$crate" \
    '.packages[] | select(.name == $crate) | .version')"
  if [[ "$crate_version" != "$workspace_version" ]]; then
    printf 'Public crate %s has version %s, expected %s.\n' \
      "$crate" "$crate_version" "$workspace_version" >&2
    exit 1
  fi

  missing_package_metadata="$(printf '%s\n' "$metadata" | jq -r \
    --arg crate "$crate" \
    '.packages[] | select(.name == $crate) |
      (.repository == null or .repository == "" or
       .homepage == null or .homepage == "" or
       .documentation == null or .documentation == "" or
       .readme == null or .readme == "")')"
  if [[ "$missing_package_metadata" != "false" ]]; then
    printf 'Public crate %s is missing release package metadata.\n' "$crate" >&2
    exit 1
  fi
done

# Path dependencies are retained for local workspace development, while the
# exact registry requirement is what survives in the normalized crate package.
# Audit every public internal edge so a version bump cannot publish a mixed set.
dependency_edges=(
  'scs-sdk:scs-sdk-sys'
  'scs-sdk-plugin:scs-sdk-sys'
  'scs-sdk-plugin:scs-sdk'
  'scs-sdk-plugin:scs-sdk-plugin-macros'
)
expected_requirement="=$workspace_version"

for edge in "${dependency_edges[@]}"; do
  package="${edge%%:*}"
  dependency="${edge#*:}"
  requirement="$(printf '%s\n' "$metadata" | jq -r \
    --arg package "$package" \
    --arg dependency "$dependency" \
    '.packages[] | select(.name == $package) |
      .dependencies[] | select(.name == $dependency) | .req')"

  if [[ "$requirement" != "$expected_requirement" ]]; then
    printf '%s -> %s requires %s, expected exact requirement %s.\n' \
      "$package" "$dependency" "$requirement" "$expected_requirement" >&2
    exit 1
  fi
done

printf 'Verified release tag %s and %d synchronized public crates.\n' \
  "$release_tag" "${#public_crates[@]}"
