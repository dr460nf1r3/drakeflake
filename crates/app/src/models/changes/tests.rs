mod staging;

use drakeflake_flake::TypeCheck;
use drakeflake_plan::{ManagedOption, ManagedState};
use serde::Deserialize as _;
use serde_json::Value;

use super::ChangeModel;
use super::check::CheckState;

fn model_with_managed() -> ChangeModel {
    let mut model = ChangeModel::default();
    model
        .managed
        .options
        .insert("a.enable".into(), ManagedOption::typed("boolean", "true"));
    model.managed.packages.insert("git".into());
    model
}

#[test]
fn the_plan_is_what_the_slot_hands_to_apply() {
    let mut model = model_with_managed();
    model.stage_reset("a.enable".into());
    model.stage_package("vim".into(), true);

    let plan = model.plan();
    assert!(!plan.options.contains_key("a.enable"));
    assert!(plan.packages.contains("vim"));

    assert_eq!(ManagedState::deserialize(&model.plan_json()).unwrap(), plan);
}

#[test]
fn rows_show_the_settings() {
    let mut model = model_with_managed();
    model.stage_setting(
        "b.ports".into(),
        "list of string".into(),
        "[ \"x\" ]".into(),
        serde_json::json!({ "priority": "before", "mode": "expression", "host": "box" }),
    );
    let rows = model.build_rows();
    let row = rows.first().unwrap();
    assert_eq!(
        (row.priority.as_str(), row.mode.as_str(), row.host.as_str()),
        ("before", "expression", "box")
    );
    let settings = model.staged_settings("b.ports".into());
    assert_eq!(settings["effectivePriority"], "before");
    assert_eq!(model.staged_settings("none".into()), serde_json::json!({}));

    let choices = model.priority_choices("list of string".into(), String::new(), "typed".into());
    assert_eq!(choices["implicit"], "merge");
    assert_eq!(choices["choices"][1], "before");

    let unknown = model.priority_choices("list of string".into(), String::new(), "weird".into());
    assert_eq!(unknown, choices, "an unknown mode offers the typed choices");
}

#[test]
fn check_states() {
    assert_eq!(CheckState::Pending.to_string(), "pending");
    assert!(CheckState::Pending.blocks());
    assert!(!CheckState::Done(TypeCheck::Valid).blocks());
    assert!(!CheckState::Done(TypeCheck::Unknown).blocks());
    assert!(CheckState::Done(TypeCheck::Invalid("boolean".into())).blocks());
    assert_eq!(
        CheckState::Done(TypeCheck::Invalid("boolean".into())).message(),
        "Not a valid boolean"
    );
    assert!(CheckState::Error("boom".into()).blocks());
}

#[test]
fn stale_checks_are_dropped_by_normalize() {
    let mut model = model_with_managed();
    model.stage_setting(
        "b.port".into(),
        "signed integer".into(),
        "80".into(),
        Value::Null,
    );
    model
        .checks
        .insert("b.port".into(), ("70".into(), CheckState::Pending));
    model
        .checks
        .insert("gone".into(), ("1".into(), CheckState::Pending));

    model.normalize();

    assert_eq!(model.checks, std::collections::HashMap::default());
}

#[test]
fn managed_rows_keep_their_json_shape() {
    let model = model_with_managed();
    let rows = model.managed_list();

    let options = model.managed.options.iter().map(|(name, option)| {
        serde_json::json!({ "kind": "option", "name": name, "value": crate::models::one_line(&option.input, 80) })
    });

    let packages = model
        .managed
        .packages
        .iter()
        .map(|attr| serde_json::json!({ "kind": "package", "name": attr, "value": "" }));

    assert_eq!(rows, Value::Array(options.chain(packages).collect()));
}

#[cfg(unix)]
#[test]
fn staged_key_survives_symlinked_flakes() {
    let flake = tempfile::tempdir().unwrap();
    let link = flake.path().join("link");
    std::os::unix::fs::symlink(flake.path(), &link).unwrap();

    let plain = super::persist::staged_key(flake.path().to_str().unwrap(), "host");
    let linked = super::persist::staged_key(link.to_str().unwrap(), "host");
    assert_eq!(plain, linked);
    assert_ne!(
        plain,
        super::persist::staged_key("/nonexistent-dir-xyz", "host")
    );
}
