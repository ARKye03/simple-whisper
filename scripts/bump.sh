#!/usr/bin/env bash
# Bump app version across VERSION, package.json, src-tauri/Cargo.toml,
# refresh Cargo.lock, commit, and create a signed tag. Push is left to you.
#
# Usage: scripts/bump.sh <version>
#   Accepts 0.3.1 or v0.3.1 (v-prefix optional).

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: scripts/bump.sh <version>  (e.g. 0.3.1 or v0.3.1)" >&2
  exit 64
fi

INPUT="$1"

if [[ ! "$INPUT" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: '$INPUT' is not a valid semver (expected X.Y.Z or vX.Y.Z)" >&2
  exit 65
fi

RAW="${INPUT#v}"
TAG="v${RAW}"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "trunk" ]]; then
  echo "error: must be on 'trunk' (currently on '$BRANCH')" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is not clean. Commit or stash first." >&2
  git status --short >&2
  exit 1
fi

if git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
  echo "error: tag '$TAG' already exists locally" >&2
  exit 1
fi

if [[ -n "$(git ls-remote --tags origin "refs/tags/$TAG" 2>/dev/null)" ]]; then
  echo "error: tag '$TAG' already exists on origin" >&2
  exit 1
fi

echo "Bumping to $TAG ($RAW)..."

printf '%s\n' "$RAW" > VERSION

pnpm version "$RAW" --no-git-tag-version >/dev/null

sed -i '' -E '1,/^version = /s/^version = .*/version = "'"$RAW"'"/' src-tauri/Cargo.toml

(cd src-tauri && cargo update -p simple-whisper --quiet)

git add VERSION package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json

git commit -m "chore: release $TAG"

git tag -s "$TAG" -m "Release $TAG"

cat <<EOF

Bumped to $TAG.
Review:
  git show HEAD
  git tag -v $TAG
Push:
  git push origin trunk
  git push origin $TAG
EOF
