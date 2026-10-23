#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
out=$(realpath -m "${1:-$repo/target/coverage/report}")
suites=${COVERAGE_SELFTESTS-plain hm darwin missing}

ignore=(--ignore-filename-regex '(^|/)tests(\.rs$|/)|/flake/src/live\.rs$|/integration\.rs$')

cd "$repo"
export CARGO_TARGET_DIR="$repo/target/coverage"
# shellcheck source=/dev/null
source <(cargo llvm-cov show-env --sh)
cargo llvm-cov clean --workspace

failed=()
cargo nextest run --workspace --profile ci || failed+=("tests")

if [ -n "$suites" ]; then
  cargo build --bin drakeflake
  for suite in $suites; do
    SELFTEST_BINARY="$CARGO_TARGET_DIR/debug/drakeflake" \
      "$repo/scripts/selftest.sh" "$suite" "coverage-$suite" || failed+=("selftest $suite")
  done
fi

mkdir -p "$out"
cargo llvm-cov report "${ignore[@]}" --cobertura --output-path "$out/cobertura.xml"
cargo llvm-cov report "${ignore[@]}" --html --output-dir "$out"
summary=$(cargo llvm-cov report "${ignore[@]}" --summary-only)
printf '%s\n' "$summary"

# llvm-cov's own line total (the TOTAL row's tenth column), not Cobertura's
# line-rate, which counts lines differently and would not match the summary.
awk '$1 == "TOTAL" { print "Line coverage: " $10 }' <<<"$summary"
if [ ${#failed[@]} -gt 0 ]; then
  echo "Failed while measuring (the report covers what ran): ${failed[*]}" >&2
fi
echo "Report: $out/cobertura.xml, $out/html/index.html"
