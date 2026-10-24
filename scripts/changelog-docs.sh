#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: changelog-docs.sh <source-root> <output>" >&2
  exit 2
fi

sed 's|docs/src/images/|images/|' "$1/CHANGELOG.md" >"$2"
