use std::path::PathBuf;

use serde_json::json;

use super::{SetupModel, effective_folder, normalize_folder, picked_value, resolve_existing};
use crate::hub::SetupMsg;
use crate::hub::portal::{Outcome, Picked};
use crate::hub::setup::ImportCheck;

fn post(model: &mut SetupModel, message: SetupMsg) {
    model.hub.mail().setup.post(message);
    model.drain();
}

#[test]
fn the_import_check_is_forgotten_with_the_target() {
    let mut model = SetupModel::default();
    let state = |setup: &SetupModel| (setup.import_status(), setup.import_error());

    post(
        &mut model,
        SetupMsg::ImportCheck(ImportCheck::Failed("eval".to_owned())),
    );
    assert_eq!(state(&model), ("failed".to_owned(), "eval".to_owned()));
    post(&mut model, SetupMsg::ImportCheck(ImportCheck::Missing));
    assert_eq!(state(&model), ("missing".to_owned(), String::new()));
    post(&mut model, SetupMsg::TargetChanged);
    assert_eq!(state(&model), (String::new(), String::new()));
}

#[test]
fn scans_and_choosers_report_through_the_mailbox() {
    let mut model = SetupModel::default();
    assert_eq!(model.flake_candidates(), json!([]));
    post(
        &mut model,
        SetupMsg::FlakeCandidates(vec![PathBuf::from("/etc/nixos")]),
    );
    assert_eq!(model.flake_candidates(), json!(["/etc/nixos"]));
    assert!(!model.flake_scanning());

    let picked = || Picked {
        purpose: "flake".to_owned(),
        outcome: Outcome::Cancelled,
    };

    post(&mut model, SetupMsg::Picked(picked()));
    post(&mut model, SetupMsg::Picked(picked()));
    assert_eq!(
        model.picked()["serial"],
        2_i32,
        "every answer is new to QML"
    );
}

#[test]
fn folder_slots_work_on_absolute_paths_only() {
    let model = SetupModel::default();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("flake.nix"), "{ }").unwrap();
    let hosts = root.join("hosts/new");
    let text = |path: &std::path::Path| path.display().to_string();

    assert_eq!(model.flake_root(text(&hosts)), text(&root));
    assert_eq!(model.flake_root("relative/dir".to_owned()), "");
    assert!(model.folder_in_flake(text(&hosts), text(&root)));
    assert!(!model.folder_in_flake("hosts".to_owned(), text(&root)));

    assert_eq!(
        model.create_folder("hosts".to_owned()),
        "\"hosts\" is not an absolute path."
    );
    assert_eq!(model.create_folder(text(&hosts)), "");
    assert!(model.folder_exists(format!(" {} ", text(&hosts))));
    let blocked = root.join("flake.nix/inner");
    assert!(
        model
            .create_folder(text(&blocked))
            .starts_with("Cannot create ")
    );
}

#[test]
fn folders_compare_through_symlinks_and_missing_parts() {
    let real = tempfile::tempdir().unwrap();
    let links = tempfile::tempdir().unwrap();
    let flake = real.path().join("infra");
    std::fs::create_dir_all(flake.join("modules")).unwrap();
    let link = links.path().join("Projects");
    std::os::unix::fs::symlink(real.path(), &link).unwrap();

    let folder = PathBuf::from(format!("{}/infra/modules/system/", link.display()));

    assert!(resolve_existing(&folder).starts_with(resolve_existing(&flake)));
    assert!(!resolve_existing(links.path()).starts_with(resolve_existing(&flake)));
}

#[test]
fn picked_keeps_its_json_shape() {
    let cases = [
        (
            Outcome::Chosen(PathBuf::from("/etc/nixos")),
            Some("/etc/nixos"),
            None,
        ),
        (Outcome::Cancelled, None, None),
        (
            Outcome::Unavailable("no portal".to_owned()),
            None,
            Some("no portal"),
        ),
    ];

    for (outcome, path, error) in cases {
        let picked = Picked {
            purpose: "flake".to_owned(),
            outcome,
        };

        let row = picked_value(3, picked);

        let by_hand = serde_json::json!({
            "serial": 3_u64,
            "purpose": "flake",
            "path": path,
            "error": error,
        });

        assert_eq!(row, by_hand);
    }
}

#[test]
fn folders_are_normalised_like_before() {
    assert_eq!(normalize_folder("  /etc/nixos//  "), "/etc/nixos");
    assert_eq!(normalize_folder("/"), "");
    assert_eq!(normalize_folder("relative/"), "relative");

    assert_eq!(effective_folder("  ", "/etc/nixos"), "/etc/nixos");
    assert_eq!(
        effective_folder("/etc/nixos/hosts/", "/etc/nixos"),
        "/etc/nixos/hosts"
    );
    assert_eq!(effective_folder("/", "/etc/nixos"), "");
}
