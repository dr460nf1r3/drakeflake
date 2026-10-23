#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
output=${1:-$repo/target/share/drakeflake/functions.json}
generator="$repo/target/${PROFILE:-${SELFTEST_PROFILE:-debug}}/drakeflake-function-index"

if [ ! -x "$generator" ]; then
  cargo build --manifest-path "$repo/Cargo.toml" -p drakeflake-functions --bin drakeflake-function-index
  generator="$repo/target/debug/drakeflake-function-index"
fi

# `--raw` prints no final newline, which `read` would report as a failure.
read -r nixpkgs rev date <<<"$(nix eval --raw --impure --expr "
  let locked = (builtins.fromJSON (builtins.readFile $repo/flake.lock)).nodes.nixpkgs.locked;
  in \"\${(builtins.fetchTree locked).outPath} \${locked.rev} \${toString locked.lastModified}\"")"
system=$(nix eval --raw --impure --expr builtins.currentSystem)

# The locked nixpkgs' lib calls `builtins.convertHash`, which Lix does not have,
# so the evaluation runs with the Nix that this nixpkgs ships.
nix_bin=$(nix build --no-link --print-out-paths --impure --expr "(import $nixpkgs {}).nix.out")/bin

PATH="$nix_bin:$PATH" bash "$repo/nix/function-index/build.sh" "$nixpkgs" "$system" "$rev" "$date" "$output" "$generator"
