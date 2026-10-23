use drakeflake_plan::{InputMode, ManagedOption, ManagedState, Priority};
use drakeflake_render::OptionName;
use serde::Deserialize as _;
use serde_json::{Value, json};

use super::{SettingsInput, Staged, StagedChanges, check_key, parse_mode, priority_choices};

fn managed() -> ManagedState {
    let mut state = ManagedState::default();
    state
        .options
        .insert("a.enable".into(), ManagedOption::typed("boolean", "true"));
    state.packages.insert("git".into());
    state
}

fn stage(
    changes: &mut StagedChanges,
    managed: &ManagedState,
    name: &str,
    input: &str,
) -> Result<bool, String> {
    changes
        .stage_option(
            &name.into(),
            ManagedOption::typed("signed integer", input),
            &Value::Null,
            managed,
        )
        .map(|check| check.is_some())
        .map_err(|error| error.to_string())
}

fn parse(json: &str) -> SettingsInput {
    let value: Value = serde_json::from_str(json).unwrap();
    SettingsInput::deserialize(&value).unwrap()
}

#[test]
fn plan_applies_staged_changes() {
    let managed = managed();
    let mut changes = StagedChanges::default();
    changes.stage_reset(&"a.enable".into(), &managed);
    stage(&mut changes, &managed, "b.port", "80").unwrap();
    changes.stage_package("git", false, &managed).unwrap();
    changes.stage_package("pkgs.vim", true, &managed).unwrap();

    let plan = changes.plan(&managed);
    assert!(!plan.options.contains_key("a.enable"));
    assert_eq!(plan.options["b.port"].input, "80");
    assert!(!plan.packages.contains("git"));
    assert!(plan.packages.contains("vim"));
    assert!(
        managed.packages.contains("git"),
        "the saved state is untouched"
    );
}

