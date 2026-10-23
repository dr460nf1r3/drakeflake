mod refusals;
mod swap;

use std::os::unix::fs::{PermissionsExt as _, symlink};

use drakeflake_plan::exit;

use super::*;
use crate::activate::{activate, checked_system};
use crate::exec::run;
use crate::lock::{check_lock, checked_flake_dir, install_lock};

const LOCK: &str = r#"{ "nodes": { "root": { } }, "root": "root", "version": 7 }"#;

#[test]
fn rejects_non_systems() {
    checked_system(Path::new("/etc")).unwrap_err();
    checked_system(Path::new("/nonexistent")).unwrap_err();
    assert!(matches!(
        activate("dance", Path::new("/etc")),
        Err(HelperError::UnknownMode(_))
    ));
}

#[test]
fn flake_dirs_are_checked() {
    checked_flake_dir(Path::new("relative")).unwrap_err();
    checked_flake_dir(Path::new("/etc/ssh")).unwrap_err();
    checked_flake_dir(Path::new("/nonexistent")).unwrap_err();

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    checked_flake_dir(dir.path()).unwrap();
}

#[test]
fn only_lock_files_are_installed() {
    check_lock(LOCK.as_bytes()).unwrap();
    assert!(check_lock(b"root:x:0:0::/root:/bin/sh").is_err());
    assert!(check_lock(br#"{ "nodes": [] }"#).is_err());

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let new_lock = dir.path().join("new.lock");
    std::fs::write(&new_lock, LOCK).unwrap();

    // A symlink as the source is not followed (it could point at a root file).
    let link = dir.path().join("link.lock");
    symlink(&new_lock, &link).unwrap();
    assert!(matches!(
        install_lock(dir.path(), &link, Some(rustix::process::getuid().as_raw())),
        Err(HelperError::ReadLock { path: _, source: _ })
    ));

    assert!(matches!(
        install_lock(dir.path(), &new_lock, None),
        Err(HelperError::UnknownCaller)
    ));

    // As root the lock file goes to `nobody` first, which a user namespace mapping only root cannot do.
    let mut owner = rustix::process::getuid().as_raw();
    if owner == 0 {
        owner = 0xFFFE;
        if let Err(err) = std::os::unix::fs::chown(&new_lock, Some(owner), None) {
            eprintln!("skipping the install as root: cannot hand the file to nobody: {err}");
            return;
        }
    }

    assert!(matches!(
        install_lock(dir.path(), &new_lock, Some(owner + 1)),
        Err(HelperError::ReadLock { path: _, source: _ })
    ));
    install_lock(dir.path(), &new_lock, Some(owner)).unwrap();
    let installed = dir.path().join("flake.lock");
    assert_eq!(std::fs::read_to_string(&installed).unwrap(), LOCK);
    assert_eq!(
        std::fs::metadata(&installed).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
fn writes_do_not_follow_a_swapped_directory() {
    let dir = tempfile::tempdir().unwrap();

    // The temporary folder is behind the `/var` symlink on macOS, which the walk alone would refuse.
    let root = secure::canonical(dir.path()).unwrap();
    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let flake = root.join("flake");
    std::fs::create_dir_all(&flake).unwrap();
    symlink(&outside, flake.join("sub")).unwrap();

    assert!(secure::write_file(&flake.join("sub/x.nix"), b"x").is_err());
    assert!(!outside.join("x.nix").exists());

    secure::write_file(&flake.join("new/x.nix"), b"x").unwrap();
    assert_eq!(std::fs::read(flake.join("new/x.nix")).unwrap(), b"x");
    secure::remove_file(&flake.join("new/x.nix")).unwrap();
    assert!(!flake.join("new").exists(), "the emptied folder is removed");
}

#[test]
fn a_flake_reached_through_a_symlink_still_works() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir_all(&real).unwrap();
    let alias = dir.path().join("alias");
    symlink(&real, &alias).unwrap();

    let target = secure::canonical(&alias.join("x.nix")).unwrap();
    secure::write_file(&target, b"x").unwrap();
    assert_eq!(std::fs::read(real.join("x.nix")).unwrap(), b"x");
}

#[test]
fn spawn_failures_are_distinguished_from_failed_runs() {
    assert!(matches!(
        run("/nonexistent/drakeflake-helper-test", Vec::<&str>::new()),
        Err(HelperError::Spawn {
            program: _,
            source: _
        })
    ));
    assert!(matches!(
        run("false", Vec::<&str>::new()),
        Err(HelperError::Failed {
            program: _,
            status: _
        })
    ));
}

#[test]
fn messages_keep_their_wording() {
    assert_eq!(
        HelperError::UnknownMode("dance".to_owned()).to_string(),
        "unknown activation mode \"dance\""
    );
}

#[test]
fn every_class_of_failure_has_its_own_exit_code_outside_pkexecs() {
    let io = || std::io::Error::from(std::io::ErrorKind::NotFound);
    let codes = [
        HelperError::Usage.exit_code(),
        HelperError::Refused(drakeflake_plan::Refusal::NotModule).exit_code(),
        HelperError::ApplyPlan(io()).exit_code(),
        HelperError::Spawn {
            program: "x".to_owned(),
            source: io(),
        }
        .exit_code(),
    ];

    assert_eq!(codes, [exit::USAGE, exit::REFUSED, exit::IO, exit::TOOL]);

    let distinct: std::collections::BTreeSet<u8> = codes.into_iter().collect();
    assert_eq!(distinct.len(), codes.len());
    assert!(codes.iter().all(|code| ![0, 1, 126, 127].contains(code)));
}

#[test]
fn temp_names_differ_within_the_same_instant() {
    let name = std::ffi::OsStr::new("drakeflake.nix");
    let names: std::collections::BTreeSet<String> =
        std::iter::repeat_with(|| secure::temp_name(name))
            .take(1000)
            .collect();

    assert_eq!(names.len(), 1000);
    assert!(
        names
            .iter()
            .all(|temp| temp.starts_with(".drakeflake.nix.tmp-"))
    );

    // The clock alone is not enough: two writes may read the same nanosecond.
    assert_ne!(secure::temp_name_at(name, 7), secure::temp_name_at(name, 7));
}

#[cfg(target_os = "linux")]
#[test]
fn subvolume_names_stay_inside_the_top_level() {
    use crate::scan::checked_subvolume;

    assert_eq!(
        checked_subvolume("/root-blank").unwrap(),
        Path::new("root-blank")
    );
    assert_eq!(
        checked_subvolume("snapshots/blank").unwrap(),
        Path::new("snapshots/blank")
    );
    assert_eq!(checked_subvolume("//x").unwrap(), Path::new("x"));
    assert_eq!(checked_subvolume("a/./b").unwrap(), Path::new("a/b"));

    for refused in ["", "/", "../etc", "/../etc", "./x"] {
        assert!(
            matches!(checked_subvolume(refused), Err(HelperError::Denied(_))),
            "{refused} was accepted"
        );
    }
}

#[test]
fn scan_failures_are_not_worded_as_refusals() {
    assert_eq!(
        HelperError::ScanFailed("cannot mount /dev/sda2: EPERM".to_owned()).to_string(),
        "cannot list the root file system: cannot mount /dev/sda2: EPERM"
    );
}
