use std::io::Write as _;
use std::path::Path;

use super::*;

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[test]
fn full_config() {
    let file = write_temp(
        "[general]\nhighlight_theme = \"Nord\"\n[nix]\nflake_path = \"/home/user/nixos\"\nhost = \"laptop\"\n",
    );

    let config = load_from(file.path()).unwrap();

    assert_eq!(config.nix.flake_path, "/home/user/nixos");
    assert_eq!(config.nix.host.as_deref(), Some("laptop"));
    assert_eq!(config.general.highlight_theme, "Nord");
}

#[test]
fn empty_file_uses_defaults() {
    let file = write_temp("");

    let config = load_from(file.path()).unwrap();

    assert_eq!(config.nix.flake_path, default_flake_path());
    assert_eq!(config.nix.host, None);
    assert_eq!(config.nix.managed_path, None);
}

#[test]
fn partial_config_fills_defaults() {
    let file = write_temp("[nix]\nhost = \"server\"\n");

    let config = load_from(file.path()).unwrap();

    assert_eq!(config.nix.flake_path, default_flake_path());
    assert_eq!(config.nix.host.as_deref(), Some("server"));
}

#[test]
fn auto_commit_defaults_to_yes() {
    assert!(Config::default().general.auto_commit);

    let file = write_temp("[general]\nhighlight_theme = \"Nord\"\n");

    let config = load_from(file.path()).unwrap();

    assert!(config.general.auto_commit);
}

#[test]
fn auto_commit_can_be_switched_off() {
    let file = write_temp("[general]\nauto_commit = false\n");

    let config = load_from(file.path()).unwrap();

    assert!(!config.general.auto_commit);
}

#[test]
fn missing_file_errors() {
    let result = load_from(Path::new("/nonexistent-dir-xyz/config.toml"));

    result.unwrap_err();
}

#[test]
fn desktop_theme_prefers_the_colour_scheme() {
    let look_only =
        "[General]\nColorSchemeHash=1e79\n[KDE]\nLookAndFeelPackage=Catppuccin-Mocha-Mauve\n";

    assert_eq!(
        theme_name_from_kdeglobals(look_only).as_deref(),
        Some("Catppuccin-Mocha-Mauve")
    );

    let both =
        "[KDE]\nLookAndFeelPackage=org.kde.breeze.desktop\n[General]\nColorScheme=BreezeDark\n";

    assert_eq!(
        theme_name_from_kdeglobals(both).as_deref(),
        Some("BreezeDark")
    );
    assert_eq!(theme_name_from_kdeglobals("[Icons]\nTheme=x\n"), None);
}

#[test]
fn roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    let config = Config {
        general: General {
            highlight_theme: "Monokai".to_owned(),
            log_level: "debug".to_owned(),
            blur: BlurMode::Off,
            auto_commit: false,
        },
        nix: Nix {
            flake_path: "/home/user/nixos".to_owned(),
            host: Some("laptop".into()),
            managed_path: Some("/home/user/nixos/modules/settings.nix".to_owned()),
        },
        layout: LayoutConfig {
            mode: LayoutMode::Area,
            rules: LayoutConfig::parse_rules("services.openssh = ssh.nix").unwrap(),
        },
        tray: Tray {
            check_hours: 12,
            staged: false,
            ..Tray::default()
        },
    };

    save_to(&config, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("[general]") && text.contains("[nix]"),
        "{text}"
    );
    let back = load_from(&path).unwrap();

    assert_eq!(back.nix.flake_path, "/home/user/nixos");
    assert_eq!(back.general.highlight_theme, "Monokai");
    assert_eq!(back.general.blur, BlurMode::Off);
    assert_eq!(back.nix.host.as_deref(), Some("laptop"));
    assert_eq!(
        back.nix.managed_path.as_deref(),
        Some("/home/user/nixos/modules/settings.nix")
    );
    assert_eq!(back.layout, config.layout);
    assert_eq!(back.tray, config.tray);
}

