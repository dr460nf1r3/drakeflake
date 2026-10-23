# Installation and requirements

This page lists what DrakeFlake needs and shows how to install it system-wide
or run it from a checkout.

## What you need

- **NixOS on x86_64 or aarch64, or macOS on Apple silicon with nix-darwin.**
  The package is built for `x86_64-linux`, `aarch64-linux` and
  `aarch64-darwin`. Intel Macs are not supported, because current nixpkgs no
  longer builds for `x86_64-darwin`.
- **A flake-based system configuration** in a local folder. It needs a
  `flake.nix` that defines `nixosConfigurations.<hostname>` (or, on a Mac,
  `darwinConfigurations.<hostname>`) for this machine. The default folder is
  `/etc/nixos`, or `/etc/nix-darwin` on a Mac. Any folder works, for example
  one in your home folder. See [nix-darwin systems](darwin.md).
- **git**, if your flake is a git repository. Flakes only see files that git
  tracks, so the app marks the generated files for git (see
  [Reviewing and applying](applying.md#git)). The package brings its own `git`
  and `curl` for this and for update checks.
- **A way to get administrator rights** for applying. The app uses the first
  one available:
  1. a setuid `pkexec` (on NixOS this comes with polkit),
  2. `run0`,
  3. `sudo`.

  On a Mac the administrator dialog of macOS is used. `sudo` only works for
  rebuilds in the Console. Writing generated files into a root-owned flake
  needs a dialog (pkexec, run0 or the macOS one). See
  [Privacy and security](security.md).

### Recommended

- **nh** (`programs.nh.enable = true;`). With nh you get **Review and apply…**:
  a summary of what changes before switching, with a confirmation. nh is also
  used for switching generations and for **Cleanup**. Without nh the app:
  - builds with `nixos-rebuild`,
  - switches generations through its own helper,
  - cleans up with `nix-collect-garbage`.

  The **Actions** page recommends installing nh when it is missing.

### Optional

| Tool                                          | Used for                                                                                                                                                                                                    |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **nvd**                                       | **Generations → Compare**, when installed. Otherwise the app uses `nix store diff-closures`.                                                                                                                |
| **nix-index**                                 | **Packages → Search by command** uses `nix-locate`. If it is not installed, the app builds it from your system's nixpkgs when needed. The command database itself can be downloaded from the Packages page. |
| **QEMU**                                      | **Actions → Start in VM** uses `qemu-system-<arch>` (for example `qemu-system-x86_64`) when it is installed. Otherwise it is built from your system's nixpkgs the first time.                               |
| **A terminal emulator** (for example Konsole) | **Try it** opens command-line tools in a terminal.                                                                                                                                                          |
| **Home Manager as a system module**           | Changing Home Manager settings on NixOS or nix-darwin. See [Home Manager](home-manager.md).                                                                                                                 |

## Getting the package

The repository is a flake whose output `packages.<system>.default` is the app.
On Linux it contains three programs:

- `drakeflake`: the app, with a menu entry called **DrakeFlake** (category
  System),
- `drakeflake-tray`: the small tray icon that checks for updates (see
  [Flake inputs](inputs.md#update-notifications-in-the-tray)),
- `drakeflake-helper`: the helper that does the few things that need root. It
  is not put on your `PATH`. The app runs it through `pkexec` (or `run0`,
  `sudo`, or on a Mac the administrator dialog).

On macOS the tray is the menu bar icon `drakeflake-tray-macos`, and
`DrakeFlake.app` and `drakeflake-elevate` are added. See
[nix-darwin systems](darwin.md#installing-on-a-mac).

### Install it system-wide

1. Add the repository as an input of your system flake. Use the URL you got
   this repository from.
2. Add the package to `environment.systemPackages`.
3. Rebuild your system.

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    drakeflake.url = "github:dr460nf1r3/drakeflake";
  };

  outputs = { nixpkgs, drakeflake, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        (
          { pkgs, ... }:
          {
            environment.systemPackages = [
              drakeflake.packages.${pkgs.stdenv.hostPlatform.system}.default
            ];
          }
        )
      ];
    };
  };
}
```

Installing it system-wide also installs the app's polkit actions
(`share/polkit-1/actions/org.dr460nf1r3.drakeflake.policy`). They give the
password prompts a clear description. Saving files asks for your password every
time. See [Privacy and security](security.md).

### Try it from a checkout

To run the app without installing it, run this in a checkout of the repository:

```sh
nix run .
```

The app works the same way. Because its polkit actions are not installed, the
password prompts are the generic ones of `pkexec`.

If the app does not start, or the first start takes long, see
[Troubleshooting](troubleshooting.md).

## Related

- [First start: the setup assistant](setup.md)
- [nix-darwin systems](darwin.md)
- [Privacy and security](security.md)
