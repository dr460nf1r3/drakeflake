# Nothing runs at boot: a switch would stop an in-guest suite service mid-run.
{
  pkgs,
  src,
  drakeflake,
}:
let
  guest = pkgs.callPackage ./guest.nix { inherit src drakeflake; };
  inherit (guest) fixture;

  debugModule = {
    virtualisation.graphics = true;
    # Only the default user-mode NIC: the standalone VM script has no VDE socket.
    virtualisation.vlans = [ ];
    virtualisation.qemu.options = [ "-display none -vnc 127.0.0.1:2" ];
    services.getty.autologinUser = "test";

    systemd.services.debug-fixtures = {
      description = "Stage the test fixtures for manual runs";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      script = ''
        cp -r ${fixture "testflake"} /tmp/vm-flake
        cp -r ${fixture "lifecycleflake"} /tmp/vm-lifecycle-flake
        chmod -R u+w /tmp/vm-flake /tmp/vm-lifecycle-flake
        mkdir -p /tmp/vm-itest/cache /tmp/vm-itest/config /tmp/vm-itest/state
        chown -R test /tmp/vm-flake /tmp/vm-lifecycle-flake /tmp/vm-itest
      '';
    };
  };

  debugTest = pkgs.testers.runNixOSTest {
    name = "drakeflake-debug-vm-lifecycle";
    nodes.machine = {
      imports = [
        guest.machineModule
        debugModule
      ];
    };
    testScript = "";
  };
in
debugTest.nodes.machine.system.build.vm
