#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
input=${1:-tests}
junit=${QMLTEST_JUNIT:-$(mktemp "${TMPDIR:-/tmp}/drakeflake-qmltest-XXXXXX.xml")}

run() {
  cd "$repo"
  export QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QT_FORCE_STDERR_LOGGING=1
  qmltestrunner -input "$input" -o "$junit,junitxml" -o -,txt
}

command -v qmltestrunner >/dev/null || exec nix develop "$repo" -c "$repo/scripts/qmltest.sh" "$input"

run

echo "junit=$junit"
