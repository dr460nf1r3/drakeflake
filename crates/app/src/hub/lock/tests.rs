use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use drakeflake_flake::Target;

use super::super::test_support::{console_lines, hub_with, local_target};
use super::*;

const LOCK: &str = r#"{
  "nodes": {
    "nixpkgs": {
      "locked": { "type": "github", "owner": "NixOS", "repo": "nixpkgs", "rev": "aaaa", "lastModified": 1, "narHash": "sha256-x" },
      "original": { "type": "github", "owner": "NixOS", "repo": "nixpkgs" }
    },
    "root": { "inputs": { "nixpkgs": "nixpkgs" } }
  },
  "root": "root",
  "version": 7
}"#;

#[test]
fn messages_name_the_inputs_or_all() {
    assert_eq!(describe_inputs(&[]), "all inputs");
    assert_eq!(
        describe_inputs(&["nixpkgs".to_owned(), "home-manager".to_owned()]),
        "nixpkgs, home-manager"
    );
}

#[test]
fn updates_need_valid_names_and_a_local_flake() {
    let flake = tempfile::tempdir().unwrap();
    let local = hub_with(Some(local_target(flake.path())));

    for bad in ["", "--commit-lock-file", "a b", "../x", "x/y"] {
        assert_eq!(
            local
                .update_lock(vec!["nixpkgs".to_owned(), bad.to_owned()])
                .map_err(|error| error.to_string()),
            Err(format!("\"{bad}\" is not an input name"))
        );
    }

    assert!(
        !local.operations().is_running(),
        "a rejected update started anyway"
    );

    let not_local = "Flake inputs can only be updated for a local flake directory.".to_owned();
    let remote = hub_with(Some(Target {
        flake: "github:owner/repo".to_owned(),
        ..local_target(flake.path())
    }));

    assert_eq!(
        hub_with(None)
            .update_lock(Vec::new())
            .map_err(|error| error.to_string()),
        Err(not_local.clone())
    );
    assert_eq!(
        remote
            .update_lock(Vec::new())
            .map_err(|error| error.to_string()),
        Err(not_local)
    );
}

#[test]
fn only_local_lock_files_are_read() {
    let flake = tempfile::tempdir().unwrap();
    let dir = flake.path().to_string_lossy().into_owned();

    read_flake_lock("github:owner/repo").unwrap_err();
    assert!(
        read_flake_lock(&dir).is_err(),
        "a missing flake.lock is no error"
    );
    fs::write(flake.path().join("flake.lock"), LOCK).unwrap();
    let inputs = read_flake_lock(&format!("path:{dir}")).unwrap();
    assert_eq!(
        inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        vec!["nixpkgs"]
    );
}

#[test]
fn a_read_only_lock_is_updated_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("flake.lock");

    assert!(lock_writable(dir.path()), "a new lock can be created here");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        0,
        "the write probe was left behind"
    );

    fs::write(&lock, LOCK).unwrap();
    assert!(lock_writable(dir.path()));

    if rustix::process::getuid().is_root() {
        return;
    }

    fs::set_permissions(&lock, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(!lock_writable(dir.path()));

    let sealed = tempfile::tempdir().unwrap();
    fs::set_permissions(sealed.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let creatable = lock_writable(sealed.path());
    fs::set_permissions(sealed.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!creatable, "no lock can be created in a read-only folder");
}

#[test]
fn inputs_that_kept_their_old_revision_fail_the_update() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub_with(None);
    let run = Exclusive::detached(&hub);
    let expecting = |rev: &str| UpdatePlan {
        expected: vec![("nixpkgs".to_owned(), rev.to_owned())],
        ..UpdatePlan::default()
    };

    assert!(
        run.verify_update(dir.path(), &expecting("aaaa"))
            .is_success()
    );
    let lines = console_lines(&hub);
    assert!(
        lines.len() == 1
            && lines
                .iter()
                .all(|line| line.starts_with("Cannot read the updated flake.lock")),
        "{lines:?}"
    );

    fs::write(dir.path().join("flake.lock"), LOCK).unwrap();
    assert!(
        run.verify_update(dir.path(), &expecting("aaaa"))
            .is_success()
    );
    assert_eq!(console_lines(&hub), Vec::<String>::new());

    assert!(
        !run.verify_update(dir.path(), &expecting("bbbb"))
            .is_success()
    );
    let stale = console_lines(&hub);
    assert!(
        stale.len() == 1
            && stale
                .iter()
                .all(|line| line.starts_with("Not updated: nixpkgs.")),
        "{stale:?}"
    );
}
