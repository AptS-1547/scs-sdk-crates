#!/usr/bin/env bash

set -euo pipefail

wait_for_release=false
if [[ "${1:-}" == "--wait" ]]; then
  wait_for_release=true
  shift
fi
if [[ $# -ne 2 ]]; then
  printf '%s\n' \
    'usage: find-github-release.sh [--wait] OWNER/REPOSITORY RELEASE_TAG' >&2
  exit 1
fi

repository="$1"
release_tag="$2"

all_releases="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-github-releases.XXXXXX")"
matching_releases="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-matching-releases.XXXXXX")"
trap 'rm -f "$all_releases" "$matching_releases"' EXIT

max_attempts=1
if [[ "$wait_for_release" == true ]]; then
  # GitHub may accept a new draft and its assets before that draft appears in
  # the paginated release collection. Bound the recovery wait to one minute.
  max_attempts=30
fi

for ((attempt = 1; attempt <= max_attempts; attempt += 1)); do
  # GitHub's release-by-tag endpoint does not expose draft releases. Enumerate
  # every page of the release collection instead, then select the exact tag
  # locally so recovery runs can find and refresh their existing draft.
  gh api --paginate "repos/$repository/releases?per_page=100" \
    | jq -s 'add' >"$all_releases"
  jq --arg release_tag "$release_tag" \
    '[.[] | select(.tag_name == $release_tag)]' \
    "$all_releases" >"$matching_releases"

  match_count="$(jq 'length' "$matching_releases")"
  case "$match_count" in
    0)
      if [[ "$attempt" -lt "$max_attempts" ]]; then
        printf 'Waiting for GitHub to expose draft %s (%s/%s).\n' \
          "$release_tag" "$attempt" "$max_attempts" >&2
        sleep 2
        continue
      fi

      # Callers distinguish an absent release from API, authentication, and
      # JSON failures. Exit 3 is reserved for this expected condition.
      exit 3
      ;;
    1)
      jq '.[0]' "$matching_releases"
      exit 0
      ;;
    *)
      printf 'Found %s GitHub releases for tag %s; expected at most one.\n' \
        "$match_count" "$release_tag" >&2
      jq -r '.[] | "release id=\(.id) draft=\(.draft) url=\(.html_url)"' \
        "$matching_releases" >&2
      exit 2
      ;;
  esac
done
