{
  pkgs,
  self,
  src,
  rustPkg,
  rustToolchain,
  treefmt,
  qmlModules,
}:
{
  formatting = treefmt.config.build.check self;

  clippy = rustPkg.check;

  tests = pkgs.runCommand "check-tests" { passthru.report = rustPkg.testReport; } ''
    status=$(cat ${rustPkg.testReport}/status)
    if [ "$status" != 0 ]; then
      echo "the tests failed (nextest exit $status): nix log ${rustPkg.testReport}" >&2
      exit 1
    fi
    touch $out
  '';

  file-size = pkgs.runCommand "check-file-size" { } ''
    cp -r ${src}/crates crates
    mkdir -p src
    cp -r ${src}/src/qml src/qml
    cp ${src}/.file-size-allow .file-size-allow
    ${pkgs.bash}/bin/bash ${src}/scripts/check-file-size.sh .
    touch $out
  '';

  lint = pkgs.runCommand "check-lint" { } ''
    cp -r ${src}/crates crates
    mkdir -p src
    cp -r ${src}/src/qml src/qml
    ${rustPkg.lint}/bin/drakeflake-lint crates src/qml
    touch $out
  '';

  cargo-deny =
    pkgs.runCommand "check-cargo-deny"
      {
        nativeBuildInputs = [
          rustToolchain
          pkgs.cargo-deny
        ];
      }
      ''
        export HOME=$TMPDIR CARGO_HOME=$TMPDIR/cargo
        mkdir -p "$CARGO_HOME"
        cat > "$CARGO_HOME/config.toml" <<EOF
        [source.crates-io]
        replace-with = "vendored-sources"
        [source.vendored-sources]
        directory = "${rustPkg.package.cargoDeps}"
        EOF
        cp ${src}/Cargo.toml Cargo.toml
        cp ${src}/Cargo.lock Cargo.lock
        cp ${src}/deny.toml deny.toml
        cp -r ${src}/crates crates
        chmod -R u+w .
        cargo deny --offline check licenses bans sources
        touch $out
      '';

  # Like scripts/qmllint.sh, with explicit import paths: the sandbox has
  # no system QML. Kirigami's wrapper only propagates the real module.
  qml-lint = pkgs.runCommand "check-qml-lint" { nativeBuildInputs = [ pkgs.qt6.qtdeclarative ]; } ''
    export HOME=$TMPDIR
    cd ${src}/src/qml
    qmllint --max-warnings 0 \
      -I ${pkgs.qt6.qtdeclarative}/lib/qt-6/qml \
      ${pkgs.lib.concatMapStringsSep " " (pkg: "-I ${pkg}/lib/qt-6/qml") qmlModules} \
      -I . \
      $(find . -name '*.qml')
    touch $out
  '';

  # Real fonts: text measurement needs them even offscreen.
  qml-test = pkgs.runCommand "check-qml-test" { nativeBuildInputs = [ pkgs.qt6.qtdeclarative ]; } ''
    export HOME=$TMPDIR QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QT_FORCE_STDERR_LOGGING=1
    export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; }}
    export QT_PLUGIN_PATH=${pkgs.qt6.qtbase}/lib/qt-6/plugins
    export QML2_IMPORT_PATH=${pkgs.qt6.qtdeclarative}/lib/qt-6/qml:${
      pkgs.lib.concatMapStringsSep ":" (pkg: "${pkg}/lib/qt-6/qml") qmlModules
    }
    mkdir -p $out
    qmltestrunner -input ${src}/tests -o $out/junit.xml,junitxml -o -,txt
  '';
}
