{
  stdenv,
  rustPlatform,
  nix,
  nixpkgsSource,
}:
let
  generator = rustPlatform.buildRustPackage {
    pname = "drakeflake-function-index-generator";
    version = "0.1.0";
    src = ../..;
    cargoLock.lockFile = ../../Cargo.lock;
    cargoBuildFlags = [
      "-p"
      "drakeflake-functions"
      "--bin"
      "drakeflake-function-index"
    ];
    cargoTestFlags = [
      "-p"
      "drakeflake-functions"
    ];
    meta.mainProgram = "drakeflake-function-index";
  };
in
stdenv.mkDerivation {
  pname = "drakeflake-function-index";
  version = nixpkgsSource.shortRev or "0";

  dontUnpack = true;
  nativeBuildInputs = [
    nix
    generator
  ];

  # Evaluation only: `--store dummy://` needs no writable store, so this
  # runs in the sandbox.
  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR NIX_STATE_DIR=$TMPDIR/nix-state NIX_CONF_DIR=$TMPDIR
    bash ${./.}/build.sh ${nixpkgsSource} ${stdenv.hostPlatform.system} \
      "${nixpkgsSource.rev or ""}" ${toString (nixpkgsSource.lastModified or 0)} \
      $out/functions.json drakeflake-function-index
    runHook postBuild
  '';

  dontInstall = true;

  passthru = { inherit generator; };
}
