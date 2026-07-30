#!/usr/bin/env bash

set -euo pipefail

release_tag="${1:?usage: extract-release-changelog.sh RELEASE_TAG CHANGELOG_FILE OUTPUT_FILE}"
changelog_file="${2:?usage: extract-release-changelog.sh RELEASE_TAG CHANGELOG_FILE OUTPUT_FILE}"
output_file="${3:?usage: extract-release-changelog.sh RELEASE_TAG CHANGELOG_FILE OUTPUT_FILE}"

if [[ ! "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  printf 'Release tag is not a v-prefixed semantic version: %s\n' "$release_tag" >&2
  exit 1
fi

if [[ ! -f "$changelog_file" ]]; then
  printf 'Changelog file does not exist: %s\n' "$changelog_file" >&2
  exit 1
fi

temporary_output="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-release-changelog.XXXXXX")"
trap 'rm -f "$temporary_output"' EXIT

if ! awk -v heading="## [$release_tag]" '
  function is_release_heading(line, suffix) {
    if (substr(line, 1, length(heading)) != heading) {
      return 0
    }

    suffix = substr(line, length(heading) + 1)
    return suffix == "" || substr(suffix, 1, 3) == " - "
  }

  is_release_heading($0) {
    matches += 1
    capture = matches == 1
    next
  }

  capture && /^## \[/ {
    capture = 0
  }

  capture {
    if (!started && $0 ~ /^[[:space:]]*$/) {
      next
    }

    started = 1
    lines[++line_count] = $0
  }

  END {
    if (matches != 1) {
      exit 2
    }

    while (line_count > 0 && lines[line_count] ~ /^[[:space:]]*$/) {
      line_count -= 1
    }

    for (line = 1; line <= line_count; line += 1) {
      print lines[line]
    }
  }
' "$changelog_file" >"$temporary_output"; then
  printf 'Expected exactly one changelog section for %s in %s.\n' \
    "$release_tag" "$changelog_file" >&2
  exit 1
fi

if ! grep -q '[^[:space:]]' "$temporary_output"; then
  printf 'Changelog section for %s is empty.\n' "$release_tag" >&2
  exit 1
fi

mv "$temporary_output" "$output_file"
trap - EXIT

printf 'Extracted changelog section for %s to %s.\n' \
  "$release_tag" "$output_file"
