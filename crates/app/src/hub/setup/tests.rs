use super::*;

fn hub_for(flake: &str, output: Option<String>) -> Arc<Hub> {
    let hub = Arc::new(Hub::default());
    hub.session().update(|state| {
        state.target = Some(Target {
            flake: flake.to_owned(),
            host: "laptop".into(),
            ..Target::default()
        });
        state.managed_setting = output;
    });
    hub
}

fn stops_with(hub: &Arc<Hub>) -> ImportCheck {
    Exclusive::detached(hub)
        .write_for_import_check()
        .err()
        .expect("the check went on")
}

#[test]
fn import_checks_stop_before_writing_when_the_file_cannot_be_imported() {
    let flake = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let outside = elsewhere.path().join("drakeflake.nix");
    let flake_path = flake.path().to_string_lossy().into_owned();

    assert_eq!(
        stops_with(&Arc::new(Hub::default())),
        ImportCheck::Failed("No system is selected yet.".to_owned())
    );
    for remote in ["github:me/config", "/nonexistent/drakeflake-flake"] {
        assert_eq!(
            stops_with(&hub_for(remote, None)),
            ImportCheck::Failed("The flake is not a local folder.".to_owned()),
            "{remote}"
        );
    }

    assert_eq!(
        stops_with(&hub_for(&flake_path, Some("relative.nix".to_owned()))),
        ImportCheck::Failed("The output path \"relative.nix\" must be absolute".to_owned())
    );

    let ImportCheck::Failed(message) = stops_with(&hub_for(
        &flake_path,
        Some(outside.to_string_lossy().into_owned()),
    )) else {
        panic!("a file outside the flake cannot be imported");
    };

    assert!(
        message.contains("is not inside the flake folder"),
        "{message}"
    );
    assert!(!outside.exists(), "nothing was written");
}

#[test]
fn import_line_is_relative_to_the_flake() {
    let flake = tempfile::tempdir().unwrap();
    let modules = flake.path().join("modules/system");
    std::fs::create_dir_all(&modules).unwrap();

    assert_eq!(
        import_line(&modules.join("drakeflake.nix"), flake.path()).as_deref(),
        Some("imports = [ ./modules/system/drakeflake.nix ];")
    );
    assert_eq!(
        import_line(&modules.join("default.nix"), flake.path()).as_deref(),
        Some("imports = [ ./modules/system ];")
    );
    assert_eq!(
        import_line(&flake.path().join("new/default.nix"), flake.path()).as_deref(),
        Some("imports = [ ./new ];"),
        "a folder written on the first save"
    );
    assert_eq!(
        import_line(Path::new("/etc/drakeflake.nix"), flake.path()),
        None
    );
}
