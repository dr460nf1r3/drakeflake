{ lib, pkgs, ... }:
{
  imports = lib.optional (builtins.pathExists ./drakeflake.nix) ./drakeflake.nix;

  networking.hostName = "darwinhost";
  environment.systemPackages = [ pkgs.git ];
  system.stateVersion = 5;
}
