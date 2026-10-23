#!/usr/bin/env bash
# The runner image has no mdbook, so a pinned release is downloaded when none is on PATH.
set -euo pipefail
cd "$(dirname "$0")/.."

version=0.5.4

if ! command -v mdbook >/dev/null; then
  bin=$(mktemp -d)
  trap 'rm -rf "$bin"' EXIT
  curl -fsSL "https://github.com/rust-lang/mdBook/releases/download/v$version/mdbook-v$version-x86_64-unknown-linux-musl.tar.gz" |
    tar -xz -C "$bin"
  PATH="$bin:$PATH"
fi

mdbook --version
mdbook build docs
