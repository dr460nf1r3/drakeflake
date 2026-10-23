#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

out=${1:-src/qml/i18n}
mkdir -p "$out"

for ts in i18n/drakeflake_*.ts; do
  lang=${ts##*/drakeflake_}
  lang=${lang%.ts}
  [ "$lang" = en ] && continue
  lrelease -silent "$ts" -qm "$out/qml_$lang.qm"
  echo "compiled $lang -> $out/qml_$lang.qm"
done
