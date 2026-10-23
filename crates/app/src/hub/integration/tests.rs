use std::time::Duration;

use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_flake::Nix;
use drakeflake_plan::ManagedState;
use drakeflake_render::OptionName;

use super::super::cleanup::Optimise;
use super::super::generations::GenerationAction;
use super::super::{Mode, test_support, try_package};
use super::{
    copied_hub_target, current_generation, hub_target, lifecycle_target, record_generation,
    recorded_generation, running_generation, vm_guest, wait_for_finish, wait_for_formats,
};

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_build_tool_hello() {
    let Some(target) = hub_target() else {
        return;
    };

    let paths = try_package::build_tool(&Nix::system(), &target, "hello").unwrap();
    assert!(!paths.is_empty(), "no output paths for hello");
    assert!(
        paths.iter().all(|path| path.join("bin/hello").is_file()),
        "{paths:?}"
    );
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_image_formats_hub() {
    let Some(target) = hub_target() else {
        return;
    };

    let hub = test_support::hub_with(Some(target.clone()));

    assert!(
        hub.capabilities().images,
        "images need a NixOS host on NixOS"
    );

    drakeflake_flake::eval_value(
        &Nix::system(),
        &target,
        &OptionName::from("networking.hostName"),
    )
    .unwrap();

    hub.load_image_formats(target.host.to_string());
    let formats = wait_for_formats(&hub, Duration::from_mins(10))
        .expect("no image formats arrived")
        .unwrap();

    assert!(!formats.is_empty(), "no image formats for {}", target.host);
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_apply_save_restore() {
    let Some((target, _copy)) = copied_hub_target() else {
        return;
    };

    let hub = test_support::hub_with(Some(target));

    hub.set_layout(LayoutConfig {
        mode: LayoutMode::Single,
        ..LayoutConfig::default()
    });

    let mut first = ManagedState::default();
    first.packages.insert("hello".into());
    hub.apply(first, Mode::Save).unwrap();
    let first_apply =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the first apply never finished");

    assert!(
        first_apply.ok,
        "the first apply failed: {}",
        first_apply.console.join("\n")
    );

    assert!(
        first_apply.backup.is_none(),
        "nothing to back up on a fresh flake"
    );

    let managed = hub.managed_file().unwrap();
    let saved = std::fs::read_to_string(&managed).unwrap();
    assert!(saved.contains("hello"), "{saved}");

    let mut second = ManagedState::default();
    second.packages.insert("cowsay".into());
    hub.apply(second, Mode::Save).unwrap();
    let second_apply =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the second apply never finished");

    assert!(
        second_apply.ok,
        "the second apply failed: {}",
        second_apply.console.join("\n")
    );

    let backup = second_apply
        .backup
        .expect("the second save backs up the first");

    hub.restore(backup).unwrap();
    let restored =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the restore never finished");

    assert!(
        restored.ok,
        "the restore failed: {}",
        restored.console.join("\n")
    );

    assert_eq!(std::fs::read_to_string(&managed).unwrap(), saved);
}

#[test]
#[ignore = "needs NixOS generations"]
fn vm_generations_compare() {
    let hub = test_support::hub_with(None);
    if !hub.capabilities().generations {
        return;
    }

    let Some(current) = current_generation() else {
        return;
    };

    hub.generation_action(current, GenerationAction::Compare)
        .unwrap();
    let compared =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the compare never finished");

    assert!(
        compared.ok,
        "the compare failed: {}",
        compared.console.join("\n")
    );
}

#[test]
#[ignore = "first switch of the test VM (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step1_switch() {
    if !vm_guest() {
        return;
    }

    let Some(target) = hub_target() else {
        return;
    };

    let hub = test_support::hub_with(Some(target));

    assert!(
        current_generation().is_none(),
        "the test VM already has generations"
    );

    hub.rebuild_only(Mode::Switch).unwrap();

    if let Some(switched) = wait_for_finish(&hub, Duration::from_mins(45)) {
        assert!(
            switched.ok,
            "the switch failed: {}",
            switched.console.join("\n")
        );
    }
}

#[test]
#[ignore = "compares the first generation (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step2_compare() {
    if !vm_guest() {
        return;
    }

    let hub = test_support::hub_with(None);

    let first = current_generation().expect("no generation after the switch");

    hub.generation_action(first, GenerationAction::Compare)
        .unwrap();
    let compared =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the compare never finished");

    assert!(
        compared.ok,
        "the compare failed: {}",
        compared.console.join("\n")
    );

    record_generation("first", first);
}

#[test]
#[ignore = "second switch of the test VM (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step3_second_switch() {
    if !vm_guest() {
        return;
    }

    let Some(second_target) = lifecycle_target() else {
        return;
    };

    let hub = test_support::hub_with(Some(second_target));

    hub.rebuild_only(Mode::Switch).unwrap();

    if let Some(switched) = wait_for_finish(&hub, Duration::from_mins(45)) {
        assert!(
            switched.ok,
            "the second switch failed: {}",
            switched.console.join("\n")
        );
    }
}

#[test]
#[ignore = "checks the second generation (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step4_verify() {
    if !vm_guest() {
        return;
    }

    let first = recorded_generation("first").expect("no first generation recorded");
    let second = current_generation().expect("no generation after the second switch");
    assert_ne!(second, first, "the second switch changed nothing");

    record_generation("second", second);
}

#[test]
#[ignore = "rolls back the test VM (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step5_rollback() {
    if !vm_guest() {
        return;
    }

    let first = recorded_generation("first").expect("no first generation recorded");

    let hub = test_support::hub_with(None);

    hub.generation_action(first, GenerationAction::Switch)
        .unwrap();

    if let Some(rolled_back) = wait_for_finish(&hub, Duration::from_mins(20)) {
        assert!(
            rolled_back.ok,
            "the rollback failed: {}",
            rolled_back.console.join("\n")
        );
    }
}

#[test]
#[ignore = "checks the rollback (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_lifecycle_step6_verify() {
    if !vm_guest() {
        return;
    }

    let first = recorded_generation("first").expect("no first generation recorded");

    let hub = test_support::hub_with(None);

    assert_eq!(
        current_generation().expect("no current generation after the rollback"),
        first,
        "the rollback did not return to generation {first}"
    );

    assert_eq!(
        running_generation(),
        Some(first),
        "the rollback did not activate generation {first}"
    );

    hub.generation_action(first, GenerationAction::Compare)
        .unwrap();
    let compared =
        wait_for_finish(&hub, Duration::from_mins(10)).expect("the compare never finished");

    assert!(
        compared.ok,
        "the compare failed: {}",
        compared.console.join("\n")
    );
}

#[test]
#[ignore = "cleans the test VM (needs DRAKEFLAKE_VM_GUEST=1)"]
fn vm_cleanup_keep_all() {
    if !vm_guest() {
        return;
    }

    let hub = test_support::hub_with(None);

    let before = current_generation().expect("no current generation in the VM");
    hub.clean_up(100, 36500, Optimise::Skip).unwrap();
    let cleaned =
        wait_for_finish(&hub, Duration::from_mins(30)).expect("the cleanup never finished");

    assert!(
        cleaned.ok,
        "the cleanup failed: {}",
        cleaned.console.join("\n")
    );

    assert_eq!(
        current_generation().expect("no current generation after the cleanup"),
        before,
        "the cleanup removed the current generation"
    );
}
