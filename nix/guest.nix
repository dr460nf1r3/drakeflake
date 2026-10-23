{
  pkgs,
  src,
  drakeflake,
}:
let
  fixture =
    name:
    pkgs.runCommand "vm-fixture-${name}" { } ''
      cp -r ${src}/tests/fixtures/${name} $out
      chmod -R u+w $out
      substituteInPlace $out/flake.nix \
        --replace-fail x86_64-linux ${pkgs.stdenv.hostPlatform.system}
    '';

  vmSeed = pkgs.callPackage ./vm-seed.nix { inherit src; };
in
{
  inherit fixture vmSeed;

  machineModule =
    { lib, pkgs, ... }:
    {
      virtualisation.memorySize = lib.mkDefault 8192;
      virtualisation.cores = lib.mkDefault 4;
      virtualisation.diskSize = lib.mkDefault (60 * 1024);
      virtualisation.graphics = lib.mkDefault false;
      nix.settings.substituters = pkgs.lib.mkForce [ ];
      virtualisation.additionalPaths = [
        vmSeed.testhost
        vmSeed.selftesthost
        vmSeed.lifecyclehost
        vmSeed.manualOptions
        vmSeed.hello
        vmSeed.xterm
        vmSeed.cowsay
        vmSeed.nixpkgsSrc
      ];
      nix.settings.experimental-features = [
        "nix-command"
        "flakes"
      ];
      environment.systemPackages = [
        drakeflake
        pkgs.curl
        pkgs.git
        pkgs.nh
        pkgs.nix-output-monitor
      ];
      # `nh` refuses to run as root. Elevation must never prompt, as there is no polkit agent or terminal.
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
      fonts.packages = [ pkgs.dejavu_fonts ];
      # QEMU's `-cpu max` exposes features the host kernel does not run and the guest oopsed. `-cpu host` wins as the last option.
      virtualisation.qemu.options = [ "-cpu host" ];
      # Kernel messages on hvc0 corrupt the driver's base64 replies, so the serial port keeps them.
      boot.kernelParams = [ "console=ttyS0,115200n8" ];
      system.stateVersion = "26.11";
    };
}
