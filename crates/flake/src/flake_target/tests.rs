use super::*;

#[test]
fn host_and_flake_validation() {
    validate_host("dragons-strix").unwrap();
    assert!(validate_host("bad\"host").is_err());
    assert!(validate_host("").is_err());

    let target = Target {
        flake: "/etc/nixos".into(),
        host: "laptop".into(),
        ..Default::default()
    };

    assert_eq!(
        target.config_ref().unwrap(),
        "/etc/nixos#nixosConfigurations.\"laptop\""
    );
    assert_eq!(
        Target {
            flake: "/etc/nix-darwin".into(),
            host: "mac".into(),
            kind: SystemKind::Darwin,
        }
        .config_ref()
        .unwrap(),
        "/etc/nix-darwin#darwinConfigurations.\"mac\""
    );
    Target {
        flake: "/etc/nixos#x".into(),
        host: "a".into(),
        ..Default::default()
    }
    .config_ref()
    .unwrap_err();
    Target::default().config_ref().unwrap_err();
}

#[test]
fn local_dirs() {
    assert_eq!(
        local_flake_dir("/etc/nixos"),
        Some(PathBuf::from("/etc/nixos"))
    );
    assert_eq!(
        local_flake_dir("git+file:///home/me/cfg?ref=main"),
        Some(PathBuf::from("/home/me/cfg"))
    );
    assert_eq!(local_flake_dir("github:owner/repo"), None);
    assert_eq!(
        local_flake_dir(" path:/srv/cfg?dir=hosts "),
        Some(PathBuf::from("/srv/cfg"))
    );
    assert_eq!(
        local_flake_dir("/srv/cfg#box"),
        Some(PathBuf::from("/srv/cfg"))
    );

    assert_eq!(local_flake_dir("./cfg"), None);
    assert_eq!(local_flake_dir("path:cfg"), None);
    if let Some(home) = std::env::var_os("HOME") {
        assert_eq!(
            local_flake_dir("~/nixos"),
            Some(Path::new(&home).join("nixos"))
        );
    }

    assert_eq!(get_flake_ref(" path:/srv/cfg "), "/srv/cfg");
    assert_eq!(get_flake_ref(" github:o/r "), "github:o/r");
}

#[test]
fn odd_flake_references_and_host_names_are_refused() {
    for flake in [
        "",
        "   ",
        "/etc/nixos#box",
        "/etc/nixos\n--impure",
        "/x\u{7}",
        "--impure",
        " -x",
        "./cfg",
        "../cfg",
        "cfg/sub",
        "path:cfg",
    ] {
        assert!(validate_flake(flake).is_err(), "{flake:?}");
    }

    assert!(matches!(
        validate_flake("./cfg").unwrap_err(),
        FlakeError::InvalidFlake(FlakeProblem::RelativePath)
    ));

    validate_flake(" github:o/r?dir=x ").unwrap();

    for host in ["web-1", "box_2", "host.example.org"] {
        validate_host(host).unwrap();
    }

    for host in ["a b", "a\"b", "${x}", "höst", "a\\b"] {
        assert!(validate_host(host).is_err(), "{host:?}");
    }
}

#[test]
fn flake_root_normalization() {
    let root = std::env::temp_dir().join(format!("drakeflake-flake-root-{}", std::process::id()));
    std::fs::create_dir_all(root.join("devshells/inner")).unwrap();
    std::fs::write(root.join("flake.nix"), "{ outputs = _: { }; }").unwrap();

    let root_text = root.to_string_lossy().into_owned();
    assert_eq!(normalize_flake(&root_text), root_text);
    assert_eq!(
        normalize_flake(&root.join("devshells/inner").to_string_lossy()),
        root_text
    );
    assert_eq!(normalize_flake("github:owner/repo"), "github:owner/repo");
    assert_eq!(normalize_flake(" /nonexistent-dir "), "/nonexistent-dir");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn get_flake_ref_resolves_symlinks() {
    let root = std::env::temp_dir().join(format!("drakeflake-flake-link-{}", std::process::id()));
    let target = root.join("real");
    std::fs::create_dir_all(&target).unwrap();
    let canonical = target.canonicalize().unwrap();

    #[cfg(unix)]
    {
        let link = root.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(
            get_flake_ref(&link.to_string_lossy()),
            canonical.to_string_lossy()
        );
    }

    assert_eq!(get_flake_ref(" /nonexistent-dir "), "/nonexistent-dir");
    assert_eq!(get_flake_ref("github:owner/repo"), "github:owner/repo");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn same_flake_resolves_symlinks() {
    let root = std::env::temp_dir().join(format!("drakeflake-flake-same-{}", std::process::id()));
    let target = root.join("real");
    std::fs::create_dir_all(&target).unwrap();
    let link = root.join("link");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(same_flake(
            &link.to_string_lossy(),
            &target.to_string_lossy()
        ));
    }

    assert!(same_flake("/etc/nixos", "/etc/nixos"));
    assert!(same_flake(" /etc/nixos ", "/etc/nixos"));
    assert!(!same_flake("/etc/nixos", "/etc/nix-darwin"));
    assert!(!same_flake("github:owner/repo", "github:owner/other"));
    assert!(!same_flake("/nonexistent-a", "/nonexistent-b"));

    std::fs::remove_dir_all(&root).ok();
}

#[cfg(unix)]
#[test]
fn a_flake_that_resolves_to_a_non_utf8_folder_is_refused() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let root = tempfile::tempdir().unwrap();
    let odd = root.path().join(OsString::from_vec(vec![b'f', 0xff, b'x']));
    let link = root.path().join("link");

    if std::fs::create_dir_all(&odd).is_err() {
        return;
    }

    std::os::unix::fs::symlink(&odd, &link).unwrap();

    let err = validate_flake(link.to_str().unwrap()).unwrap_err();

    assert!(
        matches!(err, FlakeError::InvalidFlake(FlakeProblem::NotUtf8)),
        "{err}"
    );
    validate_flake(root.path().to_str().unwrap()).unwrap();
}

#[cfg(unix)]
#[test]
fn a_path_without_a_utf8_form_is_told_apart() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let odd = PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));

    assert!(lacks_text_form(&odd, None));
    assert!(lacks_text_form(Path::new("/etc/nixos"), Some(&odd)));
    assert!(!lacks_text_form(Path::new("/etc/nixos"), None));
    assert!(!lacks_text_form(
        Path::new("/etc/nixos"),
        Some(Path::new("/etc/nixos"))
    ));
}
