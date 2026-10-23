{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/e94cb152ed51bd6e24eb4a41f1460252beb52cd2";
  outputs =
    { nixpkgs, ... }:
    {
      nixosConfigurations.lifecyclehost = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [ ./configuration.nix ];
      };
    };
}
