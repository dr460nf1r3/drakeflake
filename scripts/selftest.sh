#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
kind=${1:-plain}
run=${2:-$kind}
root=${SELFTEST_ROOT:-${TMPDIR:-/tmp}/drakeflake-selftest}
dir="$root/$run"

case $kind in
plain) fixture=testflake host=testhost ;;
hm) fixture=hmflake host=hmhost ;;
darwin) fixture=darwinflake host=darwinhost ;;
missing) fixture=testflake host=testhost ;;
*)
  echo "usage: $0 [plain|hm|darwin|missing] [run-name]" >&2
  exit 2
  ;;
esac

rm -rf "$dir"
mkdir -p "$dir/shots" "$dir/config" "$dir/cache" "$dir/state"
cp -r "$repo/tests/fixtures/$fixture" "$dir/flake"
chmod -R u+w "$dir/flake"

binary=${SELFTEST_BINARY:-$repo/target/${SELFTEST_PROFILE:-debug}/drakeflake}
[ -x "$binary" ] || {
  echo "build first: $binary is missing" >&2
  exit 2
}

export XDG_CONFIG_HOME="$dir/config" XDG_CACHE_HOME="$dir/cache" XDG_STATE_HOME="$dir/state"
export DRAKEFLAKE_FLAKE="$dir/flake" DRAKEFLAKE_HOST="$host"
export QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QT_FORCE_STDERR_LOGGING=1

if [ "$kind" = missing ]; then
  export DRAKEFLAKE_FLAKE="$dir/no-such-flake"
fi

if [ -n "${SELFTEST_DESKTOP_THEME:-}" ]; then
  if [ "$(uname -s)" = Darwin ]; then
    # The macOS controls are drawn by AppKit, which the offscreen platform lacks.
    # A grabbed image has nothing behind a blurred window, so it stays solid.
    export QT_QPA_PLATFORM=cocoa DRAKEFLAKE_BLUR="${DRAKEFLAKE_BLUR:-off}"
    unset QT_QUICK_BACKEND
  else
    cp "${HOME}/.config/kdeglobals" "$dir/config/"
    integration=$(nix build --no-link --print-out-paths --inputs-from "${SELFTEST_INPUTS_FROM:-$repo}" nixpkgs#kdePackages.plasma-integration)
    export QT_QPA_PLATFORMTHEME=kde QT_PLUGIN_PATH="$integration/lib/qt-6/plugins${QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}"
  fi

  export DRAKEFLAKE_SELFTEST_THEME=1
fi
if [ -n "${SELFTEST_MAC_ICONS:-}" ]; then
  DRAKEFLAKE_ICON_RCC_DIR=$(nix build --no-link --print-out-paths "${SELFTEST_INPUTS_FROM:-$repo}#icon-resources")
  export DRAKEFLAKE_ICON_RCC_DIR XDG_DATA_DIRS=/nonexistent XDG_CONFIG_DIRS=/nonexistent
  unset XDG_CURRENT_DESKTOP
fi
if [ -n "${SELFTEST_OFFLINE:-}" ]; then
  mkdir -p "$dir/bin"
  offline_shim() {
    {
      echo '#!/bin/sh'
      echo "echo \"selftest offline: $1 blocked\" >&2"
      echo 'exit 1'
    } >"$dir/bin/$1"
    chmod +x "$dir/bin/$1"
  }
  offline_shim git
  offline_shim curl
  export PATH="$dir/bin:$PATH" DRAKEFLAKE_SELFTEST_OFFLINE=1
fi
if [ -z "${DRAKEFLAKE_FUNCTION_INDEX:-}" ]; then
  "$repo/scripts/function-index.sh" "$dir/functions.json"
  export DRAKEFLAKE_FUNCTION_INDEX="$dir/functions.json"
fi
make_root() {
  local file
  for file in etc/machine-id etc/hosts etc/NetworkManager/system-connections/home.nmconnection \
    var/log/messages var/lib/bluetooth/adapter/info var/cache/app/blob \
    home/alice/Documents/notes.txt home/alice/.cache/app/data home/alice/.config/app/settings.ini; do
    mkdir -p "$1/$(dirname "$file")"
    printf 'selftest %s\n' "$file" >"$1/$file"
  done
  head -c 65536 /dev/zero >"$1/var/cache/app/blob"
}
make_root "$dir/root"
mkdir -p "$dir/blank/etc"
touch "$dir/blank/etc/hosts"
export DRAKEFLAKE_PERSIST_ROOT="$dir/root" DRAKEFLAKE_PERSIST_SNAPSHOT="$dir/blank"
export DRAKEFLAKE_SELFTEST_DIR="$dir/shots" DRAKEFLAKE_SELFTEST_APPLY="${SELFTEST_APPLY-save}" DRAKEFLAKE_SELFTEST_SUITE="$kind"

cd "$repo"
start=$(date +%s)
set +e

if [ -n "${SELFTEST_BINARY:-}" ] || command -v qmllint >/dev/null; then
  # shellcheck disable=SC2086 # the wrapper is a command line
  timeout 2400 ${SELFTEST_WRAPPER:-} "$binary" >"$dir/out.log" 2>&1
else
  timeout 2400 nix develop "$repo" -c "$binary" >"$dir/out.log" 2>&1
fi
code=$?
set -e

elapsed=$(($(date +%s) - start))
printf '%s %s\n' "$code" "$elapsed" >"$dir/result"
grep -E "SELFTEST (PASS|FAIL)" "$dir/out.log" || true
echo "exit=$code elapsed=${elapsed}s log=$dir/out.log shots=$dir/shots"
exit "$code"
