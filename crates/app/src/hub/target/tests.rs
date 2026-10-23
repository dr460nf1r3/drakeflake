use std::fs;
use std::path::PathBuf;

use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_flake::{FlakeError, Host, SystemKind};

use super::super::test_support::{console_lines, hub_with, local_target};
use super::*;
use crate::hub::Generation;

const FILE: &str = drakeflake_plan::FILE_NAME;

fn layout(mode: LayoutMode) -> LayoutConfig {
    LayoutConfig {
        mode,
        ..LayoutConfig::default()
    }
}

#[test]
fn a_superseded_loader_cannot_overwrite_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub_with(None);
    hub.session()
        .update(|state| state.generation = Generation(2));

    assert!(
        !hub.session()
            .store_target_if_current(Generation(1), &local_target(dir.path()))
    );
    assert!(hub.target().is_none());
    assert!(
        hub.session()
            .store_target_if_current(Generation(2), &local_target(dir.path()))
    );
    assert!(hub.target().is_some());
}

#[test]
fn the_managed_file_follows_target_setting_and_layout() {
    let flake = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let out_dir = format!("{}/", out.path().display());

    assert_eq!(
        hub_with(None)
            .managed_file()
            .map_err(|error| error.to_string()),
        Err("No system is selected yet".to_owned())
    );

    let cases = [
        (None, LayoutMode::AreaModule, flake.path().join(FILE)),
        (
            Some(out_dir.clone()),
            LayoutMode::AreaModule,
            out.path().join("default.nix"),
        ),
        (Some(out_dir), LayoutMode::Single, out.path().join(FILE)),
    ];

    for (setting, mode, expected) in cases {
        let hub = hub_with(Some(local_target(flake.path())));
        hub.session()
            .update(|state| state.managed_setting.clone_from(&setting));
        hub.set_layout(layout(mode));
        assert_eq!(
            hub.managed_file().ok(),
            Some(expected),
            "{setting:?} {mode:?}"
        );
    }

    let remote = hub_with(Some(Target {
        flake: "github:owner/repo".to_owned(),
        host: "box".into(),
        kind: SystemKind::NixOs,
    }));

    assert!(remote.managed_file().is_err(), "a remote flake has no file");
}

#[test]
fn generated_paths_are_the_entry_and_its_parts_folder() {
    let flake = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let hub = hub_with(None);
    let flake_ref = flake.path().to_string_lossy().into_owned();

    assert_eq!(
        hub.session().generated_paths_of(&flake_ref),
        vec![flake.path().join(FILE), flake.path().join("drakeflake")]
    );

    hub.session().update(|state| {
        state.managed_setting = Some(format!("{}/", out.path().display()));
    });
    let paths = hub.session().generated_paths_of(&flake_ref);
    assert_eq!(paths.first(), Some(&out.path().join("default.nix")));
    assert_eq!(
        paths
            .get(1)
            .map(|folder| folder.components().collect::<PathBuf>()),
        Some(out.path().to_path_buf())
    );

    assert_eq!(
        hub_with(None)
            .session()
            .generated_paths_of("github:owner/repo"),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn an_unlisted_flake_falls_back_to_a_valid_preferred_host() {
    let hub = hub_with(None);
    let failed: Result<Vec<Host>, FlakeError> = Err(FlakeError::Nix("eval failed".to_owned()));

    let chosen = choose_host(hub.console(), &failed, Some("box")).unwrap();
    assert_eq!(chosen.name, "box");
    assert_eq!(chosen.kind, SystemKind::native());
    assert_eq!(
        console_lines(&hub),
        vec!["Could not list hosts (eval failed); trying \"box\"".to_owned()]
    );

    for preferred in [None, Some(""), Some("box\"; rm")] {
        assert_eq!(
            choose_host(hub.console(), &failed, preferred).map(|host| host.name),
            Err("eval failed".to_owned()),
            "{preferred:?}"
        );
    }
}

fn warning_ids(hub: &Hub, target: &Target) -> Vec<WarningId> {
    check(hub.services(), &hub.nix(), target);
    hub.mail
        .system
        .take()
        .into_iter()
        .flat_map(|message| match message {
            SystemMsg::Warnings(warnings) => warnings,
            SystemMsg::Target(_)
            | SystemMsg::TargetError {
                flake: _,
                hosts: _,
                error: _,
            }
            | SystemMsg::Running(_)
            | SystemMsg::Finished { ok: _, backup: _ }
            | SystemMsg::Activities(_)
            | SystemMsg::HomeUsers {
                users: _,
                selected: _,
                error: _,
            }
            | SystemMsg::CheckUi => Vec::new(),
        })
        .map(|warning| warning.id)
        .collect()
}

#[test]
fn layout_problems_become_warnings() {
    let flake = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = local_target(flake.path());
    let missing = local_target(&flake.path().join("gone"));
    let remote = Target {
        flake: "github:owner/repo".to_owned(),
        ..target.clone()
    };

    let outside_file = Some(
        outside
            .path()
            .join("out.nix")
            .to_string_lossy()
            .into_owned(),
    );

    let cases = [
        (&remote, None, vec![WarningId::Output]),
        (&remote, outside_file.clone(), vec![WarningId::Remote]),
        (&missing, None, vec![WarningId::Missing]),
        (&target, outside_file, vec![WarningId::Outside]),
        (&target, None, vec![WarningId::NotImported]),
    ];

    for (checked, setting, expected) in cases {
        let hub = hub_with(Some(checked.clone()));
        hub.session()
            .update(|state| state.managed_setting.clone_from(&setting));
        assert_eq!(
            warning_ids(&hub, checked),
            expected,
            "{} {setting:?}",
            checked.flake
        );
    }

    fs::write(
        flake.path().join("configuration.nix"),
        "{ imports = [ ./drakeflake.nix ]; }",
    )
    .unwrap();
    let hub = hub_with(Some(target.clone()));
    assert_eq!(warning_ids(&hub, &target), Vec::<WarningId>::new());
}