#[test]
fn blur_defaults_to_auto() {
    let file = write_temp("");

    let config = load_from(file.path()).unwrap();

    assert_eq!(config.general.blur, BlurMode::Auto);
}
#[test]
fn broken_toml_errors() {
    let file = write_temp("flake_path = [unclosed\n");

    let result = load_from(file.path());

    result.unwrap_err();
}

#[test]
fn tray_reports_default_on_and_can_be_switched_off() {
    let file = write_temp("[tray]\nlow_space = false\n");
    let mut config = load_from(file.path()).unwrap();

    assert!(config.tray.restart && config.tray.failed_services && config.tray.staged);
    assert!(!config.tray.low_space);

    assert!(config.tray.set("staged", false));
    assert!(!config.tray.set("unknown", false));
    assert!(!config.tray.staged);
    assert_eq!(Report::ALL.len(), 6);
    assert_eq!(
        config.tray.check_every(),
        std::time::Duration::from_hours(6)
    );
    assert!(config.tray.set_interval("check_hours", 12));
    assert!(!config.tray.set_interval("check_hours", 0));
    assert!(!config.tray.set_interval("health_minutes", 61));
    assert_eq!(
        config.tray.check_every(),
        std::time::Duration::from_hours(12)
    );
    config.tray.health_minutes = 0;
    assert_eq!(
        config.tray.health_every(),
        std::time::Duration::from_mins(1)
    );

    let text = toml::to_string(&config).unwrap();
    assert!(
        text.contains("[tray]") && text.contains("staged = false"),
        "{text}"
    );
}

#[test]
fn each_report_switch_changes_only_its_own_field() {
    type Switch = (&'static str, bool, fn(&mut Tray));

    let cases: [Switch; 6] = [
        ("updates", false, |tray| tray.updates = false),
        ("restart", false, |tray| tray.restart = false),
        ("failed_services", false, |tray| {
            tray.failed_services = false;
        }),
        ("low_space", false, |tray| tray.low_space = false),
        ("staged", false, |tray| tray.staged = false),
        ("watch_flake", true, |tray| tray.watch_flake = true),
    ];

    for (name, value, change) in cases {
        let mut switched = Tray::default();
        assert!(switched.set(name, value), "{name} is not a known switch");
        let mut expected = Tray::default();
        change(&mut expected);
        assert_eq!(switched, expected, "{name}");
    }

    assert_eq!(cases.map(|(name, _, _)| name), Report::ALL.map(Report::key));

    for report in Report::ALL {
        assert_eq!(Report::from_key(report.key()), Some(report));
        assert_eq!(
            Tray::default().enabled(report),
            report != Report::WatchFlake
        );
    }
}

#[test]
fn hand_edited_intervals_are_clamped_to_their_range() {
    let file = write_temp("[tray]\ncheck_hours = 1000\nhealth_minutes = 0\n");
    let tray = load_from(file.path()).unwrap().tray;

    assert_eq!(tray.check_every(), std::time::Duration::from_hours(168));
    assert_eq!(tray.health_every(), std::time::Duration::from_mins(1));

    let mut edited = Tray::default();
    for (name, value, accepted) in [
        ("check_hours", 1, true),
        ("check_hours", 168, true),
        ("check_hours", 169, false),
        ("health_minutes", 60, true),
        ("health_minutes", 0, false),
        ("updates", 1, false),
    ] {
        assert_eq!(
            edited.set_interval(name, value),
            accepted,
            "{name} = {value}"
        );
    }

    assert_eq!((edited.check_hours, edited.health_minutes), (168, 60));
}

#[test]
fn wrongly_typed_values_are_refused_not_defaulted() {
    for text in [
        "[tray]\ncheck_hours = \"six\"\n",
        "[tray]\nupdates = 1\n",
        "[general]\nblur = \"sometimes\"\n",
        "[nix]\nhost = 3\n",
    ] {
        let file = write_temp(text);
        let error = load_from(file.path()).unwrap_err();
        assert!(format!("{error:#}").contains("cannot parse"), "{text}");
    }
}

#[test]
fn saving_creates_missing_folders_and_reports_unwritable_ones() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/config.toml");
    save_to(&Config::default(), &nested).unwrap();
    assert_eq!(
        load_from(&nested).unwrap().general.log_level,
        default_log_level()
    );

    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "").unwrap();
    let error = save_to(&Config::default(), &blocker.join("config.toml")).unwrap_err();
    assert!(format!("{error:#}").contains("cannot create"), "{error:#}");
}

