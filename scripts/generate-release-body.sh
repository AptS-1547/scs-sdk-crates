#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
release_source_root="${RELEASE_SOURCE_ROOT:-$repo_root}"
release_tag="${1:?usage: generate-release-body.sh RELEASE_TAG OUTPUT_FILE}"
output_file="${2:?usage: generate-release-body.sh RELEASE_TAG OUTPUT_FILE}"
repository="${GITHUB_REPOSITORY:-AptS-1547/scs-sdk-crates}"

# A recovery run may execute this generator from a current default-branch
# helper checkout while publishing an older immutable tag. Validate the tag
# against that release checkout, not against the helper checkout's version.
"$release_source_root/scripts/check-release-version.sh" "$release_tag"
version="${release_tag#v}"
base_url="https://github.com/$repository/releases/download/$release_tag"
workflow_identity="${COSIGN_CERTIFICATE_IDENTITY:-https://github.com/$repository/.github/workflows/release.yml@refs/tags/$release_tag}"

changelog_file="$release_source_root/CHANGELOG.md"
if [[ ! -f "$changelog_file" && "$release_source_root" != "$repo_root" ]]; then
  # Tags published before CHANGELOG.md was introduced can still be resumed by
  # using the backfilled historical entry from the current workflow checkout.
  changelog_file="$repo_root/CHANGELOG.md"
fi

release_notes="$(mktemp "${TMPDIR:-/tmp}/scs-sdk-release-notes.XXXXXX")"
trap 'rm -f "$release_notes"' EXIT
"$repo_root/scripts/extract-release-changelog.sh" \
  "$release_tag" "$changelog_file" "$release_notes"

windows_archive="scs-sdk-crates-$release_tag-windows-x86_64.zip"
linux_archive="scs-sdk-crates-$release_tag-linux-x86_64-glibc-2.17.tar.gz"
macos_archive="scs-sdk-crates-$release_tag-macos-x86_64.tar.gz"

cat >"$output_file" <<EOF
This release publishes the four reusable Rust crates and verified x86-64 example
plugins for every SCS-supported desktop platform.

本次发布包含四个可复用 Rust crate，以及 SCS 支持的三个桌面平台上经过验证的
x86-64 example 插件。

## Changes

EOF

cat "$release_notes" >>"$output_file"

cat >>"$output_file" <<EOF

## Platform archives / 平台归档

| Platform | Compatibility | Archive |
| --- | --- | --- |
| Windows | x86-64 GNU DLL | [$windows_archive]($base_url/$windows_archive) |
| Linux | x86-64, glibc 2.17 floor | [$linux_archive]($base_url/$linux_archive) |
| macOS | x86-64, including Apple Silicon through Rosetta | [$macos_archive]($base_url/$macos_archive) |

Each archive contains the Telemetry example, Generic Input example, Semantical
Input example, both READMEs, both workspace licenses, both preserved SCS SDK
notices, and both third-party notice files.

每个归档都包含 Telemetry example、Generic Input example、Semantical Input
example、两份 README、两份 workspace license、两份保留的 SCS SDK 声明与两份
第三方声明。

The Generic and Semantical Input examples are mutually exclusive installation
fixtures. Install at most one of those two Input plugins at a time. The Telemetry
example may be installed beside the selected Input example.

Generic 与 Semantical Input example 是互斥安装 fixture，同一时间最多安装其中一个；
Telemetry example 可以与选定的 Input example 同时安装。

## Published crates / 已发布 crates

| Crate | crates.io | Documentation |
| --- | --- | --- |
| \`scs-sdk-sys\` | [v$version](https://crates.io/crates/scs-sdk-sys/$version) | [docs.rs](https://docs.rs/scs-sdk-sys/$version) |
| \`scs-sdk\` | [v$version](https://crates.io/crates/scs-sdk/$version) | [docs.rs](https://docs.rs/scs-sdk/$version) |
| \`scs-sdk-plugin-macros\` | [v$version](https://crates.io/crates/scs-sdk-plugin-macros/$version) | [docs.rs](https://docs.rs/scs-sdk-plugin-macros/$version) |
| \`scs-sdk-plugin\` | [v$version](https://crates.io/crates/scs-sdk-plugin/$version) | [docs.rs](https://docs.rs/scs-sdk-plugin/$version) |

## Integrity and provenance / 完整性与来源

Download [\`checksums.txt\`]($base_url/checksums.txt) and
[\`checksums.txt.sigstore.json\`]($base_url/checksums.txt.sigstore.json), then
verify the keyless Sigstore bundle and archive hashes:

下载以上两个校验文件后，验证 keyless Sigstore bundle 和归档哈希：

\`\`\`bash
cosign verify-blob checksums.txt \\
  --bundle checksums.txt.sigstore.json \\
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \\
  --certificate-identity "$workflow_identity"

sha256sum -c checksums.txt
\`\`\`

On macOS, \`shasum -a 256 -c checksums.txt\` can be used for the final hash
check.
EOF

printf 'Generated release body: %s\n' "$output_file"
