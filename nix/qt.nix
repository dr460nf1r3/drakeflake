# cxx/qt-build-utils resolve every Qt module relative to a single prefix, but nixpkgs splits modules across store paths.
{ pkgs }:
let
  inherit (pkgs) lib;
in
rec {
  qtMerged = pkgs.buildEnv {
    name = "qt-merged";
    paths = [
      pkgs.qt6.qtbase
      pkgs.qt6.qtdeclarative
      pkgs.qt6.qtshadertools
      # WebP and kin (the macOS style's spinner): built-in formats
      # alone do not cover them, on either platform.
      pkgs.qt6.qtimageformats
    ]
    # Qt6Gui.prl asks for -lGLX -lOpenGL on Linux. The dev shell's cc
    # wrapper finds them via NIX_LDFLAGS. Builds outside it (IDEs) only
    # get this -L path. macOS has OpenGL as a system framework.
    ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.libGL;
    pathsToLink = [
      "/lib"
      "/include"
      "/share"
      "/mkspecs"
      "/metatypes"
      "/modules"
      "/libexec"
    ];
    # Darwin keeps headers in `<Module>.framework/Headers`, and the -F
    # flag qt-build-utils wants never reaches the compiler through cc.
    # Mirror them as `include/<Module>`.
    postBuild = lib.optionalString pkgs.stdenv.hostPlatform.isDarwin ''
      for framework in "$out"/lib/*.framework; do
        module=$(basename "$framework" .framework)
        if [ ! -e "$out/include/$module" ]; then
          ln -s "../lib/$module.framework/Headers" "$out/include/$module"
        fi
      done
    '';
  };

  qmakeWrapped =
    let
      queryScript = which: ''
        if { [ "$1" = "-query" ] || [ "$1" = "--query" ]; } && [ -n "''${2:-}" ]; then
          case "$2" in
            QT_INSTALL_PREFIX | QT_HOST_PREFIX) echo "${qtMerged}"; exit 0 ;;
            QT_INSTALL_LIBS | QT_HOST_LIBS) echo "${qtMerged}/lib"; exit 0 ;;
            QT_INSTALL_HEADERS | QT_HOST_HEADERS) echo "${qtMerged}/include"; exit 0 ;;
            QT_INSTALL_BINS | QT_HOST_BINS*) echo "''${0%/*}"; exit 0 ;;
            QT_INSTALL_LIBEXECS | QT_HOST_LIBEXECS*) echo "${pkgs.qt6.qtbase}/libexec"; exit 0 ;;
            QT_INSTALL_PLUGINS) echo "${qtMerged}/lib/qt-6/plugins"; exit 0 ;;
            QT_INSTALL_QML) echo "${qtMerged}/lib/qt-6/qml"; exit 0 ;;
          esac
        fi
        exec ${which} "$@"
      '';
    in
    pkgs.runCommand "qt-query-wrapped" { } ''
      mkdir -p $out/bin
      printf '#!${pkgs.stdenv.shell}\n%s\n' ${pkgs.lib.escapeShellArg (queryScript "${pkgs.qt6.qtbase}/bin/qmake")} > $out/bin/qmake
      printf '#!${pkgs.stdenv.shell}\n%s\n' ${pkgs.lib.escapeShellArg (queryScript "${pkgs.qt6.qtbase}/bin/qmake")} > $out/bin/qmake6
      printf '#!${pkgs.stdenv.shell}\n%s\n' ${pkgs.lib.escapeShellArg (queryScript "${pkgs.qt6.qtbase}/bin/qtpaths")} > $out/bin/qtpaths
      printf '#!${pkgs.stdenv.shell}\n%s\n' ${pkgs.lib.escapeShellArg (queryScript "${pkgs.qt6.qtbase}/bin/qtpaths")} > $out/bin/qtpaths6
      chmod +x $out/bin/qmake $out/bin/qmake6 $out/bin/qtpaths $out/bin/qtpaths6
    '';

  # qmllint. Kirigami's wrapper has no QML files itself: use the real one.
  qmlModules = [
    pkgs.kdePackages.kirigami.unwrapped
    pkgs.kdePackages.qqc2-desktop-style
    pkgs.kdePackages.syntax-highlighting
  ];
}
