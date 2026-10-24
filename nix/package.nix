{
  lib,
  stdenv,
  rustPlatform,
  qt6,
  kdePackages,
  makeDesktopItem,
  copyDesktopItems,
  qmakeWrapped,
  qtMerged,
  clippy,
  makeWrapper,
  curl,
  git,
  nix,
  pkg-config,
  wayland,
  iconResources,
  macosIcons,
  functionIndex,
}:

let
  inherit (stdenv.hostPlatform) isLinux;

  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  tagline = "NixOS, tamed: search and change the options, packages and inputs of a NixOS or nix-darwin system flake";

  appCrates = import ./app-crates.nix { inherit isLinux; };

  infoPlist = builtins.toFile "Info.plist" ''
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
      <key>CFBundleName</key><string>DrakeFlake</string>
      <key>CFBundleDisplayName</key><string>DrakeFlake</string>
      <key>CFBundleIdentifier</key><string>org.dr460nf1r3.drakeflake</string>
      <key>CFBundleExecutable</key><string>DrakeFlake</string>
      <key>CFBundleIconFile</key><string>drakeflake</string>
      <key>CFBundlePackageType</key><string>APPL</string>
      <key>CFBundleShortVersionString</key><string>${version}</string>
      <key>CFBundleVersion</key><string>${version}</string>
      <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
      <key>NSHighResolutionCapable</key><true/>
    </dict>
    </plist>
  '';

  common =
    {
      withClippy,
      withTests ? false,
    }:
    rustPlatform.buildRustPackage {
      pname = "drakeflake";
      inherit version;

      src = ../.;

      cargoLock = {
        lockFile = ../Cargo.lock;
      };

      # A plain `cargo test` in this virtual workspace builds every member, so the tests need the same flags.
      cargoBuildFlags = appCrates;
      cargoTestFlags = appCrates;

      doCheck = withTests;
      useNextest = true;
      cargoCheckType = "debug";
      env.NEXTEST_PROFILE = "ci";

      # qtbase's setup hook re-exports QMAKE at its own qmake, so it is exported again in preBuild.
      preBuild = ''
        export QMAKE=${qmakeWrapped}/bin/qmake6
      '';

      nativeBuildInputs = [
        copyDesktopItems
        makeWrapper
        qt6.wrapQtAppsHook
        qmakeWrapped
        pkg-config
        qt6.qttools
      ]
      ++ lib.optionals withClippy [ clippy ];

      buildInputs = [
        qtMerged
        kdePackages.kirigami
        kdePackages.syntax-highlighting
        qt6.qtsvg
      ]
      ++ lib.optionals isLinux [
        kdePackages.qqc2-breeze-style
        wayland
      ];

      qtWrapperArgs = [
        "--set DRAKEFLAKE_QML_DIR ${placeholder "out"}/share/drakeflake/qml"
        "--set DRAKEFLAKE_DOCS_DIR ${placeholder "out"}/share/drakeflake/docs"
        "--set QT_QUICK_CONTROLS_STYLE ${if isLinux then "org.kde.breeze" else "DrakeFlakeMac"}"
        "--suffix XDG_DATA_DIRS : ${kdePackages.breeze-icons}/share"

        # Appended, so the user's own versions win.
        "--suffix PATH : ${
          lib.makeBinPath [
            curl
            git
          ]
        }"
      ]

      # Qt on macOS ignores XDG_DATA_DIRS: Breeze comes as Qt resources.
      ++ lib.optional (!isLinux) "--set DRAKEFLAKE_ICON_RCC_DIR ${iconResources}";

      postInstall = ''
        mkdir -p $out/share/drakeflake
        cp -r ${../src/qml} $out/share/drakeflake/qml
        chmod -R u+w $out/share/drakeflake/qml

        cp -r ${../docs/src} $out/share/drakeflake/docs
        chmod -R u+w $out/share/drakeflake/docs
        # The copied symlink points outside docs/src, so it is replaced by the generated file.
        rm $out/share/drakeflake/docs/changelog.md
        bash ${../scripts/changelog-docs.sh} ${../.} $out/share/drakeflake/docs/changelog.md

        bash ${../scripts/i18n-release.sh} $out/share/drakeflake/qml/i18n

        ln -s ${functionIndex}/functions.json $out/share/drakeflake/functions.json

        # Run via pkexec, never directly: keep it off PATH.
        mkdir -p $out/libexec
        mv $out/bin/drakeflake-helper $out/libexec/drakeflake-helper
      ''
      + lib.optionalString isLinux ''
        mkdir -p $out/share/polkit-1/actions
        substitute ${../data/org.dr460nf1r3.drakeflake.policy} \
          $out/share/polkit-1/actions/org.dr460nf1r3.drakeflake.policy \
          --replace-fail @helper@ $out/libexec/drakeflake-helper
      ''
      + ''
        # Qt derives the Wayland app_id from the binary name and a wrapped `.drakeflake-wrapped` would give none.
        mv $out/bin/drakeflake $out/libexec/drakeflake

        install -Dm644 ${../data/icons/drakeflake.svg} \
          $out/share/icons/hicolor/scalable/apps/drakeflake.svg
        for size in 16x16 22x22 24x24 32x32; do
          install -Dm644 ${../data/icons/drakeflake-small.svg} \
            $out/share/icons/hicolor/$size/apps/drakeflake.svg
        done
      ''
      + lib.optionalString isLinux ''
        for icon in drakeflake-tray drakeflake-tray-update; do
          install -Dm644 ${../data}/icons/$icon.svg \
            $out/share/icons/hicolor/scalable/status/$icon.svg
        done
      '';

      # Only the GUI gets the Qt environment wrapper. The helper (run via
      # pkexec) must stay the plain binary its polkit action points to.
      dontWrapQtApps = true;
      postFixup = ''
        makeQtWrapper $out/libexec/drakeflake $out/bin/drakeflake
      ''
      + lib.optionalString (!isLinux) ''
        install -Dm755 ${../data/macos/drakeflake-elevate.sh} $out/libexec/drakeflake-elevate

        app=$out/Applications/DrakeFlake.app/Contents
        install -Dm644 ${infoPlist} $app/Info.plist
        install -Dm644 ${macosIcons}/drakeflake.icns $app/Resources/drakeflake.icns

        mkdir -p $app/MacOS

        printf '#!/bin/sh\nexec %s "$@"\n' $out/bin/drakeflake > $app/MacOS/DrakeFlake
        chmod 755 $app/MacOS/DrakeFlake

        mkdir -p $out/share/drakeflake
        cp -r ${macosIcons}/tray $out/share/drakeflake/tray
      ''
      + ''
        wrapProgram $out/bin/${
          if isLinux then "drakeflake-tray" else "drakeflake-tray-macos"
        } --suffix PATH : ${
          lib.makeBinPath [
            curl
            git
          ]
        }
      '';

      desktopItems = lib.optionals isLinux [
        (makeDesktopItem {
          name = "drakeflake";
          desktopName = "DrakeFlake";
          exec = "drakeflake";
          icon = "drakeflake";
          comment = tagline;
          categories = [ "System" ];
        })
      ];

      meta = {
        description = tagline;
        mainProgram = "drakeflake";
        homepage = "https://github.com/dr460nf1r3/drakeflake";
        license = lib.licenses.gpl3Plus;
        platforms = lib.platforms.linux ++ lib.platforms.darwin;
      };
    };
