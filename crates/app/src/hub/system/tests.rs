use std::path::PathBuf;
use std::sync::Arc;

use drakeflake_file::Elevation;
use drakeflake_flake::{SystemKind, Target};

use super::darwin::{
    DARWIN_SWITCH_SCRIPT, DARWIN_SYSTEM_PROFILE, activation_command as darwin_activation,
    switch_generation_command,
};
use super::nixos::{activation_command as nixos_activation, rebuild_tool_command, rollback_args};
use super::*;
use crate::hub::Hub;
use crate::hub::argv::arg;
use crate::hub::test_support::{console_lines, texts};

fn words(items: &[&str]) -> Vec<OsString> {
    items.iter().map(arg).collect()
}

#[test]
fn the_kind_picks_the_names() {
    assert_eq!(System::of(SystemKind::NixOs), System::NixOs);
    assert_eq!(System::of(SystemKind::Darwin), System::Darwin);
    assert_eq!(System::native(), System::of(SystemKind::native()));

    assert_eq!(
        (
            System::NixOs.nh_subcommand(),
            System::NixOs.rebuild_program()
        ),
        ("os", "nixos-rebuild")
    );
    assert_eq!(
        (
            System::Darwin.nh_subcommand(),
            System::Darwin.rebuild_program()
        ),
        ("darwin", "darwin-rebuild")
    );
}

#[test]
fn nh_features_come_from_one_help_text() {
    let help = "Options:\n  --diff <WHEN>\n  --elevation-strategy <STRATEGY>\n";

    assert_eq!(
        NhFeatures::from_help(help),
        NhFeatures {
            elevation_strategy: true,
            diff: true
        }
    );
    assert_eq!(NhFeatures::from_help("--ask"), NhFeatures::default());
}

#[test]
fn review_arguments_per_system() {
    assert_eq!(
        texts(&System::NixOs.review_args(
            Path::new("/etc/nixos"),
            "laptop",
            Mode::Switch,
            Some(Path::new("/run/wrappers/bin/pkexec"))
        ))
        .join(" "),
        "os switch /etc/nixos --hostname laptop --ask --elevation-strategy /run/wrappers/bin/pkexec"
    );
    assert!(
        !System::NixOs
            .review_args(Path::new("/etc/nixos"), "laptop", Mode::Test, None)
            .contains(&arg("--elevation-strategy"))
    );
    assert_eq!(
        texts(&System::Darwin.review_args(
            Path::new("/etc/nix-darwin"),
            "macbook",
            Mode::Switch,
            Some(Path::new("/usr/bin/sudo"))
        ))
        .join(" "),
        "darwin switch /etc/nix-darwin --hostname macbook --ask --elevation-strategy /usr/bin/sudo"
    );
}

#[test]
fn nixos_activates_through_the_helper() {
    let elevation = Elevation::Pkexec(PathBuf::from("/run/wrappers/bin/pkexec"));

    assert_eq!(
        nixos_activation(
            &elevation,
            Path::new("/lib/drakeflake-helper"),
            Mode::Boot,
            Path::new("/nix/store/abc-nixos-system")
        ),
        words(&[
            "/run/wrappers/bin/pkexec",
            "/lib/drakeflake-helper",
            "activate",
            "boot",
            "/nix/store/abc-nixos-system"
        ])
    );
    assert_eq!(
        rebuild_tool_command(
            &elevation,
            Path::new("/nix/store/abc/bin/nixos-rebuild"),
            Mode::Test,
            "/etc/nixos#laptop"
        ),
        words(&[
            "/run/wrappers/bin/pkexec",
            "/nix/store/abc/bin/nixos-rebuild",
            "test",
            "--flake",
            "/etc/nixos#laptop"
        ])
    );
    assert_eq!(
        texts(&rollback_args(7, Some(Path::new("/usr/bin/sudo")))).join(" "),
        "os rollback --to 7 --elevation-strategy /usr/bin/sudo"
    );
    assert_eq!(
        texts(&rollback_args(7, None)).join(" "),
        "os rollback --to 7"
    );
}

#[test]
fn darwin_switch_needs_one_elevation() {
    let elevation = Elevation::MacDialog(PathBuf::from("/app/libexec/drakeflake-elevate"));

    assert_eq!(
        darwin_activation(
            &elevation,
            Path::new("/run/current-system/sw/bin/nix"),
            Path::new("/nix/store/abc-darwin-system"),
        ),
        words(&[
            "/app/libexec/drakeflake-elevate",
            "/bin/sh",
            "-c",
            DARWIN_SWITCH_SCRIPT,
            "sh",
            "/run/current-system/sw/bin/nix",
            "/nix/store/abc-darwin-system",
            DARWIN_SYSTEM_PROFILE,
        ]),
    );
    assert_eq!(
        switch_generation_command(
            &elevation,
            Path::new("/nix/store/abc/bin/darwin-rebuild"),
            3
        ),
        words(&[
            "/app/libexec/drakeflake-elevate",
            "/nix/store/abc/bin/darwin-rebuild",
            "--switch-generation",
            "3"
        ])
    );

    for fragment in [
        "\"$1\"",
        "\"$2\"",
        "\"$3\"",
        "--no-link",
        "--profile",
        "darwin-rebuild\" activate",
        "HOME=/var/root",
    ] {
        assert!(
            DARWIN_SWITCH_SCRIPT.contains(fragment),
            "the switch script lost {fragment}"
        );
    }
}

#[test]
fn darwin_rebuild_review_and_generations_only_switch() {
    let hub = Arc::new(Hub::default());
    let run = Exclusive::detached(&hub);
    let target = Target {
        flake: "/etc/nix-darwin".to_owned(),
        host: "macbook".into(),
        kind: SystemKind::Darwin,
    };

    let generation = Generation {
        id: 1,
        path: "/nix/store/abc-system".to_owned(),
        date: None,
        version: String::new(),
        kernel: String::new(),
        specialisations: Vec::new(),
        current: false,
        running: false,
        running_specialisation: None,
        booted: false,
    };

    assert_eq!(
        System::Darwin.rebuild_as_root(
            &run,
            Rebuild {
                installable: "/etc/nix-darwin#macbook",
                system: System::Darwin,
                mode: Mode::Test,
                flake_dir: Path::new("/"),
            }
        ),
        Outcome::Failed
    );
    assert_eq!(
        System::Darwin.review(&run, &target, Mode::Test, Path::new("/")),
        Outcome::Failed
    );
    assert_eq!(
        System::Darwin.activate_generation(&run, &generation, Mode::Boot),
        Outcome::Failed
    );
    assert_eq!(
        console_lines(&hub),
        vec!["nix-darwin hosts can only be switched."; 3],
    );
}

#[test]
#[ignore = "needs nh"]
fn vm_nh_feature_probe() {
    let Some(nh) = crate::hub::find_program("nh") else {
        return;
    };

    let features = System::NixOs.nh_features(&Context::system(), &nh);
    assert!(
        features.elevation_strategy,
        "expected a new nh with --elevation-strategy, update the guest"
    );
    assert!(
        System::Darwin.nh_features(&Context::system(), &nh).diff,
        "expected nh with --diff, update the guest"
    );
}
