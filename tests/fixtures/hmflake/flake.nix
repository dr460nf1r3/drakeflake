{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/e94cb152ed51bd6e24eb4a41f1460252beb52cd2";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    impermanence = {
      url = "github:nix-community/impermanence";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.home-manager.follows = "home-manager";
    };
  };
  outputs =
    {
      nixpkgs,
      home-manager,
      impermanence,
      ...
    }:
    {
      nixosConfigurations.hmhost = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ./configuration.nix
          home-manager.nixosModules.home-manager
          impermanence.nixosModules.impermanence
          {
            users.users.alice.isNormalUser = true;
            home-manager.users.alice = {
              home.stateVersion = "26.11";
              programs.git.enable = true;
              home.persistence."/persist".directories = [ "Documents" ];
            };
          }
        ];
      };
    };
}
