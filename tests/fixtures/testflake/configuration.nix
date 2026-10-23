{ lib, pkgs, ... }:
{
  imports = lib.optional (builtins.pathExists ./drakeflake.nix) ./drakeflake.nix;

  boot.loader.grub.device = "nodev";
  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };
  # No registry pin or NIX_PATH entries: the VM guest runs fully offline,
  # and nothing under test resolves bare `nixpkgs` references.
  nixpkgs.flake.setFlakeRegistry = lib.mkForce false;
  nixpkgs.flake.setNixPath = lib.mkForce false;
  networking.hostName = "testhost";
  services.openssh.enable = true;
  environment.systemPackages = [ pkgs.git ];

  users.users.test = {
    isNormalUser = true;
    createHome = true;
    home = "/home/test";
    group = "test";
    extraGroups = [ "wheel" ];
  };
  users.groups.test = { };
  security.polkit.enable = true;
  security.polkit.enablePkexecWrapper = true;
  security.polkit.extraConfig = ''
    polkit.addRule(function(action, subject) {
      return polkit.Result.YES;
    });
  '';
  security.sudo.extraRules = [
    {
      users = [ "root" ];
      commands = [
        {
          command = "ALL";
          options = [ "NOPASSWD" ];
        }
      ];
    }
    {
      groups = [ "wheel" ];
      commands = [
        {
          command = "ALL";
          options = [ "NOPASSWD" ];
        }
      ];
    }
  ];
  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  # Keep the runner's SSH address across switches (alphabetical node
  # order fixes this guest at 192.168.1.2).
  networking.interfaces.eth1.ipv4.addresses = [
    {
      address = "192.168.1.2";
      prefixLength = 24;
    }
  ];
  networking.firewall.allowedTCPPorts = [ 22 ];

  # Unmounting /nix/store aborts the activation, and fstab lines compare
  # as raw strings: these mirror the guest byte for byte.
  fileSystems."/nix/.ro-store" = {
    device = "nix-store";
    fsType = "virtiofs";
    neededForBoot = true;
    options = [
      "x-initrd.mount"
      "ro"
    ];
  };
  fileSystems."/nix/.rw-store" = {
    device = "tmpfs";
    fsType = "tmpfs";
    neededForBoot = true;
    options = [
      "x-initrd.mount"
      "mode=0755"
    ];
  };
  fileSystems."/nix/store" = {
    overlay = {
      lowerdir = [ "/nix/.ro-store" ];
      upperdir = "/nix/.rw-store/upper";
      workdir = "/nix/.rw-store/work";
    };
    depends = [
      "/nix/.ro-store"
      "/nix/.rw-store/upper"
      "/nix/.rw-store/work"
    ];
    options = [
      "lowerdir=/sysroot/nix/.ro-store"
      "upperdir=/sysroot/nix/.rw-store/upper"
      "workdir=/sysroot/nix/.rw-store/work"
      "x-initrd.mount"
      "x-systemd.requires-mounts-for=/sysroot/nix/.ro-store"
      "x-systemd.requires-mounts-for=/sysroot/nix/.rw-store/upper"
      "x-systemd.requires-mounts-for=/sysroot/nix/.rw-store/work"
    ];
  };
  system.stateVersion = "26.11";
}
