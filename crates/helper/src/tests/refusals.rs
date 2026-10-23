use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use drakeflake_plan::{HEADER, WritePlan};

use crate::activate::{activate, checked_system};
use crate::apply::apply_json;
use crate::error::HelperError;
use crate::lock::check_lock;

/// Resolved because the helper's checks see resolved paths.
fn flake() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap().join("flake");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    (dir, root)
}

pub(super) fn plan_json(write: &[(&Path, &str)], remove: &[&Path]) -> String {
    serde_json::to_string(&WritePlan {
        write: write
            .iter()
            .map(|(path, text)| (path.to_path_buf(), (*text).to_owned()))
            .collect(),
        remove: remove.iter().map(|path| path.to_path_buf()).collect(),
    })
    .unwrap()
}

#[test]
fn nothing_is_removed_unless_every_removal_is_acceptable() {
    let (_dir, flake) = flake();
    let generated = flake.join("drakeflake.nix");
    let foreign = flake.join("configuration.nix");
    std::fs::write(&generated, HEADER).unwrap();
    std::fs::write(&foreign, "{ ... }: { }").unwrap();

    let result = apply_json(&plan_json(&[], &[&generated, &foreign]));

    assert!(matches!(result, Err(HelperError::Refused(_))), "{result:?}");
    assert!(generated.exists() && foreign.exists());
}

#[test]
fn foreign_content_and_targets_outside_a_flake_are_refused() {
    let (_dir, flake) = flake();
    let outside = flake.parent().unwrap().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    symlink(&outside, flake.join("linked")).unwrap();

    let through_link = flake.join("linked/drakeflake.nix");
    let not_ours = flake.join("new.nix");
    for json in [
        plan_json(&[(&through_link, HEADER)], &[]),
        plan_json(&[(&not_ours, "{ evil = true; }")], &[]),
        plan_json(&[(Path::new("/etc/drakeflake.nix"), HEADER)], &[]),
    ] {
        let result = apply_json(&json);
        assert!(
            matches!(result, Err(HelperError::Refused(_))),
            "{json}: {result:?}"
        );
    }

    assert!(!outside.join("drakeflake.nix").exists());
    assert!(!not_ours.exists());
}

#[test]
fn malformed_plans_are_refused() {
    assert!(matches!(
        apply_json("{\"write\": "),
        Err(HelperError::InvalidPlan(_))
    ));
    assert!(matches!(
        apply_json(&plan_json(&[(Path::new("relative.nix"), HEADER)], &[])),
        Err(HelperError::ApplyPlan(_))
    ));
}

#[test]
fn a_planted_symlink_is_not_removed_or_followed() {
    let (_dir, flake) = flake();
    let victim = flake.parent().unwrap().join("victim.nix");
    std::fs::write(&victim, HEADER).unwrap();
    let link = flake.join("drakeflake.nix");
    symlink(&victim, &link).unwrap();

    let result = apply_json(&plan_json(&[], &[&link]));

    assert!(matches!(result, Err(HelperError::Refused(_))), "{result:?}");
    assert!(link.is_symlink() && victim.exists());
}

#[test]
fn only_the_three_activation_modes_are_accepted() {
    for mode in ["Switch", "", "dry-activate", "switch ", "boot\0"] {
        assert!(
            matches!(
                activate(mode, Path::new("/etc")),
                Err(HelperError::UnknownMode(_))
            ),
            "{mode:?}"
        );
    }

    for mode in ["switch", "boot", "test"] {
        assert!(matches!(
            activate(mode, Path::new("/etc")),
            Err(HelperError::NotInStore(_))
        ));
    }
}

#[test]
fn a_system_outside_the_store_is_never_activated() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("system");
    std::fs::create_dir_all(fake.join("bin")).unwrap();
    std::fs::write(fake.join("bin/switch-to-configuration"), "#!/bin/sh\n").unwrap();
    std::fs::write(fake.join("nixos-version"), "26.05").unwrap();

    assert!(matches!(
        checked_system(&fake),
        Err(HelperError::NotInStore(_))
    ));
    assert!(matches!(
        activate("test", &fake),
        Err(HelperError::NotInStore(_))
    ));
}

#[test]
fn a_link_into_the_store_is_resolved_and_must_be_a_system() {
    let Some(store_dir) = std::fs::read_dir("/nix/store").ok().and_then(|entries| {
        entries
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.is_dir() && !path.join("nixos-version").exists())
    }) else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("result");
    symlink(&store_dir, &link).unwrap();

    assert!(matches!(
        checked_system(&link),
        Err(HelperError::NotASystem(resolved)) if resolved.starts_with("/nix/store/")
    ));
}

#[test]
fn only_small_regular_files_are_read_for_the_user() {
    let (_dir, flake) = flake();
    let lock = flake.join("new.lock");
    std::fs::write(&lock, "0123456789").unwrap();

    crate::secure::read_user_file(&lock, 9, None).unwrap_err();
    crate::secure::read_user_file(&flake, 1024, None).unwrap_err();
    crate::secure::read_user_file(Path::new("new.lock"), 1024, None).unwrap_err();

    // Root's own files are never read, so the size boundary shows only as a user.
    if !rustix::process::getuid().is_root() {
        assert_eq!(
            crate::secure::read_user_file(&lock, 10, None).unwrap(),
            b"0123456789"
        );
    }
}

#[test]
fn the_write_walk_refuses_parent_references() {
    let (_dir, flake) = flake();
    std::fs::create_dir_all(flake.join("sub")).unwrap();

    let error = crate::secure::write_file(&flake.join("sub/../x.nix"), b"x").unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!flake.join("x.nix").exists());
}

#[test]
fn lock_files_need_version_root_and_nodes_of_the_right_kinds() {
    for (lock, valid) in [
        (r#"{"version": 7, "root": "root", "nodes": {}}"#, true),
        (r#"{"version": "7", "root": "root", "nodes": {}}"#, false),
        (r#"{"version": -1, "root": "root", "nodes": {}}"#, false),
        (r#"{"version": 7, "root": 1, "nodes": {}}"#, false),
        (r#"{"version": 7, "root": "root"}"#, false),
        ("[]", false),
        ("", false),
    ] {
        assert_eq!(check_lock(lock.as_bytes()).is_ok(), valid, "{lock}");
    }
}
