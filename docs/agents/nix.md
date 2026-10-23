# Evaluation, generated files and inputs

## Running programs

- Anything that starts a program takes a `drakeflake_exec::Context`. Anything
  that evaluates takes `&drakeflake_flake::Nix`. Pass them as parameters, never
  store them in `Target`. Threads get a clone (`Nix::sibling`, `Nix::workers`).
- Pty and detached programs (`hub/pty*`, `hub/terminal`) are outside the port.
- A new load cancels the previous one. Each operation has its own cancel token.
- Test `is_cancelled()` and `is_timeout()` on errors. Never search program output.
- Sharded evaluations share one slot pool, sized by CPU and half the free memory.
- Never reload fully when only the output path changes (`Hub::set_managed`).

## Tests

- Enable `drakeflake-exec`'s `testing` feature. Use `ScriptedRunner` and read
  `runner.calls()`. Hub tests use `Hub::with_context`.

## Load and reload

- The load pipeline (`hub/steps.rs`) runs in parallel with priorities.
- Values are evaluated only for options in `USER_FILES`. The rest show defaults
  and evaluate on demand.
- After Apply, only changed options re-evaluate (`hub/reload.rs`).
- Reloading the same flake and host keeps staged changes. Switching systems
  drops them.
- A save that creates the git repository restarts the load, because Nix refused
  untracked files until then.

## Generated files

Code is in `crates/file/src/{module,option}.rs`.

- Shared prefixes become nested attribute sets.
- Per-host scope is `lib.mkIf (config.networking.hostName == …)`, never on
  hostName itself.
- A split layout writes `default.nix` plus a folder. Switching layouts removes
  the other entry.
- Source questions use the syntax tree (`drakeflake_validate::definition_sites`,
  `drakeflake_file::is_imported`), never text search.

## Inputs and setup

- The first start without a flake runs the setup assistant. `DRAKEFLAKE_SETUP=1`
  forces it.
- `nix flake update` exits 0 when GitHub rate-limits and keeps cached revisions.
  `UpdatePlan` therefore pins known-newest inputs with `--override-input`.

## Hub ownership

- `Session` holds loaded state under one lock. Readers take `snapshot()`. Never
  add a second `Mutex` for state that changes with it.
- `Hub::spawn_background` reads. Writing files, pty programs and commits are
  `Exclusive` methods, reachable only through `Hub::spawn_exclusive`.
- A second operation gets `HubError::Busy`.
- `Running(true)` and `Finished` post under the slot lock, so a new start never
  overtakes the old end.
