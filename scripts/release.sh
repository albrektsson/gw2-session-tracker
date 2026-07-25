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
