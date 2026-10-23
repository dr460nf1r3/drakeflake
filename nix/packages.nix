{
  pkgs,
  rustPlatform,
  rustToolchain,
  qmakeWrapped,
  qtMerged,
  nixpkgsSource,
  src,
}:
let
  functionIndex = pkgs.callPackage ./function-index {
    inherit rustPlatform nixpkgsSource;
  };

  iconResources = pkgs.callPackage ./icon-resources.nix { };
  macosIcons = pkgs.callPackage ./macos-icons.nix { };

  rustPkg = pkgs.callPackage ./package.nix {
    inherit
      qmakeWrapped
      qtMerged
      rustPlatform
      iconResources
      macosIcons
      functionIndex
      ;
    clippy = rustToolchain;
  };

  integrationTests = pkgs.callPackage ./integration-tests.nix {
    inherit
      rustPlatform
      qmakeWrapped
      qtMerged
      src
      ;
  };

  # with KVM. The switch replaces the running system, so the
  # system-changing tests get a guest of their own.
  vmTest = pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
    vm-test = pkgs.callPackage ./vm-test.nix {
      inherit src functionIndex integrationTests;
      drakeflake = rustPkg.package;
    };
    vm-lifecycle-test = pkgs.callPackage ./vm-lifecycle-test.nix {
      inherit src integrationTests;
      drakeflake = rustPkg.package;
    };
    debug-vm = pkgs.callPackage ./debug-vm.nix {
      inherit src integrationTests;
      drakeflake = rustPkg.package;
    };
    debug-vm-lifecycle = pkgs.callPackage ./debug-vm-lifecycle.nix {
      inherit src;
      drakeflake = rustPkg.package;
    };
  };
in
{
  inherit
    functionIndex
    iconResources
    integrationTests
    macosIcons
    rustPkg
    vmTest
    ;
}
