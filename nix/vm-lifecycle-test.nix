{
  pkgs,
  src,
  drakeflake,
  integrationTests,
}:
let
  guest = pkgs.callPackage ./guest.nix { inherit src drakeflake; };
  inherit (guest) fixture;

  # Alphabetical node order fixes the test-VLAN addresses, so renaming
  # either node breaks the fixtures keeping the target reachable.
  targetExtra = {
    services.openssh.enable = true;
    networking.firewall.allowedTCPPorts = [ 22 ];
  };
  runnerExtra = {
    environment.systemPackages = [ pkgs.openssh ];
  };

  # Absolute store paths: after a switch /run/current-system no longer
  # provides these tools, and flakes need the fixtures' nix settings.
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
  testEnv =
    "HOME=/home/test XDG_CACHE_HOME=/tmp/vm-itest/cache XDG_CONFIG_HOME=/tmp/vm-itest/config"
    + " XDG_STATE_HOME=/tmp/vm-itest/state"
    + " DRAKEFLAKE_TEST_FLAKE=/tmp/vm-flake DRAKEFLAKE_TEST_HOST=testhost"
    + " DRAKEFLAKE_TEST_LIFECYCLE_FLAKE=/tmp/vm-lifecycle-flake"
    + " DRAKEFLAKE_TEST_LIFECYCLE_STATE=/tmp/vm-itest/lifecycle-state"
    + " DRAKEFLAKE_VM_GUEST=1"
    + " PATH=${pkgs.lib.makeBinPath testTools}:$PATH";

  # Pinned against the cleanup's garbage collection, which would
  # otherwise whiteout these shared host paths in the guest overlay.
  gcroots = [ integrationTests ] ++ testTools;
  gcrootsArg = pkgs.lib.concatMapStringsSep " " toString gcroots;