#[test]
fn settings_are_validated_and_normalized() {
    let typed = ManagedOption::typed("boolean", "true");

    let option = parse(r#"{"priority":"force","host":" box "}"#)
        .apply("services.x.enable", typed.clone())
        .unwrap();

    assert_eq!(option.priority, Some(Priority::Force));
    assert_eq!(option.host.as_deref(), Some("box"));

    let implicit = parse(r#"{"priority":"override"}"#)
        .apply("services.x.enable", typed.clone())
        .unwrap();

    assert_eq!(
        implicit.priority, None,
        "the implicit priority is stored as none chosen"
    );

    let error = |json: &str| {
        parse(json)
            .apply("a", typed.clone())
            .unwrap_err()
            .to_string()
    };

    assert_eq!(error(r#"{"priority":"loud"}"#), "Unknown priority \"loud\"");
    assert_eq!(error(r#"{"mode":"weird"}"#), "Unknown input mode \"weird\"");
    assert!(error(r#"{"priority":"before"}"#).contains("does not apply"));
    assert!(
        parse(r#"{"host":"box"}"#)
            .apply("networking.hostName", ManagedOption::typed("string", "box"))
            .is_err(),
        "the host name cannot depend on itself"
    );

    let expression = parse(r#"{"mode":"expression"}"#)
        .apply("a", ManagedOption::typed("string", "config.x"))
        .unwrap();

    assert_eq!(expression.mode, InputMode::Expression);
    assert_ne!(check_key(&expression), check_key(&typed));
}

#[test]
fn modes_go_through_one_parser() {
    assert_eq!(parse_mode("").unwrap(), InputMode::Typed);
    assert_eq!(parse_mode("expression").unwrap(), InputMode::Expression);
    assert_eq!(
        parse_mode("weird").unwrap_err().to_string(),
        "Unknown input mode \"weird\""
    );

    let typed = priority_choices("list of string", "", InputMode::Typed);
    assert_eq!(typed.implicit, Priority::Merge);
    assert!(typed.choices.contains(&Priority::Before));
}

#[test]
fn invalid_requests_are_refused_with_their_message() {
    let managed = managed();
    let mut changes = StagedChanges::default();

    assert_eq!(
        stage(&mut changes, &managed, "", "1").unwrap_err(),
        "No option selected"
    );
    let settings = changes
        .stage_option(
            &"b.port".into(),
            ManagedOption::typed("signed integer", "1"),
            &json!({ "priority": 3_i32 }),
            &managed,
        )
        .unwrap_err()
        .to_string();

    assert!(settings.starts_with("Invalid settings"), "{settings}");
    stage(&mut changes, &managed, "b.port", "eighty").unwrap_err();
    changes
        .stage_package("not a package", true, &managed)
        .unwrap_err();
    assert!(changes.is_empty());
}

#[test]
fn normalize_drops_an_edit_equal_to_the_managed_value() {
    let managed = managed();
    let mut changes = StagedChanges::default();

    let same = changes
        .stage_option(
            &"a.enable".into(),
            ManagedOption::typed("boolean", "true"),
            &Value::Null,
            &managed,
        )
        .unwrap();

    assert_eq!(same, None);
    assert!(changes.is_empty());
    assert_eq!(
        changes.option_state(&"a.enable".into(), &managed),
        "managed"
    );

    assert!(stage(&mut changes, &managed, "b.port", "1").unwrap());
    assert_eq!(changes.option_state(&"b.port".into(), &managed), "set");
}

#[test]
fn normalize_drops_no_ops() {
    let managed = managed();
    let mut changes = StagedChanges::default();
    changes.options.insert(
        "a.enable".into(),
        Staged::Set(ManagedOption::typed("boolean", "true")),
    );
    changes
        .options
        .insert("never.managed".into(), Staged::Reset);
    changes.packages.insert("git".into(), Staged::Add);
    changes.packages.insert("absent".into(), Staged::Remove);

    changes.normalize(&managed);

    assert!(changes.is_empty());
}

#[test]
fn resets_only_stick_to_managed_options() {
    let managed = managed();
    let mut changes = StagedChanges::default();
    stage(&mut changes, &managed, "b.port", "80").unwrap();

    changes.stage_reset(&"b.port".into(), &managed);
    assert_eq!(changes.option_state(&"b.port".into(), &managed), "");
    changes.stage_reset(&"a.enable".into(), &managed);
    assert_eq!(changes.option_state(&"a.enable".into(), &managed), "reset");
    assert_eq!(
        changes
            .current_option(&"a.enable".into(), &managed)
            .map(|option| option.input.as_str()),
        Some("true"),
        "the saved value stays the editor's starting point"
    );
}

#[test]
fn packages_are_staged_and_unstaged() {
    let managed = managed();
    let mut changes = StagedChanges::default();

    changes.stage_package("pkgs.vim", true, &managed).unwrap();
    assert_eq!(changes.package_state(&"vim".into(), &managed), "add");
    changes.stage_package("git", true, &managed).unwrap();
    assert_eq!(changes.package_state(&"git".into(), &managed), "managed");
    changes.stage_package("git", false, &managed).unwrap();
    assert_eq!(changes.package_state(&"git".into(), &managed), "remove");

    assert!(changes.unstage("package:vim"));
    assert!(!changes.unstage("bogus"));
    assert_eq!(changes.package_state(&"vim".into(), &managed), "");
    changes.clear();
    assert!(changes.is_empty());
}

#[test]
fn restored_edits_never_replace_current_ones() {
    let managed = managed();
    let mut changes = StagedChanges::default();
    stage(&mut changes, &managed, "b.port", "80").unwrap();

    let mut saved = StagedChanges::default();
    stage(&mut saved, &managed, "b.port", "1").unwrap();
    stage(&mut saved, &managed, "c.port", "2").unwrap();
    saved.stage_package("vim", true, &managed).unwrap();

    let restored = StagedChanges::from_value(&serde_json::from_str(&saved.to_json()).unwrap());
    assert_eq!(restored, saved);

    let checks = changes.merge(restored, &managed);

    assert_eq!(
        changes
            .staged_option(&OptionName::from("b.port"))
            .map(|option| option.input.as_str()),
        Some("80")
    );
    assert_eq!(
        changes
            .staged_option(&OptionName::from("c.port"))
            .map(|option| option.input.as_str()),
        Some("2")
    );
    assert_eq!(
        checks
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["c.port"],
        "only the entry that was added needs a check"
    );
    assert_eq!(changes.package_state(&"vim".into(), &managed), "add");
}

#[test]
fn broken_saved_edits_are_ignored() {
    let cases = [
        json!({}),
        json!({ "options": [1_i32, 2_i32] }),
        json!({ "options": { "a": "Explode" } }),
    ];

    for saved in cases {
        assert!(StagedChanges::from_value(&saved).is_empty(), "{saved}");
    }
}

#[test]
fn one_broken_entry_keeps_the_good_ones() {
    let managed = managed();
    let mut saved = StagedChanges::default();
    stage(&mut saved, &managed, "b.port", "1").unwrap();
    let mut value: Value = serde_json::from_str(&saved.to_json()).unwrap();
    value["options"]["broken"] = json!("Explode");

    let restored = StagedChanges::from_value(&value);
    assert!(
        restored
            .staged_option(&OptionName::from("b.port"))
            .is_some(),
        "{value}"
    );
    assert_eq!(restored.options().len(), 1, "{value}");
}

#[test]
fn saving_and_loading_round_trips_and_empty_removes_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/staged.json");
    assert_eq!(StagedChanges::load(&path).unwrap(), None);

    let managed = managed();
    let mut changes = StagedChanges::default();
    stage(&mut changes, &managed, "b.port", "80").unwrap();
    changes.save(&path).unwrap();
    assert_eq!(StagedChanges::load(&path).unwrap(), Some(changes));

    StagedChanges::default().save(&path).unwrap();
    assert!(!path.exists());
    StagedChanges::default().save(&path).unwrap();

    std::fs::write(&path, "not json").unwrap();
    StagedChanges::load(&path).unwrap_err();
}
