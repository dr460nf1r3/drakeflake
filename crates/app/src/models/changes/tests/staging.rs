use drakeflake_flake::TypeCheck;
use drakeflake_plan::ManagedState;
use serde_json::{Value, json};

use super::model_with_managed;
use crate::hub::ChangesMsg;
use crate::models::changes::ChangeModel;

fn stage(model: &mut ChangeModel, name: &str, input: &str, settings: Value) -> String {
    model.stage_setting(
        name.to_owned(),
        "signed integer".to_owned(),
        input.to_owned(),
        settings,
    )
}

fn status(model: &ChangeModel, name: &str) -> String {
    model.check_status(name.to_owned())
}

#[test]
fn a_staged_value_is_type_checked_once_per_input() {
    let mut model = model_with_managed();

    assert_eq!(stage(&mut model, "b.port", "80", Value::Null), "");
    assert_eq!(model.option_state("b.port".to_owned()), "set");
    assert_eq!(status(&model, "b.port"), "pending");

    model.on_checked("b.port".into(), "typed:80".to_owned(), Ok(TypeCheck::Valid));
    assert_eq!(status(&model, "b.port"), "ok");

    assert_eq!(
        stage(&mut model, "b.port", "80", json!({ "priority": "force" })),
        ""
    );
    assert_eq!(status(&model, "b.port"), "ok");

    assert_eq!(stage(&mut model, "b.port", "81", Value::Null), "");
    assert_eq!(status(&model, "b.port"), "pending");
    assert_eq!(model.staged_input("b.port".to_owned()), "81");
}

#[test]
fn values_that_change_nothing_or_are_invalid_are_not_staged() {
    let mut model = model_with_managed();
    let same = model.stage_setting(
        "a.enable".to_owned(),
        "boolean".to_owned(),
        "true".to_owned(),
        Value::Null,
    );

    assert_eq!(same, "");
    assert_eq!(model.option_state("a.enable".to_owned()), "managed");
    assert_eq!(status(&model, "a.enable"), "");

    assert_eq!(
        stage(&mut model, "", "1", Value::Null),
        "No option selected"
    );
    let settings = stage(&mut model, "b.port", "1", json!({ "priority": 3_i32 }));
    assert!(settings.starts_with("Invalid settings"), "{settings}");
    assert_ne!(stage(&mut model, "b.port", "eighty", Value::Null), "");
    assert_eq!(model.option_state("b.port".to_owned()), "");
    assert_eq!(model.row_count(), 0_i32);
}

#[test]
fn checks_of_older_inputs_are_discarded() {
    let mut model = model_with_managed();
    stage(&mut model, "b.port", "80", Value::Null);
    stage(&mut model, "c.port", "1", Value::Null);

    model.on_checked("b.port".into(), "typed:79".to_owned(), Ok(TypeCheck::Valid));
    model.on_checked("gone".into(), "typed:1".to_owned(), Ok(TypeCheck::Valid));
    assert_eq!(
        (model.checks_pending(), model.checks_failed()),
        (2_i32, 0_i32)
    );

    model.on_checked(
        "c.port".into(),
        "typed:1".to_owned(),
        Err("eval broke".to_owned()),
    );
    assert_eq!(status(&model, "c.port"), "failed");
    assert_eq!(model.check_message("c.port".to_owned()), "eval broke");
    assert_eq!(
        (model.checks_pending(), model.checks_failed()),
        (1_i32, 1_i32)
    );
}

#[test]
fn packages_are_staged_and_unstaged() {
    let mut model = model_with_managed();
    let state = |changes: &ChangeModel, attr: &str| changes.package_state(attr.to_owned());

    assert_eq!(model.stage_package("pkgs.vim".to_owned(), true), "");
    assert_eq!(state(&model, "vim"), "add");
    assert_eq!(model.stage_package("git".to_owned(), true), "");
    assert_eq!(state(&model, "git"), "managed");
    model.stage_package("git".to_owned(), false);
    assert_eq!(state(&model, "git"), "remove");
    assert_ne!(model.stage_package("not a package".to_owned(), true), "");

    stage(&mut model, "b.port", "80", Value::Null);
    assert_eq!(model.row_count(), 3_i32);
    model.unstage("package:vim".to_owned());
    model.unstage("option:b.port".to_owned());
    model.unstage("bogus".to_owned());
    assert_eq!(state(&model, "vim"), "");
    assert_eq!(model.option_state("b.port".to_owned()), "");
    assert_eq!(model.row_count(), 1_i32);

    model.clear();
    assert_eq!(
        (model.row_count(), state(&model, "git")),
        (0_i32, "managed".to_owned())
    );
}

#[test]
fn resets_only_stick_to_managed_options() {
    let mut model = model_with_managed();
    stage(&mut model, "b.port", "80", Value::Null);

    model.stage_reset("b.port".into());
    assert_eq!(model.option_state("b.port".to_owned()), "");
    model.stage_reset("a.enable".into());
    assert_eq!(model.option_state("a.enable".to_owned()), "reset");

    assert_eq!(model.staged_input("a.enable".to_owned()), "true");

    let row = model.build_rows().into_iter().next().unwrap();
    assert_eq!(
        (
            row.key.as_str(),
            row.change_type.as_str(),
            row.before.as_str(),
            row.after.as_str()
        ),
        ("option:a.enable", "reset", "true", "")
    );
}

#[test]
fn restored_edits_never_replace_current_ones() {
    let mut model = model_with_managed();
    stage(&mut model, "b.port", "80", Value::Null);

    let mut saved = ChangeModel::default();
    stage(&mut saved, "b.port", "1", Value::Null);
    stage(&mut saved, "c.port", "2", Value::Null);
    saved.stage_package("vim".to_owned(), true);

    model.merge_staged(saved.changes.clone());

    assert_eq!(model.staged_input("b.port".to_owned()), "80");
    assert_eq!(model.staged_input("c.port".to_owned()), "2");
    assert_eq!(status(&model, "c.port"), "pending");
    assert_eq!(
        model.checks.get("b.port").map(|(input, _)| input.as_str()),
        Some("typed:80"),
        "the restored edit lost, so it must not replace the pending check"
    );
    assert_eq!(model.package_state("vim".to_owned()), "add");
}

#[test]
fn hub_messages_update_the_managed_state() {
    let mut model = model_with_managed();
    stage(&mut model, "b.port", "80", Value::Null);

    model.handle(ChangesMsg::Reset { keep_staged: true });
    assert!(!model.ready());
    assert_eq!(model.option_state("b.port".to_owned()), "set");

    model.handle(ChangesMsg::Managed(Err("unreadable".to_owned())));
    assert_eq!(
        (model.ready(), model.error()),
        (false, "unreadable".to_owned())
    );

    let mut saved = ManagedState::default();
    saved.packages.insert("htop".into());
    model.handle(ChangesMsg::Saved(saved));
    assert!(model.ready());
    assert_eq!(model.managed_count(), 1_i32);

    model.handle(ChangesMsg::Reset { keep_staged: false });
    assert_eq!(model.option_state("b.port".to_owned()), "");
    assert_eq!(model.checks_pending(), 0_i32);
}

#[test]
fn previews_explain_why_they_cannot_render() {
    let model = model_with_managed();
    let invalid = model.preview(json!({ "options": 3_i32 }));
    assert!(invalid.starts_with("# Invalid plan: "), "{invalid}");

    let no_system = model.preview(model.plan_json());
    assert_eq!(no_system, "# Cannot render:\n# No system is selected yet");
}
