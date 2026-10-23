#!/usr/bin/env bash
# DRAKEFLAKE_VM_GUEST stays unset so tests that switch the running system return early on the host.
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
root=${VM_ITEST_ROOT:-${TMPDIR:-/tmp}/drakeflake-itest}
dir="$root/host"

# Inside this repository the app would `git add` generated files into the checkout.
rm -rf "$dir"
mkdir -p "$dir/cache" "$dir/config" "$dir/state"
cp -r "$repo/tests/fixtures/testflake" "$dir/flake"
cp -r "$repo/tests/fixtures/hmflake" "$dir/hmflake"
cp -r "$repo/tests/fixtures/lifecycleflake" "$dir/lifecycle-flake"
chmod -R u+w "$dir/flake" "$dir/hmflake" "$dir/lifecycle-flake"

export XDG_CACHE_HOME="$dir/cache" XDG_CONFIG_HOME="$dir/config" XDG_STATE_HOME="$dir/state"
export DRAKEFLAKE_TEST_FLAKE="$dir/flake" DRAKEFLAKE_TEST_HOST=testhost
export DRAKEFLAKE_TEST_HM_FLAKE="$dir/hmflake" DRAKEFLAKE_TEST_HM_HOST=hmhost
export DRAKEFLAKE_TEST_LIFECYCLE_FLAKE="$dir/lifecycle-flake"

# The seed check needs the second system in the store without a substituter.
nix build "$dir/lifecycle-flake#nixosConfigurations.lifecyclehost.config.system.build.toplevel" \
  --out-link "$dir/lifecycle-system" --print-out-paths >/dev/null

cd "$repo"
cargo nextest run --workspace --profile ci --run-ignored ignored-only --no-fail-fast \
  -p drakeflake-flake -p drakeflake-file -p drakeflake-plan -p drakeflake-app -p drakeflake-store
