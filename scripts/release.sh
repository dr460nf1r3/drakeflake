#!/usr/bin/env bash
set -euo pipefail

dry_run=0
if [ "${1:-}" = "--dry-run" ]; then
  dry_run=1
  shift
fi

root=$(git rev-parse --show-toplevel)
cd "$root"

fail() {
  echo "release: $*" >&2
  exit 1
}

branch=$(git branch --show-current)
[ "$branch" = main ] || fail "run it on main, not on $branch"

if [ -n "$(git status --porcelain)" ]; then
  fail "the working tree is not clean"
fi

next=${1:-$(git cliff --bumped-version)}
next=${next#v}

[[ $next =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "$next is not a MAJOR.MINOR.PATCH version"

if git rev-parse -q --verify "refs/tags/$next" >/dev/null; then
  fail "tag $next exists already, so there are no unreleased commits to release"
fi

if [ "$dry_run" = 1 ]; then
  echo "Next version: $next"
  git cliff --unreleased --tag "$next" --strip header
  exit 0
fi

sed -i "/^\[workspace\.package\]/,/^\[/ s/^version = .*/version = \"$next\"/" Cargo.toml
cargo update --workspace --offline >/dev/null
git cliff --tag "$next" --output CHANGELOG.md

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): $next"
git tag -a "$next" -m "$next"

echo "Tagged $next on $(git rev-parse --short HEAD)."
echo "Push with: git push origin main $next"
