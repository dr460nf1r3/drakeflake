# Toolchain, lints and CI

## Flake checks

- `formatting` (treefmt, must change nothing), `clippy`, `tests` (nextest),
  `file-size`, `lint`, `cargo-deny` (offline), `qml-lint` (qmllint, zero
  warnings) and `qml-test` (qmltestrunner over `tests/tst_*.qml`).
- Formatters live in `nix/formatter.nix`. The Nix templates in
  `crates/flake/src/expr` and QML JavaScript are excluded from formatting.
- The sandbox has no system QML, so qmllint uses `kirigami.unwrapped`.

## CI

- `selftest` runs all four suites against `.#default`. It needs `nix` and
  network, and uploads screenshots and logs.
- `rust-tests` and `selftest` publish JUnit (`scripts/selftest-junit.sh`).
- `coverage` runs `scripts/coverage.sh` and never fails on test failures.
- Live tests stay out of CI.

## Toolchain and Cargo

- Versions live in `[workspace.dependencies]`. `rust-version` is 1.91.
- One pinned nightly (`nix/rust-toolchain.nix`) serves the dev shell, checks,
  formatter and package. `nix flake update rust-overlay` moves it.
- Nothing builds on stable, so the minimum version is unchecked.
- No LTO and no `panic = "abort"`. qtbridge-interfaces is at opt-level 0.
- `cargo deny check` needs network for advisories, so run it in the dev shell.

## Lints

- Clippy's pedantic, nursery and cargo groups are denied. Useful `restriction`
  lints are denied by name.
- Exceptions use `#[expect(lint, reason = …)]`, never `#[allow]`.
- `unsafe` needs `#[expect(unsafe_code, reason = …)]` and a `// SAFETY:` comment
  directly above the block.
- `clippy.toml` orders module items by kind only. Fields, variants and impl items
  keep their order.
- `rustfmt.toml` uses nightly options.

## Dev loop

- `scripts/dev.sh` rebuilds and restarts on changes to Rust, QML, the Nix
  templates or Cargo files. QML is read from `src/qml`, so QML edits restart
  without recompiling.
- Rust builds bake in the shell's Qt paths. Rebuild inside the shell after shell
  changes.
- Debug with Attach to Process. Debug builds allow it despite Yama.
