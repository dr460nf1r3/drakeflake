#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

command -v qmllint >/dev/null || exec nix develop --command "$0" "$@"

qt=$(dirname "$(dirname "$(command -v qmllint)")")/lib/qt-6/qml
imports=(-I "$qt" -I src/qml)
qml_paths="${QMLLINT_IMPORT_PATH:-}"
for dir in ${qml_paths//:/ }; do
  [ -d "$dir" ] && imports+=(-I "$dir")
done

if [ $# -eq 0 ]; then
  mapfile -t files < <(find src/qml -name '*.qml')
  set -- "${files[@]}"
fi

exec qmllint --max-warnings 0 "${imports[@]}" "$@"
