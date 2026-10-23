{
  pkgs,
  treefmt-nix,
  rustToolchain,
}:
treefmt-nix.lib.evalModule pkgs {
  projectRootFile = "flake.nix";
  programs = {
    actionlint.enable = true;
    deadnix.enable = true;
    nixfmt.enable = true;
    prettier.enable = true;
    qmlformat = {
      enable = true;
      package = pkgs.qt6.qtdeclarative;
    };
    rustfmt = {
      enable = true;
      package = rustToolchain;
    };
    shellcheck.enable = true;
    shfmt.enable = true;
    statix.enable = true;
  };
  settings = {
    global.excludes = [
      "target/**"
      "*.lock"
      "crates/flake/src/expr/*.nix"
    ];
    formatter = {
      shellcheck.excludes = [ ".envrc" ];
      qmlformat.options = [
        "--group-attributes-together"
        "--objects-spacing"
        "--functions-spacing"
      ];
      # QML JavaScript uses `.pragma library` and `.import` lines, which prettier
      # cannot parse as plain JS, so it is left out.
      prettier.excludes = [
        "src/qml/**/*.js"
        "i18n/*.ts"
      ];
    };
  };
}
