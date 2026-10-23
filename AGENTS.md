# DrakeFlake — notes for agents

Rust (qtbridge 0.3) + Kirigami/QML desktop app that searches and edits NixOS and
Home Manager options, packages and flake inputs of a system flake. Edition 2024.

## Topic notes

Read the matching note in `docs/agents/` before changing its area:

| Note                                     | Read before you touch                                                               |
| ---------------------------------------- | ----------------------------------------------------------------------------------- |
| [qml.md](docs/agents/qml.md)             | QML files, model slots and properties, `API_VERSION`, icons, file pickers           |
| [nix.md](docs/agents/nix.md)             | `crates/flake`, `file`, `validate`, load and reload, generated files, input updates |
| [security.md](docs/agents/security.md)   | untrusted text in QML, root rights, `crates/helper`, `plan`, `scan`, polkit         |
| [platforms.md](docs/agents/platforms.md) | NixOS vs macOS, `cfg(target_os)`, the darwin package, the macOS style               |
| [logging.md](docs/agents/logging.md)     | a `log` call or a Console message                                                   |
| [toolchain.md](docs/agents/toolchain.md) | `Cargo.toml`, lints, flake checks, CI, the dev loop                                 |
| [selftest.md](docs/agents/selftest.md)   | running or extending the self-test                                                  |
| [docs.md](docs/agents/docs.md)           | the user guide in `docs/src/`, its screenshots and `SUMMARY.md`                     |

## Layout

- `crates/app`: the GUI. `hub.rs` and `hub/*` run services and background work,
  `models/*` are the QML models, `wiring.rs` connects models to the hub.
  - Models: `systemModel`, `consoleModel`, `setupModel`, `inputsModel`,
    `imagesModel`, `generationsModel`, `settingsModel`, `docsModel`,
    `persistenceModel`, `optionModel` (with `configuredModel`, `homeModel` and
    the tree models, which are `OptionModel`s fed the same messages),
    `packageModel`, `changeModel`, `functionModel` and `searchModel`.
  - qtbridge allows one `#[qobject]` impl per type, so a model's slots stay in
    one file and helpers go to child modules.
  - Each model drains its own hub mailbox. `models/relay.rs` carries notes from
    one model to `SystemModel`, and models never borrow each other.
- `crates/app/src/hub/*`: `Hub` (shared as `Arc<Hub>`) owns four services:
  `Session` (what is loaded, read as a snapshot), `Operations` (the one running
  operation and its cancel token), `Evaluation` (value, check and source-search
  workers) and `Console` (Console lines and status activities).
  - `facade.rs` is what the models call. `impl Hub` lives only in `hub.rs` and
    `facade.rs`.
  - `Hub::spawn_background` runs read-only work. `Hub::spawn_exclusive` runs an
    operation that holds the slot and ends in an `Outcome`.
  - Writing files, running a program in the pty and committing are methods of
    `Exclusive`, so background work cannot call them.
  - Hub functions called by the slots return `HubError`.
  - A cache choice is an enum (`CacheUse`, `HomeData`), not a `bool`.
  - A save that creates the git repository restarts the load.
- `crates/exec`: the port for running programs. `Runner` is the trait,
  `SystemRunner` starts real processes, `Context` passes a runner and a
  `CancelToken` explicitly. It depends on no workspace crate.
- `crates/fs`: config, state, cache and runtime dirs, and `atomic_write`. It
  falls back to a private temp dir without a usable `HOME`, and never writes to
  a shared place. It depends on no workspace crate.
- `crates/flake`: everything that runs `nix`. Expressions live in
  `src/expr/*.nix`. `lock/*` reads and plans `flake.lock` updates.
- `crates/file`: generated files, write plans (`direct`, `elevated`, `backup`),
  and the staged changes. It does not depend on `flake`. The automatic commit
  lives in `crates/app/src/hub/commit/`.
- `crates/plan`: what the root helper links, and nothing more. Import from
  `drakeflake_plan`.
- `crates/helper`: the pkexec helper. It depends only on `plan` and `scan`
  (check with `cargo tree -p drakeflake-helper`).
- `crates/scan`: the root file system scan for the Impermanence page.
- `crates/config`: `config.toml`, layout types, default flake path, blur mode,
  tray lock path. It depends on `fs` and `render` only.
