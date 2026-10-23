use std::path::PathBuf;

use drakeflake_flake::{Host, SystemKind};
use drakeflake_render::UserName;
use serde_json::json;

use super::SystemModel;
use super::runs::{Outcome, RunState};
use super::target::TargetState;
use crate::hub::{ConsoleMsg, HubError, InputsMsg, Mode, SetupMsg, SystemMsg, TargetInfo};

fn console_lines(model: &SystemModel) -> Vec<String> {
    model
        .hub
        .mail()
        .console
        .take()
        .into_iter()
        .flat_map(|message| match message {
            ConsoleMsg::Lines(lines) => lines,
            ConsoleMsg::Screen {
                session: _,
                changes: _,
            }
            | ConsoleMsg::Prompt(_) => Vec::new(),
        })
        .collect()
}

fn posted_flakes(model: &SystemModel) -> Vec<String> {
    model
        .hub
        .mail()
        .inputs
        .take()
        .into_iter()
        .filter_map(|message| match message {
            InputsMsg::Flake(flake) => Some(flake),
            InputsMsg::Lock(_)
            | InputsMsg::Checks {
                flake: _,
                checks: _,
            }
            | InputsMsg::Updated(_) => None,
        })
        .collect()
}

fn host(name: &str) -> Host {
    Host {
        name: name.to_owned(),
        kind: SystemKind::NixOs,
    }
}

#[test]
fn empty_settings_let_the_hub_choose() {
    let unset = TargetState::from_config(&drakeflake_config::Nix {
        flake_path: "/etc/nixos".to_owned(),
        host: None,
        managed_path: None,
    });

    assert_eq!((unset.host_setting(), unset.setting()), (None, None));

    let chosen = TargetState::from_config(&drakeflake_config::Nix {
        flake_path: "/etc/nixos".to_owned(),
        host: Some("box".into()),
        managed_path: Some("/etc/nixos/out.nix".to_owned()),
    });

    assert_eq!(
        (chosen.host_setting(), chosen.setting()),
        (
            Some("box".to_owned()),
            Some("/etc/nixos/out.nix".to_owned())
        )
    );
}

#[test]
fn restore_is_offered_only_after_a_failed_run_with_a_backup() {
    let backup = PathBuf::from("/tmp/backup.nix");
    let mut run = RunState::default();
    assert!(!run.can_restore());

    run.finished(false, Some(backup.clone()));
    assert!(run.can_restore());

    run.finished(false, None);
    assert_eq!(run.last_backup.as_ref(), Some(&backup));
    assert!(run.can_restore());

    run.running = true;
    assert!(!run.can_restore(), "not while another run is going");

    run.finished(true, None);
    assert!(run.last_result == Outcome::Ok);
    assert!(!run.can_restore());
}

#[test]
fn home_state_follows_the_check() {
    let mut model = SystemModel::default();
    assert_eq!(model.home_state(), "");

    let cases = [
        (vec![], None, "none"),
        (vec!["alice".into()], None, "available"),
        (
            vec![UserName::from("alice")],
            Some("eval failed"),
            "error: eval failed",
        ),
    ];

    for (users, error, expected) in cases {
        model.handle(SystemMsg::HomeUsers {
            users,
            selected: "alice".into(),
            error: error.map(str::to_owned),
        });
        assert_eq!(model.home_state(), expected);
        assert_eq!(model.home_user(), "alice");
    }
}

#[test]
fn finished_runs_report_their_outcome() {
    let mut model = SystemModel::default();
    model.handle(SystemMsg::Running(true));
    assert!(model.running());
    assert_eq!(model.last_result(), "");

    model.handle(SystemMsg::Finished {
        ok: false,
        backup: Some(PathBuf::from("/tmp/b.nix")),
    });
    assert!(!model.running());
    assert_eq!(model.last_result(), "failed");
    assert!(model.can_restore());
}

