{
  pkgs,
  src,
  drakeflake,
  integrationTests,
}:
let
  guest = pkgs.callPackage ./guest.nix { inherit src drakeflake; };
  inherit (guest) fixture;

  testTools = [
    pkgs.nix
    pkgs.git
    pkgs.nh
    pkgs.curl
    pkgs.coreutils
    pkgs.bash
    pkgs.gnugrep
    pkgs.gnused
  ];

  debugModule = {
    virtualisation.graphics = true;
    # Only the default user-mode NIC: the standalone VM script has no VDE socket.
    virtualisation.vlans = [ ];
    virtualisation.qemu.options = [ "-display none -vnc 127.0.0.1:1" ];
    services.getty.autologinUser = "test";

    systemd.services.drakeflake-itest = {
      description = "DrakeFlake safe integration suite";
      after = [
        "debug-fixtures.service"
        "network-online.target"
      ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];
      path = testTools ++ [ pkgs.util-linux ];
      serviceConfig = {
        Type = "oneshot";
        TimeoutStartSec = "3h";
        StandardOutput = "tty";
        TTYPath = "/dev/ttyS0";
      };
      script = ''
        say() { echo "DRAKEFLAKE $(date +%T) $*" > /dev/ttyS0; }
        say "suite start"
        online=OFFLINE
        curl -s -o /dev/null --max-time 10 https://api.github.com/zen && online=ONLINE
        say "network: $online"
        runuser -u test -- bash -c '
          export HOME=/home/test XDG_CACHE_HOME=/tmp/vm-itest/cache \
            XDG_CONFIG_HOME=/tmp/vm-itest/config XDG_STATE_HOME=/tmp/vm-itest/state \
            DRAKEFLAKE_TEST_FLAKE=/tmp/vm-flake DRAKEFLAKE_TEST_HOST=testhost \
            DRAKEFLAKE_TEST_HM_FLAKE=/tmp/vm-hm-flake DRAKEFLAKE_TEST_HM_HOST=hmhost \
            DRAKEFLAKE_TEST_LIFECYCLE_FLAKE=/tmp/vm-lifecycle-flake \
            DRAKEFLAKE_VM_GUEST=1 DRAKEFLAKE_LOCK_DIR=/tmp/vm-flake \
            PATH=${pkgs.lib.makeBinPath testTools}:$PATH
          cd /tmp
          for test in ${integrationTests}/bin/*; do
            "$test" --ignored --test-threads=2 --skip lifecycle --skip cleanup > /tmp/vm-itest/safe-$(basename "$test").log 2>&1 || echo "FAILED $test" >> /tmp/vm-itest/failures
          done
        '
        say "safe tests done"
        say "failures: $({ wc -l < /tmp/vm-itest/failures; } 2>/dev/null || echo 0)"
        for log in /tmp/vm-itest/safe-*.log; do
          say "--- $log"
          tail -n 40 "$log" | sed 's/\x1b\[[0-9;]*m//g' > /dev/ttyS0
        done
        say "suite end"
        systemctl poweroff
      '';
    };

    systemd.services.debug-fixtures = {
      description = "Stage the test fixtures for manual runs";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      script = ''
        cp -r ${fixture "testflake"} /tmp/vm-flake
        cp -r ${fixture "hmflake"} /tmp/vm-hm-flake
        cp -r ${fixture "lifecycleflake"} /tmp/vm-lifecycle-flake
        chmod -R u+w /tmp/vm-flake /tmp/vm-hm-flake /tmp/vm-lifecycle-flake
        mkdir -p /tmp/vm-run/shots /tmp/vm-run/config /tmp/vm-run/cache /tmp/vm-run/state
        mkdir -p /tmp/vm-itest/cache /tmp/vm-itest/config /tmp/vm-itest/state
        chown -R test /tmp/vm-flake /tmp/vm-hm-flake /tmp/vm-lifecycle-flake /tmp/vm-itest /tmp/vm-run
      '';
    };
  };

  # A node of a test that never runs: the test framework evaluates its
  # nodes in one consistent nixpkgs tree, so the VM module merges cleanly.
  debugTest = pkgs.testers.runNixOSTest {
    name = "drakeflake-debug-vm";
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
