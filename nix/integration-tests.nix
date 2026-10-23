# The VM guest has no toolchain, so this builds the test binaries offline for it to execute.
{
  pkgs,
  rustPlatform,
  qmakeWrapped,
  qtMerged,
  src,
}:
let
  inherit (pkgs.stdenv.hostPlatform) isLinux;
  inherit (pkgs) lib;

  appCrates = import ./app-crates.nix { inherit isLinux; };
in
rustPlatform.buildRustPackage {
  pname = "drakeflake-integration-tests";
  version = "0.1.0";

  inherit src;

  cargoLock = {
    lockFile = src + "/Cargo.lock";
  };

  doCheck = false;

  dontWrapQtApps = true;

  nativeBuildInputs = [
    qmakeWrapped
    pkgs.pkg-config
  ];

  buildInputs = [
    qtMerged
    pkgs.kdePackages.kirigami
    pkgs.kdePackages.kwindowsystem
    pkgs.kdePackages.syntax-highlighting
    pkgs.qt6.qtsvg
  ]
  ++ lib.optional isLinux pkgs.kdePackages.qqc2-breeze-style;

  preBuild = ''
    export QMAKE=${qmakeWrapped}/bin/qmake6
  '';

  buildPhase = ''
    runHook preBuild
    cargo test --offline --frozen --no-run ${lib.escapeShellArgs appCrates}
    runHook postBuild
  '';

  installPhase = ''
    mkdir -p $out/bin
    while IFS= read -r exe; do
      case "$(basename "$exe")" in
        build_script_* | *.d | *.rlib | *.rmeta | *.so) continue ;;
      esac
      cp "$exe" $out/bin/
    done < <(find target -type f -executable)
  '';
}