#[test]
fn a_target_error_then_the_loaded_target() {
    let mut model = SystemModel::default();

    model.handle(SystemMsg::TargetError {
        flake: "/flakes/a".to_owned(),
        hosts: vec![host("box")],
        error: "no host".to_owned(),
    });
    assert_eq!(posted_flakes(&model), ["/flakes/a"]);
    assert_eq!(
        (model.target_error(), model.target_ready(), model.kind()),
        ("no host".to_owned(), false, String::new())
    );
    assert_eq!(model.hosts(), json!([{ "name": "box", "kind": "nixos" }]));

    model.handle(SystemMsg::Target(TargetInfo {
        flake: "/flakes/a".to_owned(),
        host: "box".into(),
        kind: SystemKind::Darwin,
        hosts: vec![host("box")],
        managed_path: Some(PathBuf::from("/flakes/a/drakeflake.nix")),
    }));
    assert_eq!(posted_flakes(&model), Vec::<String>::new());
    assert_eq!(
        (
            model.target_error(),
            model.target_ready(),
            model.kind(),
            model.managed_path()
        ),
        (
            String::new(),
            true,
            "darwin".to_owned(),
            "/flakes/a/drakeflake.nix".to_owned()
        )
    );
    assert_eq!(model.host(), "box");
}

#[test]
fn invalid_targets_are_refused_and_the_old_one_kept() {
    let cases = [
        ("", "box", "", "The flake path is empty"),
        ("/flakes/b", "bad host!", "", "Unsupported host name"),
        ("/flakes/b", "", "relative.nix", "must be absolute"),
        ("/flakes/b", "", "/nowhere/out.txt", "must be a .nix file"),
    ];

    for (flake, host_name, managed, expected) in cases {
        let mut model = SystemModel::default();
        model.target.flake = "/flakes/a".to_owned();

        model.retarget(flake.to_owned(), host_name.to_owned(), managed.to_owned());

        let lines = console_lines(&model);
        assert!(
            lines.iter().any(|line| line.contains(expected)),
            "{flake:?} {host_name:?} {managed:?}: {lines:?}"
        );
        assert_eq!(model.flake(), "/flakes/a");
        assert_eq!(model.managed_setting(), "");
        assert!(
            model
                .hub
                .mail()
                .setup
                .take()
                .iter()
                .all(|message| !matches!(message, SetupMsg::TargetChanged))
        );
    }
}

#[test]
fn run_slots_report_why_they_cannot_start() {
    let mut model = SystemModel::default();

    let unknown_mode = model.rebuild("explode".to_owned());
    assert!(unknown_mode.contains("\"explode\" is not available"));

    let bad_list = model.update_lock(json!("nixpkgs"));
    assert!(bad_list.starts_with("Invalid input list"), "{bad_list}");

    let bad_name = model.update_lock(json!(["not an input!"]));
    assert!(bad_name.contains("is not an input name"), "{bad_name}");

    assert!(!model.running());
    assert_eq!(console_lines(&model), Vec::<String>::new());
}

#[test]
fn bad_generation_actions_go_to_the_console() {
    let mut model = SystemModel::default();

    model.generation_action(3, "explode".to_owned());
    model.generation_action(-1, "switch".to_owned());

    model.restore_backup();

    assert_eq!(
        console_lines(&model),
        [
            "Unknown generation action \"explode\"",
            "Invalid generation -1"
        ]
    );
    assert!(!model.running());
}

#[test]
fn started_runs_show_as_running() {
    let mut model = SystemModel::default();
    model.run.last_result = Outcome::Failed;

    assert_eq!(model.started(Err(HubError::invalid("busy"))), "busy");
    assert!(!model.running());

    model.run_started(Ok(()));
    assert!(model.running());
    assert_eq!(model.last_result(), "");
}

#[test]
fn a_refused_start_keeps_restore_on_offer() {
    let mut model = SystemModel::default();
    model.run.last_result = Outcome::Failed;
    model.run.last_backup = Some(PathBuf::from("/tmp/b.nix"));
    assert!(model.can_restore());

    assert_ne!(model.start_lock_update(&json!([])), "");
    assert_eq!(model.last_result(), "failed");
    assert!(model.can_restore());

    model.run_started(Err(HubError::Busy));
    assert!(model.can_restore());
}

#[test]
fn apply_plans_are_checked_before_anything_starts() {
    let model = SystemModel::default();
    let plan = json!({ "options": {}, "packages": ["git"] });

    let (parsed, mode) = model.parse_apply(&plan, "save", |can| can.apply).unwrap();
    assert_eq!(mode, Mode::Save);
    assert!(parsed.packages.contains("git"));

    assert!(model.parse_apply(&plan, "save", |can| can.review).is_none());
    assert!(
        model
            .parse_apply(&json!({ "options": 3_i32 }), "save", |can| can.apply)
            .is_none()
    );
    let lines = console_lines(&model);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].contains("\"save\" is not available"));
    assert!(lines[1].starts_with("Invalid plan"));
}
