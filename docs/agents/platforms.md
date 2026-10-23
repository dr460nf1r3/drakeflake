# NixOS and macOS

## Capabilities

- Hosts are `nixosConfigurations` or `darwinConfigurations` (`SystemKind`).
- `hub::Capabilities` (`systemModel.capabilities`) decides what a host allows.
  A host of the other platform is save-only. Darwin has no `test` or `boot`.
  Images need NixOS on NixOS.
- Gate features on capabilities, never on `kind`.
- Recommendations must stay valid per kind. nix-darwin rejects
  `nix.settings.auto-optimise-store`.

## Build and packaging

- Linux-only calls sit behind `cfg(target_os = "linux")`.
- The flake builds `aarch64-darwin` only.
- The darwin package ships `drakeflake-tray-macos`, the helper,
  `drakeflake-elevate`, `DrakeFlake.app`, and Breeze as Qt resources
  (`nix/icon-resources.nix`). Qt on macOS searches only `:/icons`, so the icons
  are registered at start.
- No darwin builder is available here. Check non-Qt crates with
  `cargo clippy --target aarch64-apple-darwin -p <crate>`.

## macOS look

- Linux keeps Breeze. The style on macOS is `DrakeFlakeMac`
  (`src/qml/style/DrakeFlakeMac`).
- Kirigami uses a style's `Theme.qml` only from the first import path that holds
  `org/kde/kirigami`. `macos::add_kirigami_theme` mirrors that folder into
  `~/.cache/drakeflake/qml` and puts it first on `QML_IMPORT_PATH`.
- Colours come from the `DrakeFlake.Native` singleton `SystemTheme`
  (`macos.mm`). The stub in `src/qml/DrakeFlake/Native` exists for qmllint.

## Linux blur

- Blur works on Wayland only, through `ext-background-effect-v1` in
  `blur/wayland.rs`. X11 gets no blur.
- Test nested in niri 26.04 or KWin 6.7 with OpenGL.
