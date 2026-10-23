let
  flake = builtins.getFlake __FLAKE__;
  system = flake.__ATTR__.__HOST__.extendModules {
    modules = [
      (
        { lib, options, ... }:
        {
          # Only NixOS has `documentation.nixos`. nix-darwin just needs `enable`.
          documentation = {
            enable = lib.mkForce true;
          }
          // lib.optionalAttrs (options.documentation ? nixos) {
            nixos.enable = lib.mkForce true;
            nixos.includeAllModules = lib.mkForce true;
          };
        }
      )
    ];
  };
in
system.config.system.build.manual.optionsJSON
