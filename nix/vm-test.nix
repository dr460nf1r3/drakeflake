{
  pkgs,
  src,
  drakeflake,
  functionIndex,
  integrationTests,
}:
let
  guest = pkgs.callPackage ./guest.nix { inherit src drakeflake; };
  inherit (guest) fixture;
in
pkgs.testers.runNixOSTest {
  name = "drakeflake-vm-selftest";

  nodes.machine = guest.machineModule;

  testScript = ''
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        "cp -r ${fixture "testflake"} /tmp/vm-flake"
        + " && cp -r ${fixture "hmflake"} /tmp/vm-hm-flake"
        + " && cp -r ${fixture "lifecycleflake"} /tmp/vm-lifecycle-flake"
        + " && chmod -R u+w /tmp/vm-flake /tmp/vm-hm-flake /tmp/vm-lifecycle-flake"
        + " && mkdir -p /tmp/vm-run/shots /tmp/vm-run/config /tmp/vm-run/cache /tmp/vm-run/state"
        + " /tmp/vm-itest/cache /tmp/vm-itest/config /tmp/vm-itest/state"
        + " && chown -R test /tmp/vm-flake /tmp/vm-hm-flake /tmp/vm-lifecycle-flake"
        + " /tmp/vm-itest /tmp/vm-run"
    )

    print(machine.succeed("timeout 20 runuser -u test -- bash -c 'echo SU-OK && id'"))

    # The pseudo-terminal test only runs online: rebuilding hello needs its build-time closure, which is not baked in.
    online = "ONLINE" in machine.succeed(
        "curl -s -o /dev/null --max-time 10 https://api.github.com/zen && echo ONLINE || echo OFFLINE"
    )
    offline_skips = (
        ""
        if online
        else " --skip vm_flake_update_copy --skip vm_lock_check_runs"
        + " --skip live_check_updates --skip live_home_manager"
    )
    nom_env = (
        " DRAKEFLAKE_NOM=${pkgs.nix-output-monitor}/bin/nom"
        + " DRAKEFLAKE_NOM_INSTALLABLE=/tmp/vm-flake#nixosConfigurations.testhost.pkgs.hello"
        if online
        else ""
    )

    status, _ = machine.execute(
        "runuser -u test -- bash -c '"
        + "{ export HOME=/home/test XDG_CACHE_HOME=/tmp/vm-itest/cache XDG_CONFIG_HOME=/tmp/vm-itest/config"
        + " XDG_STATE_HOME=/tmp/vm-itest/state"
        + " DRAKEFLAKE_TEST_FLAKE=/tmp/vm-flake DRAKEFLAKE_TEST_HOST=testhost"
        + " DRAKEFLAKE_TEST_HM_FLAKE=/tmp/vm-hm-flake DRAKEFLAKE_TEST_HM_HOST=hmhost"
        + " DRAKEFLAKE_TEST_LIFECYCLE_FLAKE=/tmp/vm-lifecycle-flake"
        + " DRAKEFLAKE_VM_GUEST=1 DRAKEFLAKE_LOCK_DIR=/tmp/vm-flake"
        + nom_env
        + " failed=0"
        + " && for test in ${integrationTests}/bin/*;"
        + " do \"$test\" --ignored --test-threads=2 --skip lifecycle --skip cleanup"
        + offline_skips
        + " || failed=1; done"
        + " ; echo \"$(date +%T) safe tests done, failures: $failed\"; exit $failed"
        + " ; } > /tmp/vm-itest/out.log 2>&1"
        + "'",
        timeout=7200,
    )
    print(machine.succeed("cat /tmp/vm-itest/out.log"))
    assert status == 0, "the integration suite failed"

    machine.execute(
        "runuser -u test -- bash -c '"
        + "export HOME=/home/test XDG_CONFIG_HOME=/tmp/vm-run/config XDG_CACHE_HOME=/tmp/vm-run/cache XDG_STATE_HOME=/tmp/vm-run/state"
        + " DRAKEFLAKE_FLAKE=/tmp/vm-flake DRAKEFLAKE_HOST=testhost"
        + " DRAKEFLAKE_FUNCTION_INDEX=${functionIndex}/functions.json"
        + " DRAKEFLAKE_SELFTEST_DIR=/tmp/vm-run/shots DRAKEFLAKE_SELFTEST_APPLY=build DRAKEFLAKE_SELFTEST_SUITE=plain"
        + (
            " DRAKEFLAKE_SELFTEST_OFFLINE=1"
            if not online
            else ""
        )
        + " QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QT_FORCE_STDERR_LOGGING=1"
        + " && timeout 5400 ${drakeflake}/bin/drakeflake > /tmp/vm-run/out.log 2>&1"
        + "'",
        timeout=5500,
    )

    print(machine.succeed("grep -a -m10 'SELFTEST FAIL' /tmp/vm-run/out.log || true"))
    print(machine.succeed("grep -a -m2 -A40 'derivations will be built' /tmp/vm-run/out.log || true"))
    print(machine.succeed("grep -a -m10 'unable to download' /tmp/vm-run/out.log || true"))
    print(machine.succeed("cat /tmp/vm-flake/drakeflake.nix /tmp/vm-flake/drakeflake/*.nix /tmp/vm-flake/drakeflake/*/*.nix 2>/dev/null || true"))
    print(machine.succeed("tail -c 20000 /tmp/vm-run/out.log"))
    machine.succeed("grep -q 'SELFTEST PASS' /tmp/vm-run/out.log")
  '';
}
