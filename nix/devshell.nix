# clang is selected through CC/CXX and the linker, since clangStdenv would put its build-side gcc first on PATH.
{
  pkgs,
  rustToolchain,
  qmakeWrapped,
  qmlModules,
  iconResources,
  treefmtWrapper,
  preCommitHook,
}:
let
  inherit (pkgs) lib;

  devLoop = pkgs.writeShellApplication {
    name = "start-dev";
    runtimeInputs = [ pkgs.git ];
    text = ''
      root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
      exec "$root/scripts/dev.sh" "$@"
    '';
  };

  devShellWith =
    compiler: compilerEnv:
    pkgs.mkShell {
      packages = [
        compiler
        pkgs.nixfmt
        pkgs.statix
        pkgs.deadnix
        pkgs.qt6.qtbase
        pkgs.qt6.qtdeclarative
        pkgs.kdePackages.kirigami
        pkgs.kdePackages.syntax-highlighting
        pkgs.pkg-config
        pkgs.nh
        pkgs.nix
        rustToolchain
        pkgs.cargo-deny
        pkgs.cargo-nextest
        pkgs.cargo-llvm-cov
        pkgs.watchexec
        devLoop
        pkgs.qt6.qttools
        qmakeWrapped
        treefmtWrapper
      ]
      ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
        pkgs.kdePackages.qqc2-breeze-style
        pkgs.wayland
      ];

      shellHook = ''
        ${compilerEnv}
        ${preCommitHook}
        export QT_QUICK_CONTROLS_STYLE=${
          if pkgs.stdenv.hostPlatform.isLinux then "org.kde.breeze" else "DrakeFlakeMac"
        }
        export PATH="${qmakeWrapped}/bin:$PATH"
        export QMAKE="${qmakeWrapped}/bin/qmake6"
        # Qt's own QML dir first: without it QtQuick.Controls resolves
        # from the embedded resources, whose plugins cannot load.
        export QML2_IMPORT_PATH="${
          "${pkgs.qt6.qtdeclarative}/lib/qt-6/qml:"
          + lib.concatMapStringsSep ":" (pkg: "${pkg}/lib/qt-6/qml") qmlModules
        }:${lib.optionalString pkgs.stdenv.hostPlatform.isLinux "${pkgs.kdePackages.qqc2-breeze-style}/lib/qt-6/qml:"}$QML2_IMPORT_PATH"
        export QT_PLUGIN_PATH="${pkgs.qt6.qtbase}/lib/qt-6/plugins:${pkgs.qt6.qtdeclarative}/lib/qt-6/plugins:${pkgs.qt6.qtimageformats}/lib/qt-6/plugins:${pkgs.qt6.qtsvg}/lib/qt-6/plugins:$QT_PLUGIN_PATH"
        ${lib.optionalString pkgs.stdenv.hostPlatform.isDarwin "export DRAKEFLAKE_ICON_RCC_DIR=${iconResources}"}
        export QMLLINT_IMPORT_PATH="${pkgs.qt6.qtdeclarative}/lib/qt-6/qml:${
          lib.concatMapStringsSep ":" (pkg: "${pkg}/lib/qt-6/qml") qmlModules
        }"
      '';
    };
in
{
  inherit devShellWith;
}
