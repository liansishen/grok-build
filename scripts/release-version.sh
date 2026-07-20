#!/usr/bin/env bash
# Validate a fork release tag and print the SemVer without the leading v.
# Expected format: v<official-version>-fork.<positive integer>

set -Eeuo pipefail

tag="${1:-${GITHUB_REF_NAME:-}}"
if [[ -z "$tag" ]]; then
    echo "usage: $0 v<official-version>-fork.<number>" >&2
    exit 2
fi

if [[ ! "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)-fork\.([1-9][0-9]*)$ ]]; then
    echo "invalid fork release tag: $tag" >&2
    echo "expected: v<official-version>-fork.<positive integer> (example: v0.2.106-fork.1)" >&2
    exit 1
fi

official_version="${BASH_REMATCH[1]}"
fork_number="${BASH_REMATCH[2]}"
manifest="crates/codegen/xai-grok-version/Cargo.toml"
manifest_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$manifest" | head -n1)"

if [[ "$official_version" != "$manifest_version" ]]; then
    echo "tag official version $official_version does not match $manifest version $manifest_version" >&2
    exit 1
fi

printf '%s-fork.%s\n' "$official_version" "$fork_number"
