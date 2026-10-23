use drakeflake_render::OptionName;

use super::*;

const CONFIG: &str = "{ lib, ... }:\n{\n  networking.hostName = \"x\";\n  services.openssh = {\n    enable = true;\n    ports = [ 22 ];\n  };\n  services.printing.enable = true;\n}\n";

#[test]
fn finds_definition_lines() {
    assert_eq!(find_line(CONFIG, "networking.hostName"), 3);
    assert_eq!(find_line(CONFIG, "services.openssh.enable"), 5);
    assert_eq!(find_line(CONFIG, "services.openssh.ports"), 6);
    assert_eq!(find_line(CONFIG, "services.printing.enable"), 8);
    assert_eq!(find_line(CONFIG, "does.not.exist"), 1);
}

#[test]
fn ignores_comments_and_finds_home_manager_paths() {
    let text = "{ ... }:\n{\n  # programs.git.enable = false;\n  programs.git = {\n    enable = true;\n  };\n}\n";

    assert_eq!(find_line(text, "programs.git.enable"), 5);
    assert_eq!(
        find_line(text, "home-manager.users.alice.programs.git.enable"),
        5,
        "Home Manager modules define options relative to the user"
    );
}

#[test]
fn maps_store_copies_to_the_checkout() {
    let dir = std::env::temp_dir().join(format!("drakeflake-editor-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("hosts")).unwrap();
    std::fs::write(dir.join("hosts/laptop.nix"), CONFIG).unwrap();

    let found = locate(
        &OptionName::from("services.openssh.enable"),
        &[
            "/nix/store/abc-source/hosts/laptop.nix".to_owned(),
            "/nix/store/def-source/elsewhere.nix".to_owned(),
        ],
        Some(&dir),
    );

    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(found.len(), 2);
    assert!(found[0].local);
    assert_eq!(found[0].display, "hosts/laptop.nix");
    assert_eq!(found[0].line, 5);
    assert!(!found[1].local);
}

#[test]
fn imported_folders_map_to_their_default_nix() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("modules/desktop")).unwrap();
    std::fs::write(dir.path().join("modules/desktop/default.nix"), CONFIG).unwrap();
    std::fs::create_dir_all(dir.path().join("empty")).unwrap();

    let found = locate(
        &OptionName::from("services.printing.enable"),
        &[
            "/nix/store/abc-source/modules/desktop".to_owned(),
            "/nix/store/abc-source/empty".to_owned(),
            "/nix/store/abc-source".to_owned(),
            "/nonexistent/drakeflake-test/laptop.nix".to_owned(),
        ],
        Some(dir.path()),
    );

    assert_eq!(found[0].display, "modules/desktop/default.nix");
    assert_eq!(found[0].line, 8);
    assert!(found[0].local);
    for other in &found[1..] {
        assert!(!other.local, "{other:?}");
        assert_eq!(other.line, 1, "unreadable files open at the top");
        assert_eq!(other.display, other.file, "shown as found");
    }
}

#[test]
fn desktop_editors_get_their_own_line_arguments() {
    let file = "/etc/nixos/a b.nix";
    for (desktop, program, args) in [
        ("org.kde.kate.desktop", "kate", vec!["--line", "7", file]),
        (
            "org.kde.kwrite.desktop",
            "kwrite",
            vec!["--line", "7", file],
        ),
        (
            "codium.desktop",
            "codium",
            vec!["-g", "/etc/nixos/a b.nix:7"],
        ),
        ("code.desktop", "code", vec!["-g", "/etc/nixos/a b.nix:7"]),
        (
            "dev.zed.zed.desktop",
            "zeditor",
            vec!["/etc/nixos/a b.nix:7"],
        ),
        (
            "org.gnome.texteditor.desktop",
            "gnome-text-editor",
            vec!["+7", file],
        ),
        ("emacs.desktop", "emacs", vec!["+7", file]),
    ] {
        let (found, found_args) = editor_arguments(desktop, Path::new(file), 7).unwrap();
        assert_eq!(found, program, "{desktop}");
        assert_eq!(found_args, args, "{desktop}");
    }

    assert_eq!(
        editor_arguments("org.gnome.gedit.desktop", Path::new(file), 7),
        None
    );
    assert_eq!(editor_arguments("", Path::new(file), 7), None);
}

#[test]
fn only_absolute_nix_files_are_opened() {
    for file in [
        "/nix/store/abc-x/share/x.desktop",
        "/tmp/run.sh",
        "/tmp/nix",
        "relative.nix",
        "-g.nix",
    ] {
        assert!(
            open_in_editor(&Context::system(), file, 1).is_err(),
            "{file}"
        );
    }
}

#[test]
fn lines_from_qml_are_at_least_one() {
    assert_eq!(clamp_line(-4), 1);
    assert_eq!(clamp_line(0), 1);
    assert_eq!(clamp_line(12), 12);
}

#[test]
fn reads_default_nix_of_imported_folders() {
    let dir = std::env::temp_dir().join(format!("drakeflake-editor-dir-{}", std::process::id()));
    let module = dir.join("modules/services/nix-optimise");
    std::fs::create_dir_all(&module).unwrap();
    std::fs::write(
        module.join("default.nix"),
        "{ ... }:\n{\n  nix.optimise.automatic = true;\n}\n",
    )
    .unwrap();

    let found = locate(
        &OptionName::from("nix.optimise.automatic"),
        &[module.to_string_lossy().into_owned()],
        None,
    );

    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .file
            .ends_with("modules/services/nix-optimise/default.nix")
    );
    assert_eq!(found[0].line, 3);
    assert!(!found[0].local);
}

#[test]
fn mac_editors_with_a_tool_get_the_line() {
    let app = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(app.path().join("Contents/MacOS")).unwrap();
    std::fs::write(app.path().join("Contents/MacOS/cli"), "").unwrap();
    let cli = app.path().join("Contents/MacOS/cli");

    assert_eq!(
        mac_editor_arguments(app.path(), "dev.zed.Zed", Path::new("/f/a.nix"), 7),
        vec![arg(&cli), arg("/f/a.nix:7")]
    );

    // LaunchServices stores the identifier in lower case.
    assert_eq!(
        mac_editor_arguments(app.path(), "dev.zed.zed-preview", Path::new("/f/a.nix"), 7)[1],
        "/f/a.nix:7"
    );
}

#[test]
fn other_mac_apps_open_the_file_without_a_line() {
    let app = tempfile::tempdir().unwrap();
    let open = |bundle: &str| {
        vec![arg("open"), arg("-a"), arg(app.path()), arg("/f/a.nix")]
            == mac_editor_arguments(app.path(), bundle, Path::new("/f/a.nix"), 7)
    };

    assert!(open("com.apple.TextEdit"));

    assert!(open("com.microsoft.VSCode"));
}
