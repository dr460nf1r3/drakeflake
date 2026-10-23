# Security policy

## Reporting a vulnerability

Please do not report security problems in a public issue.

Open a **private vulnerability report** on
[github.com/dr460nf1r3/drakeflake](https://github.com/dr460nf1r3/drakeflake/security/advisories/new).
Only you and the maintainers can read it.

Include what you can of:

- the version or commit you tested,
- the platform (NixOS or macOS) and how the app got root rights (pkexec,
  run0, sudo or the macOS administrator dialog),
- the steps to reproduce and what an attacker gains.

## Scope

DrakeFlake runs most of its work as the user and a small helper
(`drakeflake-helper`) as root. Problems in the helper, in the polkit actions
or in how the app treats text from flakes, inputs, packages and build output
matter most. [Privacy and security](docs/src/security.md) describes what runs
with which rights.

## Supported versions

DrakeFlake has no release yet. Fixes go to `main`.
