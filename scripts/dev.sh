#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

command -v watchexec >/dev/null || exec nix develop --command "$0" "$@"

[ -e target/share/drakeflake/functions.json ] || scripts/function-index.sh ||
  echo "dev.sh: no function index, the Functions page stays empty" >&2

exec watchexec --restart --debounce 300ms \
  --watch crates --watch src/qml --watch Cargo.toml --watch Cargo.lock \
  --exts rs,qml,js,nix,toml,lock \
  -- cargo run --bin drakeflake -- "$@"
