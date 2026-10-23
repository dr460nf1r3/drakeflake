# Self-test

`scripts/selftest.sh [plain|hm|darwin|missing] [run]` copies a fixture flake to a
temp dir. Do not run it in the repo: the app would `git add` the generated files.

It runs the app headless (`QT_QPA_PLATFORM=offscreen`) and prints `SELFTEST PASS`
or `FAIL`. Shots land in the run's `shots/`. `hm` also runs a real
`nix flake update home-manager` on the copy. Use `console.warn` in steps, because
debug output is filtered.

## Suites

- `plain`, `hm` and `darwin` use the fixtures `testflake`, `hmflake` and
  `darwinflake`, in the step order of `SelfTest.qml`.
- `missing` runs only `selftest/missing.js`, with no flake folder.
- CI, `scripts/coverage.sh` and `AGENTS.md` run all four. A new suite goes into
  all three.

## Environment

| Variable                    | Effect                                                |
| --------------------------- | ----------------------------------------------------- |
| `SELFTEST_APPLY`            | `save` (default), `build` or empty                    |
| `SELFTEST_PROFILE`          | `debug` (default) or `release`                        |
| `SELFTEST_BINARY`           | run this `bin/drakeflake` as is (CI)                  |
| `SELFTEST_WRAPPER`          | a command in front of the binary, for example `gdb`   |
| `SELFTEST_DESKTOP_THEME=1`  | desktop colours, fonts and icons                      |
| `SELFTEST_INPUTS_FROM`      | flake whose nixpkgs provides Plasma's platform theme  |
| `SELFTEST_MAC_ICONS=1`      | only the macOS icon resources                         |
| `SELFTEST_OFFLINE=1`        | upstream `git` and `curl` fail; the app must not hang |
| `DRAKEFLAKE_FUNCTION_INDEX` | an existing function index                            |

## Offscreen limits

- The global menu mirror cannot run offscreen. Qt.labs.platform menus crash
  (exit 139). Test the mirror by hand.
- Shots are grabbed a frame after `test.shot()`. A step that changes the page
  right after a shot needs `delay` or its own step.

## Driving controls

- Reach controls with `test.findChild(item, objectName)` and call signals
  (`button.clicked()`), so the wiring is tested. Calling a model slot directly
  skips it.
