use drakeflake_render::HostName;

use super::*;
use crate::hub::argv::arg;

#[test]
fn desktop_entries() {
    let package = Path::new("/nix/store/x-demo");
    let entry = "[Desktop Entry]\nName=Demo\nExec=demo --open %U\nTerminal=false\n[Desktop Action new]\nExec=other\n";

    assert_eq!(
        parse_desktop_entry(entry, package),
        Some(Launch::App {
            name: "Demo".to_owned(),
            command: vec![arg("demo"), arg("--open")],
            terminal: false,
        })
    );
    assert_eq!(
        parse_desktop_entry("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n", package),
        None
    );
    assert_eq!(
        parse_desktop_entry("[Desktop Entry]\nName=X\n", package),
        None
    );
}

#[test]
fn shell_scripts_quote_their_words() {
    let script = shell_script("hello", &[PathBuf::from("/nix/store/a-hello/bin")]);
    assert!(script.starts_with("export PATH='/nix/store/a-hello/bin':\"$PATH\""));
    assert!(script.ends_with(EXEC_SHELL));

    let run = terminal::command_line(&[arg("/nix/store/b/bin/htop"), arg("it's")]);
    assert_eq!(run, "'/nix/store/b/bin/htop' 'it'\\''s'");
}

#[test]
fn attribute_paths() {
    assert!(is_attr_path("hello"));
    assert!(is_attr_path("python313Packages.requests"));
    assert!(is_attr_path("linuxKernel.packages.linux_6_6.zfs"));
    assert!(!is_attr_path("--impure"));
    assert!(!is_attr_path("a..b"));
    assert!(!is_attr_path("a;rm"));
    assert!(!is_attr_path(""));
    assert!(!is_attr_path("pkgs.-flag"));
    assert!(!is_attr_path("a.b."));
    assert!(!is_attr_path("a b"));
    assert!(is_attr_path("haskellPackages.lens_'"));
}

#[test]
fn desktop_entry_details() {
    let package = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(package.path().join("bin")).unwrap();
    std::fs::write(package.path().join("bin/demo"), "").unwrap();
    let local = package
        .path()
        .join("bin/demo")
        .to_string_lossy()
        .into_owned();

    for (entry, expected) in [
        (
            "[Desktop Entry]\nExec=\"demo\" %f\nTerminal = true\n",
            Some(Launch::App {
                name: local.clone(),
                command: vec![arg(&local)],
                terminal: true,
            }),
        ),
        (
            "[Desktop Entry]\nName=Tool\nExec=/usr/bin/demo --x\n",
            Some(Launch::App {
                name: "Tool".to_owned(),
                command: vec![arg("/usr/bin/demo"), arg("--x")],
                terminal: false,
            }),
        ),
        (
            "[Desktop Entry]\nName=Other\nExec=other\n",
            Some(Launch::App {
                name: "Other".to_owned(),
                command: vec![arg("other")],
                terminal: false,
            }),
        ),
        ("Name=X\nExec=x\n[Desktop Action new]\nExec=y\n", None),
        ("[Desktop Entry]\nName=X\nExec=x\nHidden=true\n", None),
        ("[Desktop Entry]\nName=X\nExec=%U\n", None),
        ("", None),
    ] {
        assert_eq!(
            parse_desktop_entry(entry, package.path()),
            expected,
            "{entry:?}"
        );
    }
}

#[test]
fn launches_prefer_a_visible_app_then_a_shell() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app");
    let applications = app.join("share/applications");
    std::fs::create_dir_all(&applications).unwrap();
    std::fs::write(
        applications.join("a-hidden.desktop"),
        "[Desktop Entry]\nName=Hidden\nExec=hidden\nNoDisplay=true\n",
    )
    .unwrap();
    std::fs::write(applications.join("notes.txt"), "[Desktop Entry]\nExec=no\n").unwrap();
    std::fs::write(
        applications.join("b-shown.desktop"),
        "[Desktop Entry]\nName=Shown\nExec=shown\n",
    )
    .unwrap();
    let tool = dir.path().join("tool");
    std::fs::create_dir_all(tool.join("bin")).unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir_all(&data).unwrap();

    assert!(matches!(
        launch_for(&[data.clone(), app]),
        Launch::App { name, command: _, terminal: _ } if name == "Shown"
    ));
    assert_eq!(
        launch_for(&[data.clone(), tool.clone()]),
        Launch::Shell {
            bins: vec![tool.join("bin")]
        }
    );
    assert_eq!(launch_for(&[data]), Launch::Nothing);
    assert_eq!(launch_for(&[]), Launch::Nothing);
}

#[test]
fn packages_without_programs_are_reported_not_started() {
    assert_eq!(
        start("zlib", &Launch::Nothing),
        Ok("zlib has no programs to run (it is a library or data package).".to_owned())
    );
}

#[test]
fn bad_attributes_and_missing_systems_are_refused_before_building() {
    let hub = Arc::new(Hub::default());
    let target = drakeflake_flake::Target {
        flake: "/etc/nixos".to_owned(),
        host: "laptop".into(),
        ..Default::default()
    };

    assert_eq!(
        hub.try_package("--impure".to_owned())
            .map_err(|error| error.to_string()),
        Err("\"--impure\" is not a package attribute".to_owned())
    );
    assert_eq!(
        hub.try_package("hello".to_owned())
            .map_err(|error| error.to_string()),
        Err("No system is selected yet".to_owned())
    );
    assert_eq!(
        build_tool(&Nix::system(), &target, "nix-index; rm -rf /"),
        Err("\"nix-index; rm -rf /\" is not a package attribute".to_owned())
    );
    assert!(
        build_tool(
            &Nix::system(),
            &drakeflake_flake::Target {
                host: HostName::default(),
                ..target
            },
            "nix-index"
        )
        .is_err(),
        "an unusable target is not built from"
    );
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST (builds hello and xterm)"]
fn live_launch_kinds() {
    let (Ok(flake), Ok(host)) = (
        std::env::var("DRAKEFLAKE_TEST_FLAKE"),
        std::env::var("DRAKEFLAKE_TEST_HOST"),
    ) else {
        return;
    };

    let target = drakeflake_flake::Target {
        flake,
        host: host.into(),
        ..Default::default()
    };

    let build = |attr: &str| {
        let installable = format!("{}.pkgs.{attr}", target.config_ref().unwrap());
        build_for_trying(
            &Nix::system(),
            &installable,
            target.local_dir().as_deref(),
            attr,
        )
        .unwrap()
    };

    let hello = launch_for(&build("hello"));
    assert!(matches!(hello, Launch::Shell { bins: _ }));

    let xterm = launch_for(&build("xterm"));
    assert!(matches!(
        xterm,
        Launch::App {
            terminal: false,
            name: _,
            command: _
        }
    ));
}
