#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 X.Y.Z" >&2
    exit 1
fi

version="$1"
tag="v$version"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "version must look like X.Y.Z (got: $version)" >&2
    exit 1
fi

current_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
IFS=. read -r cur_major cur_minor cur_patch <<< "$current_version"
IFS=. read -r new_major new_minor new_patch <<< "$version"

if [[ "$new_major" -eq $((cur_major + 1)) && "$new_minor" -eq 0 && "$new_patch" -eq 0 ]]; then
    :
elif [[ "$new_major" -eq "$cur_major" && "$new_minor" -eq $((cur_minor + 1)) && "$new_patch" -eq 0 ]]; then
    :
elif [[ "$new_major" -eq "$cur_major" && "$new_minor" -eq "$cur_minor" && "$new_patch" -eq $((cur_patch + 1)) ]]; then
    :
else
    echo "invalid version bump: $current_version -> $version" >&2
    echo "from $current_version, only these are allowed:" >&2
    echo "  $cur_major.$cur_minor.$((cur_patch + 1))  (patch)" >&2
    echo "  $cur_major.$((cur_minor + 1)).0  (minor)" >&2
    echo "  $((cur_major + 1)).0.0  (major)" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "working tree is not clean, commit or stash first" >&2
    exit 1
fi

if git rev-parse "$tag" >/dev/null 2>&1; then
    echo "tag $tag already exists" >&2
    exit 1
fi

sed -i.bak "s/^version = .*/version = \"$version\"/" Cargo.toml
rm Cargo.toml.bak
cargo build -p session_tracker_core -p session_tracker_net >/dev/null

git add Cargo.toml Cargo.lock
git commit -m "bump version to $version"
git push
git tag "$tag"
git push origin "$tag"

echo "Pushed $tag - watch the Release workflow in the Actions tab for the build/publish."