#[test]
fn a_broken_file_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let broken = "[general\nflake_path = \"/mine\"\n";
    std::fs::write(&path, broken).unwrap();

    let error = save_to(&Config::default(), &path).unwrap_err();
    let message = format!("{error:#}");

    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("Fix or remove"), "{message}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn saving_creates_a_missing_file_and_replaces_a_valid_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut config = Config::default();

    save_to(&config, &path).unwrap();
    assert!(path.exists());

    config.general.log_level = "debug".to_owned();
    save_to(&config, &path).unwrap();

    assert_eq!(load_from(&path).unwrap().general.log_level, "debug");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn blur_modes_are_saved_by_their_names() {
    for mode in [BlurMode::Auto, BlurMode::On, BlurMode::Off] {
        let config = Config {
            general: General {
                blur: mode,
                ..General::default()
            },
            ..Config::default()
        };

        let text = toml::to_string(&config).unwrap();

        assert!(
            text.contains(&format!("blur = \"{}\"", mode.as_str())),
            "{text}"
        );
        assert_eq!(mode.as_str().parse(), Ok(mode));
    }
}

#[test]
fn kdeglobals_ignores_empty_values_and_other_sections() {
    let text = "[General]\nColorScheme=\n[Colors:View]\nColorScheme=Wrong\n[KDE]\n  LookAndFeelPackage = org.kde.breezedark.desktop  \n";
    assert_eq!(
        theme_name_from_kdeglobals(text).as_deref(),
        Some("org.kde.breezedark.desktop")
    );
}

#[test]
fn a_bad_layout_rule_is_dropped_and_the_rest_of_the_config_loads() {
    let file = write_temp(
        "[general]\nauto_commit = false\n\n\
         [[layout.rules]]\nprefix = \"services\"\nfile = \"/etc/x.nix\"\n\n\
         [[layout.rules]]\nprefix = \"boot\"\nfile = \"boot.nix\"\n",
    );

    let (config, notes) = load_with_notes(file.path()).unwrap();

    assert!(!config.general.auto_commit);
    assert_eq!(
        config
            .layout
            .rules
            .iter()
            .map(|rule| (rule.prefix(), rule.file()))
            .collect::<Vec<_>>(),
        [("boot", "boot.nix")]
    );
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("(\"services\")"), "{notes:?}");
}

#[test]
fn a_config_with_a_bad_rule_can_still_be_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "[[layout.rules]]\nprefix = \"services\"\nfile = \"../x.nix\"\n",
    )
    .unwrap();

    let (config, notes) = load_with_notes(&path).unwrap();
    assert_eq!(notes.len(), 1);

    save_to(&config, &path).unwrap();

    let (_, after) = load_with_notes(&path).unwrap();
    assert_eq!(after, Vec::<String>::new());
}

#[test]
fn layout_modes_round_trip_through_their_names() {
    let modes = [LayoutMode::Single, LayoutMode::Area, LayoutMode::AreaModule];
    assert_eq!(modes.map(|mode| mode.to_string().parse()), modes.map(Ok));
    assert_eq!(LayoutMode::AreaModule.to_string(), "area-module");
    assert_eq!("nope".parse::<LayoutMode>(), Err(UnknownLayoutMode));
}
