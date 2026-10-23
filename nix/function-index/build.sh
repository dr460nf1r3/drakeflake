#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
nixpkgs=$1 system=$2 rev=$3 date=$4 output=$5
generator=${6:-drakeflake-function-index}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Deprecated lib functions print warnings while they are listed: keep them
# out of the way unless the evaluation fails.
if ! nix-instantiate --store dummy:// --eval --strict --xml "$here/collect.nix" \
  --arg nixpkgs "$nixpkgs" --argstr system "$system" >"$work/positions.xml" 2>"$work/eval.log"; then
  cat "$work/eval.log" >&2
  exit 1
fi

# Hidden commands only work as the first argument: no flags before it.
NIX_CONFIG="extra-experimental-features = nix-command" nix __dump-language >"$work/builtins.json"

mkdir -p "$(dirname "$output")"
"$generator" \
  --positions "$work/positions.xml" \
  --builtins "$work/builtins.json" \
  --nixpkgs "$nixpkgs" \
  --rev "$rev" \
  --date "$date" \
  --nix-version "$(nix --version)" \
  --output "$output"