- `crates/render`: Nix literals and `split_option_name`.
- `crates/functions`: an offline index of Nix functions, built at package build
  time by `nix/function-index`.
- `crates/store`: in-memory option and package indexes with search.
- `crates/validate`: option types, typed values and rnix-based source questions.
- `crates/evaluate`: the evaluation steps and the per-target `Cache` shared by
  the app and the trays.
- `crates/log`: rotating log file behind the `log` facade. Qt messages reach it
  through the stderr tee in `app/src/stderr_log.rs`.
- `crates/tray-core`: the schedule, update check, single-instance lock and
  `--check-once`, shared by both trays.
- `crates/tray` (Linux, ksni) and `crates/tray-macos` (`tray-icon`, LaunchAgent
  written by `models/settings/tray.rs`): platform layers over `tray-core`.
- `crates/lint`: `drakeflake-lint`, the comment and shape rules. Not installed
  with the app.
- `src/qml`: the UI. `SelfTest.qml` runs the scripted test, whose steps live in
  `selftest/*.js`, one file per area, in a fixed order.
- `tests/fixtures/*`: throwaway flakes. `testflake` and `hmflake` for the
  self-test, `darwinflake` for the darwin self-test, `lifecycleflake` for the VM
  tests.

## Names and icons

- The app is **DrakeFlake**. Binaries are `drakeflake`, `drakeflake-tray` and
  `drakeflake-helper`. The package installs `libexec/drakeflake` behind the Qt
  wrapper `bin/drakeflake`, so the Wayland app_id is `drakeflake`. qtbridge 0.3
  cannot set `desktopFileName`.
- XDG dirs are `drakeflake`. Environment variables are `DRAKEFLAKE_*`. Polkit
  actions are `org.dr460nf1r3.drakeflake.*`.
- Crates are `drakeflake-<x>`, Rust paths are `drakeflake_<x>::`, directories
  are `crates/<x>`. Translations are `i18n/drakeflake_<lang>.ts`.
- Generated files: default entry `drakeflake.nix`, parts folder `drakeflake`,
  header `# GENERATED by DrakeFlake …`, state lines `# drakeflake-state:` and
  `# drakeflake-files:`. No fallback for older names.
- Icons and the logo come from `scripts/make-icons.py`. Do not hand-edit the SVGs.

## Checks (all must pass)

```sh
nix develop                             # Qt, Kirigami, qmllint, qmlformat
cargo clippy --workspace --all-targets  # zero warnings
cargo nextest run --workspace
cargo test -p drakeflake-flake -- --ignored  # live tests: network, test flake
nix fmt                                 # treefmt: nix, rust, qml, shell, prettier
scripts/qmllint.sh
scripts/qmltest.sh [tst-file]           # Qt Quick Tests, headless
scripts/coverage.sh
scripts/lint.sh                         # drakeflake-lint
scripts/selftest.sh plain && scripts/selftest.sh hm && scripts/selftest.sh darwin && scripts/selftest.sh missing  # after cargo build
```

CI runs these through `nix flake check` and `.github/workflows/`. See
[toolchain.md](docs/agents/toolchain.md) for what each check covers.

## Done means

- A user-visible feature adds selftest steps in `selftest/*.js`, wired into the
  order in `SelfTest.qml`. Cover the happy path and one edge case. Add user docs
  under `docs/src/`. Rust tests alone are not enough. See
  [selftest.md](docs/agents/selftest.md) and [docs.md](docs/agents/docs.md).
- UI work is finished only after you open the screenshots in `shots/` taken by
  every step that shows a page you changed. Check for missing or hidden controls,
  clipped or overlapping text, error banners, empty pages and wrong pages. A
  passing run does not catch these.

## Rules for every change

- `.rs`, `.qml` and QML `.js` files stay at or under 400 lines
  (`scripts/check-file-size.sh`). `.file-size-allow` lists files still being
  split and may only shrink. Tests live in their own file, for example
  `foo/tests.rs`.
- Comments say why, and only when the code cannot show it. Never restate names,
  types or the next line, and never describe what the code does.
- No semicolons in comments or prose. End the sentence and start a new one.
  `scripts/lint.sh` enforces the comment rules. A real exception gets
  `// drakeflake-lint: allow(<rule>) <reason>` on the line above.
- Every `nix` call runs with cwd set to the flake directory.
- New files must be `git add`ed before `nix build` sees them.
