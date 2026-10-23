use super::*;

#[test]
fn elevation_prefixes() {
    let run0 = Elevation::Run0(PathBuf::from("/usr/bin/run0"));
    assert_eq!(run0.prefix(), ["/usr/bin/run0", "--background="]);

    let dialog = Elevation::MacDialog(PathBuf::from("/app/libexec/drakeflake-elevate"));
    assert_eq!(dialog.prefix(), ["/app/libexec/drakeflake-elevate"]);
    assert_eq!(
        dialog.program(),
        Path::new("/app/libexec/drakeflake-elevate")
    );
    assert_eq!(dialog.name(), "the macOS administrator dialog");
}

fn bin_with(names: &[&str]) -> (tempfile::TempDir, PathBuf, OsString) {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    for name in names {
        let path = bin.join(name);
        fs::write(&path, "").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let paths = std::env::join_paths([Path::new("/nonexistent-drakeflake"), &bin]).unwrap();
    (dir, bin, paths)
}

#[test]
fn elevation_is_chosen_best_first() {
    let (_dir, bin, paths) = bin_with(&["pkexec", "run0", "sudo"]);
    let wrapper = |name: &str| Path::new("/run/wrappers/bin").join(name);
    let in_bin = |path: &Path| path.starts_with(&bin);

    let dialog = PathBuf::from("/app/libexec/drakeflake-elevate");
    assert_eq!(
        choose_elevation(
            &paths,
            Prompt::TerminalAllowed,
            Some(dialog.clone()),
            |_| true
        ),
        Some(Elevation::MacDialog(dialog))
    );
    assert_eq!(
        choose_elevation(&paths, Prompt::TerminalAllowed, None, |_| true),
        Some(Elevation::Pkexec(wrapper("pkexec"))),
        "the NixOS wrapper before any pkexec on PATH"
    );
    assert_eq!(
        choose_elevation(&paths, Prompt::TerminalAllowed, None, in_bin),
        Some(Elevation::Pkexec(bin.join("pkexec")))
    );

    assert_eq!(
        choose_elevation(&paths, Prompt::TerminalAllowed, None, |_| false),
        Some(Elevation::Run0(bin.join("run0")))
    );
}

#[test]
fn sudo_is_only_used_with_a_terminal_for_the_password() {
    let (_dir, bin, paths) = bin_with(&["sudo"]);
    let sudo_only = |path: &Path| path.ends_with("sudo");

    assert_eq!(
        choose_elevation(&paths, Prompt::DialogOnly, None, sudo_only),
        None
    );
    assert_eq!(
        choose_elevation(&paths, Prompt::TerminalAllowed, None, sudo_only),
        Some(Elevation::Sudo(PathBuf::from(SUDO_WRAPPER)))
    );
    assert_eq!(
        choose_elevation(&paths, Prompt::TerminalAllowed, None, |path| path
            == bin.join("sudo")),
        Some(Elevation::Sudo(bin.join("sudo")))
    );
}

#[test]
fn a_run0_folder_is_not_a_program() {
    let (_dir, bin, paths) = bin_with(&[]);
    fs::create_dir_all(bin.join("run0")).unwrap();

    assert_eq!(
        choose_elevation(&paths, Prompt::DialogOnly, None, |_| false),
        None
    );
}

#[test]
fn only_root_owned_setuid_files_can_elevate() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let (_dir, bin, _) = bin_with(&["pkexec"]);
    let program = bin.join("pkexec");

    // The Nix build sandbox (the flake's `tests` check) refuses the bit itself.
    if fs::set_permissions(&program, fs::Permissions::from_mode(0o4755)).is_ok() {
        let owned_by_root = fs::metadata(&program).unwrap().uid() == 0;
        assert_eq!(is_setuid_root(&program), owned_by_root);
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    }

    assert!(!is_setuid_root(&program));
    assert!(!is_setuid_root(&bin));
    assert!(!is_setuid_root(&bin.join("missing")));
}

#[test]
fn no_mac_dialog_elsewhere() {
    if !cfg!(target_os = "macos") {
        assert_eq!(mac_dialog(), None);
    }
}

#[test]
#[cfg(all(target_os = "macos", debug_assertions))]
fn mac_dialog_in_source_tree() {
    // Cargo builds run from the checkout: the script has no libexec
    // beside them yet, and dev runs must not fall through to sudo.
    let script = mac_dialog().expect("drakeflake-elevate in the source tree");
    assert!(script.ends_with("data/macos/drakeflake-elevate.sh"));
}