in
pkgs.testers.runNixOSTest {
  name = "drakeflake-vm-lifecycle";

  nodes.runner = {
    imports = [
      guest.machineModule
      runnerExtra
    ];
  };

  nodes.target = {
    imports = [
      guest.machineModule
      targetExtra
      # Registered at boot for their references, which the symlinks below need.
      { virtualisation.additionalPaths = gcroots; }
    ];
  };

  testScript = ''
    import base64
    import datetime

    runner.wait_for_unit("multi-user.target")
    target.wait_for_unit("multi-user.target")

    target_ip = target.succeed("ip -4 -brief addr show eth1 | head -n 1").split()[2].split("/")[0]
    print("target ip: " + target_ip)
    assert target_ip == "192.168.1.2", "unexpected target address: " + target_ip

    target.succeed(
        "cp -r ${fixture "testflake"} /tmp/vm-flake"
        + " && cp -r ${fixture "lifecycleflake"} /tmp/vm-lifecycle-flake"
        + " && chmod -R u+w /tmp/vm-flake /tmp/vm-lifecycle-flake"
        + " && mkdir -p /tmp/vm-itest/cache /tmp/vm-itest/config /tmp/vm-itest/state"
        + " && chown -R test /tmp/vm-flake /tmp/vm-lifecycle-flake /tmp/vm-itest"
    )

    print(target.succeed("timeout 20 runuser -u test -- bash -c 'echo SU-OK && id'"))

    # A password prompt here would hang the suite instead of failing it.
    print(target.succeed("ls -la /run/wrappers/bin/"))
    _, pkexec_out = target.execute(
        "timeout 10 runuser -u test -- /run/wrappers/bin/pkexec true && echo PKEXEC:0 || echo PKEXEC:$?",
        timeout=datetime.timedelta(seconds=60),
    )
    print(pkexec_out)
    _, sudo_out = target.execute(
        "timeout 10 runuser -u test -- /run/wrappers/bin/sudo -n true && echo SUDO:0 || echo SUDO:$?",
        timeout=datetime.timedelta(seconds=60),
    )
    print(sudo_out)

    # Before any switch: afterwards only SSH reaches the target.
    runner.succeed("ssh-keygen -t ed25519 -N \"\" -f /tmp/ssh-key")
    pubkey = runner.succeed("cat /tmp/ssh-key.pub").strip()
    target.succeed(
        "mkdir -p /home/test/.ssh && chmod 700 /home/test/.ssh && echo \"" + pubkey + "\" > /home/test/.ssh/authorized_keys && chmod 600 /home/test/.ssh/authorized_keys && chown -R test:test /home/test/.ssh"
    )

    def on_target(command, timeout):
        return runner.execute(
            "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o BatchMode=yes -i /tmp/ssh-key test@" + target_ip + " " + command,
            timeout=timeout,
        )

    _, smoke_out = on_target("echo SSH-OK", datetime.timedelta(seconds=60))
    print(smoke_out)
    assert "SSH-OK" in smoke_out, "cannot reach the target over SSH: " + smoke_out

    # Long composed commands break the serial transport. Short chunk
    # appends and bare-word ssh calls pass through.
    step_script = (
        "#!/bin/sh\n"
        + "export ${testEnv}\n"
        + "if [ \"$1\" = gcroot ]; then n=0; for path in ${gcrootsArg}; do n=$((n + 1)); sudo -n ln -sf \"$path\" \"/nix/var/nix/gcroots/auto/drakeflake-suite-$n\" || exit 1; done; exit 0; fi\n"
        + "filter=$1\n"
        + "failed=0\n"
        + "for test in ${integrationTests}/bin/*; do \"$test\" --ignored --test-threads=1 \"$filter\" || failed=1; done\n"
        + "echo \"STEP:$failed\"\n"
        + "exit \"$failed\"\n"
    )
    encoded = base64.b64encode(step_script.encode()).decode()
    target.succeed("rm -f /tmp/step.b64")
    for offset in range(0, len(encoded), 400):
        target.succeed("echo " + encoded[offset:offset + 400] + " >> /tmp/step.b64")
    target.succeed("base64 -d /tmp/step.b64 > /tmp/vm-itest/run-step.sh && chmod +x /tmp/vm-itest/run-step.sh")

    def run_step(filter, timeout):
        status, out = on_target("/tmp/vm-itest/run-step.sh " + filter, timeout)
        print(out)
        return status

    def wait_for_target_ssh():
        for _wait in range(0, 20):
            status, _ = on_target("echo UP", datetime.timedelta(seconds=60))
            if status == 0:
                return
            runner.succeed("sleep 15", timeout=datetime.timedelta(seconds=60))
        raise Exception("the target never came back over SSH")

    # A dropped connection (255) only means the switch took the channel
    # down. The generations afterwards say whether the switch worked.
    def run_switch_step(filter):
        try:
            status, out = on_target(
                "/tmp/vm-itest/run-step.sh " + filter,
                datetime.timedelta(seconds=3000),
            )
        except Exception as err:
            print("switch step dropped: " + str(err))
            return 255
        print(out)
        return status

    print("step 1: first switch")
    assert run_switch_step("vm_lifecycle_step1_switch") in [0, 255]
    wait_for_target_ssh()

    print("step 2: compare and record")
    assert run_step("vm_lifecycle_step2_compare", datetime.timedelta(seconds=900)) == 0

    print("step 3: second switch")
    assert run_switch_step("vm_lifecycle_step3_second_switch") in [0, 255]
    wait_for_target_ssh()

    print("step 4: verify")
    assert run_step("vm_lifecycle_step4_verify", datetime.timedelta(seconds=300)) == 0

    print("step 5: rollback")
    assert run_switch_step("vm_lifecycle_step5_rollback") in [0, 255]
    wait_for_target_ssh()

    print("step 6: verify and compare")
    assert run_step("vm_lifecycle_step6_verify", datetime.timedelta(seconds=900)) == 0

    # Last: it must see the generations the lifecycle left.
    print("gcroot the suite tools")
    assert run_step("gcroot", datetime.timedelta(seconds=600)) == 0
    print("cleanup")
    assert run_step("vm_cleanup_keep_all", datetime.timedelta(seconds=2400)) == 0

    # The teardown syncs every machine it sees up, including the dead
    # target channel: power the target off host-side first.
    try:
        target.crash()
    except Exception as err:
        print("target crash: " + str(err))
  '';
}
