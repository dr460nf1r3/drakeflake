# Security, root rights and the helper

User-facing text is in `docs/src/security.md`.

## Untrusted text

- Labels showing flake, package or build text set `textFormat: Text.PlainText`.
- Rich text goes through `Helpers.link(url, label)` or `Helpers.richText(x)`.
  Markdown goes through `app.docText`, which strips images.
- Links open only through `app.openExternal`, and only for http and https.
- Paths from evaluation are untrusted. An image's `filePath` stays inside the
  image output, QEMU option values double their commas, and only `.nix` files
  reach `xdg-open`.
- The password bar is offered only while the pty has echo off. Console passwords
  are never logged.

## Elevation

- `drakeflake_file::find_elevation` picks, in order: `Elevation::MacDialog` on
  macOS, setuid pkexec, `run0`, and setuid sudo (console only).
- The macOS dialog runs `data/macos/drakeflake-elevate.sh` through `osascript`,
  and sets `SUDO_UID`. It is a plain prefix, so nh accepts it as
  `--elevation-strategy`.
- Never assume the NixOS pkexec wrapper exists.

## Helper

- One polkit action per subcommand. `apply`, `activate` and `install-lock` are
  `auth_admin` without keep. Only `scan-ephemeral` is `auth_admin_keep`.
- `apply` validates paths once, stages every file under a temp name, swaps them
  in with the entry last, and rolls back on failure. The walk uses `openat` with
  `O_NOFOLLOW` (`helper/src/secure.rs`).
- Writer and helper take only a `ValidatedPlan` (`WritePlan::validate` or
  `validate_local`).
- `install-lock` refuses without `PKEXEC_UID` or `SUDO_UID`.
- The helper never evaluates, builds or updates a flake. Lock updates run as the
  user.
- `scan-ephemeral` reads names and sizes only, with hard visit limits.

## Exit codes

`drakeflake_plan::exit`: 0 done, 2 usage, 3 refused, 4 I/O, 5 a program failed.
The helper never uses pkexec's 126 or 127. `drakeflake-elevate` ends a cancelled
dialog with 127.
