#!/usr/bin/env bash
# Bump app version across VERSION, package.json, src-tauri/Cargo.toml,
# refresh Cargo.lock, regenerate CHANGELOG.md, commit, and create a signed tag.
# Push is left to you.
#
# Usage: scripts/bump.sh
#   Version is inferred from unreleased conventional commits via `git cliff --bumped-version`.

set -euo pipefail

if ! command -v git-cliff >/dev/null 2>&1; then
  echo "error: git-cliff not found. Install with: brew install git-cliff" >&2
  exit 1
fi

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

CURRENT="$(cat VERSION)"
TAG="$(git cliff --bumped-version)"

if [[ -z "$TAG" || ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: git-cliff did not return a valid version (got: '$TAG')" >&2
  exit 1
fi

RAW="${TAG#v}"

if [[ "$RAW" == "$CURRENT" ]]; then
  echo "error: no unreleased conventional commits since v$CURRENT — nothing to bump" >&2
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

node -e 'const fs=require("fs");const p=JSON.parse(fs.readFileSync("package.json","utf8"));p.version=process.argv[1];fs.writeFileSync("package.json",JSON.stringify(p,null,2)+"\n");' "$RAW"

CARGO_TMP="$(mktemp)"
sed -E '1,/^version = /s/^version = .*/version = "'"$RAW"'"/' src-tauri/Cargo.toml > "$CARGO_TMP"
mv "$CARGO_TMP" src-tauri/Cargo.toml

(cd src-tauri && cargo update -p simple-whisper --quiet)

git cliff --tag "$TAG" --unreleased --prepend CHANGELOG.md

git add VERSION package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md

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
