#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
check_only=false

if [[ "${1:-}" == "--check-only" ]]; then
  check_only=true
  shift
fi
if [[ $# -ne 0 ]]; then
  printf '%s\n' 'usage: publish-crates.sh [--check-only]' >&2
  exit 1
fi

if [[ "$check_only" == false && -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  printf '%s\n' 'CARGO_REGISTRY_TOKEN is required to publish crates.' >&2
  exit 1
fi

metadata="$(cargo metadata \
  --manifest-path "$repo_root/Cargo.toml" \
  --locked \
  --no-deps \
  --format-version 1)"
version="$(printf '%s\n' "$metadata" | jq -r \
  '.packages[] | select(.name == "scs-sdk-sys") | .version')"
"$repo_root/scripts/check-release-version.sh" "v$version"

public_crates=(
  scs-sdk-sys
  scs-sdk
  scs-sdk-plugin-macros
  scs-sdk-plugin
)

notice() {
  if [[ "${GITHUB_ACTIONS:-false}" == "true" ]]; then
    printf '::notice::%s\n' "$1"
  else
    printf '%s\n' "$1"
  fi
}

# Return 0 when the exact version exists, 1 only for Cargo's exact not-found
# diagnostic, and 2 for registry, network, authentication, or parser failures.
# This distinction prevents a transient crates.io outage from being mistaken
# for permission to attempt a duplicate or out-of-order publication.
crate_version_status() {
  local crate="$1"
  local output
  local exit_code
  local expected="error: could not find \`$crate@$version\` in registry"

  output="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-cargo-info.XXXXXX")"
  # The workflow enables colored Cargo output globally for readable build logs.
  # `cargo info` is different: this function parses one exact diagnostic, and
  # ANSI sequences inserted by `CARGO_TERM_COLOR=always` split that text before
  # grep sees it. Override color only for this captured machine-readable query.
  if CARGO_TERM_COLOR=never cargo info \
    --registry crates-io "$crate@$version" >"$output" 2>&1; then
    rm -f "$output"
    return 0
  else
    exit_code=$?
  fi

  if grep -Fq "$expected" "$output"; then
    rm -f "$output"
    return 1
  fi

  printf 'cargo info failed for %s@%s with status %s:\n' \
    "$crate" "$version" "$exit_code" >&2
  cat "$output" >&2
  rm -f "$output"
  return 2
}

wait_until_visible() {
  local crate="$1"
  local attempt
  local status_code
  local max_attempts=36

  for ((attempt = 1; attempt <= max_attempts; attempt += 1)); do
    if crate_version_status "$crate"; then
      notice "$crate@$version is visible through the crates.io index"
      return 0
    else
      status_code=$?
    fi

    if [[ "$status_code" -ne 1 ]]; then
      return "$status_code"
    fi

    if [[ "$attempt" -lt "$max_attempts" ]]; then
      printf 'Waiting for crates.io index propagation (%s/%s): %s@%s\n' \
        "$attempt" "$max_attempts" "$crate" "$version"
      sleep 10
    fi
  done

  printf 'Timed out waiting for crates.io to expose %s@%s.\n' \
    "$crate" "$version" >&2
  return 1
}

for crate in "${public_crates[@]}"; do
  if crate_version_status "$crate"; then
    notice "$crate@$version is already published; skipping"
    continue
  else
    status_code=$?
  fi

  if [[ "$status_code" -ne 1 ]]; then
    exit "$status_code"
  fi

  if [[ "$check_only" == true ]]; then
    notice "$crate@$version is not published"
    continue
  fi

  printf 'Publishing %s@%s...\n' "$crate" "$version"
  cargo publish \
    --manifest-path "$repo_root/Cargo.toml" \
    --package "$crate" \
    --locked
  wait_until_visible "$crate"
done
