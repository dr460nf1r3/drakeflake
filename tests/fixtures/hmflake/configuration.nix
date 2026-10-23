{ lib, pkgs, ... }:
{
  imports = lib.optional (builtins.pathExists ./drakeflake.nix) ./drakeflake.nix;

  boot.loader.grub.device = "nodev";
  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };
  networking.hostName = "hmhost";
  services.openssh.enable = true;
  environment.systemPackages = [ pkgs.git ];
  system.stateVersion = "26.11";

  fileSystems."/persist" = {
    device = "/dev/disk/by-label/persist";
    fsType = "ext4";
    neededForBoot = true;
  };
  environment.persistence."/persist" = {
    directories = [ "/var/log" ];
    files = [ "/etc/machine-id" ];
  };
}