in
{
  package = common { withClippy = false; };

  lint = rustPlatform.buildRustPackage {
    pname = "drakeflake-lint";
    inherit version;

    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;

    cargoBuildFlags = [
      "-p"
      "drakeflake-lint"
    ];
    cargoTestFlags = [
      "-p"
      "drakeflake-lint"
    ];

    meta.mainProgram = "drakeflake-lint";
  };

  check =
    let
      base = common { withClippy = true; };
    in
    base.overrideAttrs (_old: {
      name = "drakeflake-check";

      buildPhase = ''
        runHook preBuild
        cargo clippy --offline --workspace --all-targets -- -D warnings
        runHook postBuild
      '';

      installPhase = ''
        mkdir -p $out
        touch $out/done
      '';

      postFixup = "";
    });

  testReport =
    (common {
      withClippy = false;
      withTests = true;
    }).overrideAttrs
      (old: {
        name = "drakeflake-test-report";

        # The tests shell out to git, nix and nix-instantiate, which are otherwise only wrapped onto the installed PATH.
        nativeBuildInputs = old.nativeBuildInputs ++ [
          git
          nix
        ];

        buildPhase = ''
          runHook preBuild
          runHook postBuild
        '';

        # Not the nextest hook, which stops the build when a test fails.
        checkPhase = ''
          runHook preCheck

          # The sandbox has no writable /nix/var, and nix creates its profile
          # directory there when it evaluates.
          export NIX_STATE_DIR="$TMPDIR/nix-state"
          mkdir -p "$NIX_STATE_DIR"

          # nix caches under $HOME, which is not writable in the sandbox.
          export HOME="$TMPDIR/home"
          mkdir -p "$HOME"
          status=0

          cargo nextest run --offline --show-progress=none -j "$NIX_BUILD_CORES" \
            ${lib.escapeShellArgs appCrates} || status=$?
          runHook postCheck
        '';

        installPhase = ''
          mkdir -p $out
          echo "''${status:-1}" > $out/status
          if [ -e target/nextest/ci/junit.xml ]; then
            cp target/nextest/ci/junit.xml $out/
          fi
        '';

        postFixup = "";
      });
}
