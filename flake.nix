{
  description = "DrakeFlake: NixOS, tamed. Search and change the options, packages and inputs of a NixOS or nix-darwin system flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      treefmt-nix,
      git-hooks,
    }:
    let
      inherit (nixpkgs) lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        # nixpkgs unstable no longer supports x86_64-darwin.
        "aarch64-darwin"
      ];
      forAllSystems = f: lib.genAttrs systems (system: f (perSystem system));

      perSystem =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              rust-overlay.overlays.default
            ]
            ++ lib.optional (lib.hasSuffix "-darwin" system) (import ./nix/darwin-kde.nix);
          };

          rustToolchain = pkgs.callPackage ./nix/rust-toolchain.nix { };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          treefmt = import ./nix/formatter.nix { inherit pkgs treefmt-nix rustToolchain; };
          qt = import ./nix/qt.nix { inherit pkgs; };
          packages = import ./nix/packages.nix {
            inherit pkgs rustPlatform rustToolchain;
            inherit (qt) qmakeWrapped qtMerged;
            nixpkgsSource = nixpkgs;
            src = ./.;
          };
          devshell = import ./nix/devshell.nix {
            inherit pkgs rustToolchain;
            inherit (qt) qmakeWrapped qmlModules;
            inherit (packages) iconResources;
            treefmtWrapper = treefmt.config.build.wrapper;
            preCommitHook = preCommit.shellHook;
          };
          preCommit = import ./nix/git-hooks.nix {
            inherit pkgs treefmt;
            gitHooksLib = git-hooks.lib;
            src = ./.;
          };
        in
        {
          inherit pkgs treefmt;
          inherit (packages)
            rustPkg
            functionIndex
            iconResources
            integrationTests
            vmTest
            ;
          inherit (devshell) devShellWith;
          inherit (qt) qmlModules;
          checks = import ./nix/checks.nix {
            inherit pkgs self;
            inherit (packages) rustPkg;
            inherit rustToolchain treefmt;
            inherit (qt) qmlModules;
            src = ./.;
          };
        };
    in
    {
      packages = forAllSystems (
        {
          rustPkg,
          iconResources,
          integrationTests,
          functionIndex,
          vmTest,
          ...
        }:
        {
          default = rustPkg.package;
          icon-resources = iconResources;
          integration-tests = integrationTests;
          function-index = functionIndex;
        }
        // vmTest
      );

      formatter = forAllSystems ({ treefmt, ... }: treefmt.config.build.wrapper);

      devShells = forAllSystems (
        { pkgs, devShellWith, ... }:
        {
          default = devShellWith pkgs.stdenv.cc "";
          clang = devShellWith pkgs.clang ''
            export CC=clang CXX=clang++
            export CARGO_TARGET_${pkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_LINKER=clang
          '';
        }
      );

      checks = forAllSystems ({ checks, ... }: checks);
    };
}
