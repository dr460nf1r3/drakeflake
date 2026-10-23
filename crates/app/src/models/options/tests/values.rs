use std::collections::HashSet;
use std::sync::Arc;

use drakeflake_render::OptionName;
use drakeflake_store::{OptionIndex, OptionInfo, ValueSnapshot};
use serde_json::json;

use crate::hub::{Generation, HomeEpoch, OptionsMsg};
use crate::models::options::values::Refresh;
use crate::models::options::{OptionModel, View};

pub fn info(name: &str, default: Option<&str>) -> OptionInfo {
    OptionInfo {
        name: name.into(),
        type_name: "signed integer".to_owned(),
        default_text: default.map(str::to_owned),
        templated: name.contains('<'),
        ..OptionInfo::default()
    }
}

pub fn model(view: View, infos: Vec<OptionInfo>, snapshot: &str) -> OptionModel {
    let mut model = OptionModel {
        view,
        ..OptionModel::default()
    };

    model.handle(OptionsMsg::Index(Arc::new(OptionIndex::from_entries(
        infos,
    ))));
    model.handle(OptionsMsg::Snapshot(Arc::new(
        ValueSnapshot::from_json_str(snapshot).unwrap(),
    )));
    model
}

fn value_of(model: &OptionModel, name: &str) -> String {
    model.value_text(model.index.get(&OptionName::from(name)).unwrap())
}

fn shown(model: &OptionModel) -> Vec<String> {
    model.rows.iter().map(|row| row.name.clone()).collect()
}

#[test]
fn value_column_prefers_known_values_over_defaults() {
    let infos = vec![
        info("a.set", Some("1")),
        info("a.default", Some("42")),
        info("a.broken", Some("1")),
        info("a.evaluated", Some("1")),
        info("a.failed", Some("1")),
    ];

    let snapshot = r#"{
        "a.set": { "v": 7, "u": ["/etc/nixos/a.nix"] },
        "a.default": {},
        "a.broken": { "v": { "_drakeflakeError": "boom" }, "u": ["/etc/nixos/a.nix"] }
    }"#;

    let mut model = model(View::Search, infos.clone(), snapshot);
    model.handle(OptionsMsg::Value(
        "a.evaluated".into(),
        Ok(json!(8_i32)),
        Generation::default(),
    ));
    model.handle(OptionsMsg::Value(
        "a.failed".into(),
        Err("no".to_owned()),
        Generation::default(),
    ));

    let cases = [
        ("a.set", "7"),
        ("a.default", "42"),
        ("a.broken", ""),
        ("a.evaluated", "8"),
        ("a.failed", ""),
    ];

    for (name, expected) in cases {
        assert_eq!(value_of(&model, name), expected, "{name}");
    }

    let mut loading = OptionModel::default();
    loading.handle(OptionsMsg::Index(Arc::new(OptionIndex::from_entries(
        infos,
    ))));
    assert_eq!(value_of(&loading, "a.default"), "");
}

#[test]
fn rows_name_the_files_that_define_the_option() {
    let snapshot = r#"{
        "a.one": { "v": 1, "u": ["/nix/store/abc-source/hosts/a.nix"] },
        "a.two": { "v": 2, "u": ["/etc/nixos/a.nix", "/etc/nixos/b.nix", "/etc/nixos/c.nix"] }
    }"#;

    let infos = vec![
        info("a.none", None),
        info("a.one", None),
        info("a.two", None),
        info("a.<name>.x", None),
    ];

    let model = model(View::Search, infos, snapshot);
    let row = |name: &str| model.row_for(model.index.get(&OptionName::from(name)).unwrap());

    assert_eq!(row("a.none").defined_in, "");
    assert_eq!(row("a.one").defined_in, "hosts/a.nix");
    assert_eq!(row("a.two").defined_in, "/etc/nixos/a.nix (+2)");
    assert!(row("a.two").user_set);
    assert!(!row("a.none").user_set);
    assert!(
        row("a.<name>.x").read_only,
        "templated options are not settable"
    );
}

#[test]
fn home_manager_areas_skip_the_user_prefix() {
    let option = info("home-manager.users.alice.programs.git.enable", None);
    assert_eq!(OptionModel::namespace_of(View::Home, &option), "programs");
    assert_eq!(
        OptionModel::namespace_of(View::Search, &option),
        "home-manager"
    );
    assert_eq!(
        OptionModel::namespace_of(View::HomeTree, &info("home-manager.users", None)),
        ""
    );
}

#[test]
fn search_filters_by_area_configured_and_templated() {
    let infos = vec![
        info("boot.loader", None),
        info("services.<name>.c", None),
        info("services.a", None),
        info("services.b", None),
    ];

    let snapshot = r#"{ "services.b": { "v": 1, "u": ["/etc/nixos/a.nix"] } }"#;
    let mut model = model(View::Search, infos, snapshot);

    model.refresh();
    assert_eq!(shown(&model), ["boot.loader", "services.a", "services.b"]);

    model.namespace = "services".to_owned();
    model.show_templated = true;
    model.refresh();
    assert_eq!(
        shown(&model),
        ["services.<name>.c", "services.a", "services.b"]
    );

    model.only_user = true;
    model.refresh();
    assert_eq!(shown(&model), ["services.b"]);
}

#[test]
fn overview_matches_leave_the_list_alone() {
    let infos = vec![info("services.a", None), info("services.b", None)];
    let snapshot = r#"{ "services.b": { "v": 1, "u": ["/etc/nixos/a.nix"] } }"#;
    let search = model(View::Search, infos.clone(), snapshot);
    let configured = model(View::Configured, infos, snapshot);

    assert!(search.top_matches("", 5) == (0, Vec::new()));
    let (count, rows) = search.top_matches("services", 1);
    assert_eq!((count, rows.len()), (2, 1));
    assert_eq!(configured.top_matches("services", 5).0, 1);
    assert_eq!(shown(&search), Vec::<String>::new());
}

#[test]
fn messages_ask_for_the_least_work_that_shows_them() {
    let snapshot = || Arc::new(ValueSnapshot::default());
    let mut search = model(View::Search, Vec::new(), "{}");
    let mut tree = model(View::Tree, Vec::new(), "{}");

    assert!(search.handle(OptionsMsg::Snapshot(snapshot())) == Refresh::Values);
    assert!(tree.handle(OptionsMsg::Snapshot(snapshot())) == Refresh::Search);
    search.only_user = true;
    assert!(search.handle(OptionsMsg::Snapshot(snapshot())) == Refresh::Search);
    assert!(search.handle(OptionsMsg::SnapshotError("x".to_owned())) == Refresh::Nothing);
    assert!(search.handle(OptionsMsg::IndexError("y".to_owned())) == Refresh::Nothing);
    assert_eq!(
        (search.values_error.as_str(), search.index_error.as_str()),
        ("x", "y")
    );

    search.pending.borrow_mut().insert("a".into());
    assert!(
        search.handle(OptionsMsg::Value(
            "a".into(),
            Ok(json!(1_i32)),
            Generation::default()
        )) == Refresh::Values
    );
    assert_eq!(*search.pending.borrow(), HashSet::new());
    assert!(
        tree.handle(OptionsMsg::Value(
            "a".into(),
            Ok(json!(1_i32)),
            Generation::default()
        )) == Refresh::Values
    );
    assert_eq!(tree.changed_values, [OptionName::from("a")]);

    assert!(search.handle(OptionsMsg::Reset) == Refresh::Search);
    assert!(!search.has_snapshot);
    assert_eq!(search.values_error, "");
}

#[test]
fn a_new_snapshot_drops_values_evaluated_on_demand() {
    let new_snapshot =
        || Arc::new(ValueSnapshot::from_json_str(r#"{ "kept": { "v": 1 } }"#).unwrap());

    let mut configured = model(View::Configured, Vec::new(), "{}");
    configured.handle(OptionsMsg::Value(
        "gone".into(),
        Ok(json!(1_i32)),
        Generation::default(),
    ));
    configured.handle(OptionsMsg::Snapshot(new_snapshot()));
    assert!(configured.evaluated.is_empty());
    assert_eq!(
        *configured.pending.borrow(),
        HashSet::new(),
        "the search view asks"
    );

    let mut search = model(View::Search, Vec::new(), "{}");
    search.handle(OptionsMsg::Value(
        "gone".into(),
        Ok(json!(1_i32)),
        Generation::default(),
    ));
    search.handle(OptionsMsg::Value(
        "kept".into(),
        Ok(json!(1_i32)),
        Generation::default(),
    ));
    search.handle(OptionsMsg::Snapshot(new_snapshot()));
    assert!(search.evaluated.is_empty());
    assert_eq!(
        *search.pending.borrow(),
        HashSet::from([OptionName::from("gone")])
    );
}

#[test]
fn details_report_where_the_value_stands() {
    let infos = vec![
        info("a.failed", None),
        info("a.broken", None),
        info("a.ready", None),
        info("a.unknown", None),
        info("a.loading", None),
    ];

    let snapshot = r#"{ "a.broken": { "v": { "_drakeflakeError": "x" } }, "a.ready": { "v": 5 } }"#;
    let mut model = model(View::Search, infos, snapshot);
    model.handle(OptionsMsg::Value(
        "a.failed".into(),
        Err("boom".to_owned()),
        Generation::default(),
    ));
    model.pending.borrow_mut().insert("a.loading".into());

    let state = |name: &str| {
        let details = model.details_value(&OptionName::from(name));
        (
            details["valueState"].clone(),
            details["valueDisplay"].clone(),
            details["valueError"].clone(),
        )
    };

    let fails = "This option fails to evaluate in the current configuration";
    assert_eq!(
        state("a.failed"),
        (json!("error"), json!(""), json!("boom"))
    );
    assert_eq!(state("a.broken"), (json!("error"), json!(""), json!(fails)));
    assert_eq!(state("a.ready"), (json!("ready"), json!("5"), json!("")));

    assert_eq!(state("a.unknown"), (json!("unknown"), json!(""), json!("")));
    assert_eq!(state("a.loading"), (json!("loading"), json!(""), json!("")));
    assert_eq!(model.details_value(&"a.ready".into())["valueInput"], "5");
    assert_eq!(model.details_value(&"nope".into()), json!({}));
}

#[test]
fn arriving_values_update_the_shown_rows() {
    let mut model = model(View::Search, vec![info("a.x", Some("1"))], "{}");
    model.refresh();
    assert_eq!(model.rows.first().map(|row| row.value.as_str()), Some("1"));

    model.handle(OptionsMsg::Value(
        "a.x".into(),
        Ok(json!(9_i32)),
        Generation::default(),
    ));
    model.refresh_values();
    assert_eq!(model.rows.first().map(|row| row.value.as_str()), Some("9"));
    assert!(model.revision > 0_i32);
}

#[test]
fn values_of_a_replaced_load_are_dropped() {
    let mut model = model(View::Search, vec![info("a.x", Some("1"))], "{}");
    model.pending.borrow_mut().insert("a.x".into());

    let refresh = model.handle(OptionsMsg::Value(
        "a.x".into(),
        Ok(json!(9_i32)),
        Generation(7),
    ));

    assert!(refresh == Refresh::Nothing);
    assert!(model.evaluated.is_empty());
    assert!(model.pending.borrow().contains("a.x"));

    model.handle(OptionsMsg::Value(
        "a.x".into(),
        Ok(json!(9_i32)),
        Generation::default(),
    ));
    assert_eq!(model.evaluated.len(), 1);
}

#[test]
fn tree_slots_leave_flat_views_alone() {
    let mut model = model(View::Search, vec![info("a.x", Some("1"))], "{}");
    model.refresh();
    assert_eq!(shown(&model), ["a.x"]);

    model.toggle_folder("a".to_owned());
    model.collapse_all();
    model.expand_all();
    model.open_tree();
    assert_eq!(shown(&model), ["a.x"]);
}

#[test]
fn answers_for_the_previous_home_user_are_dropped() {
    let mut model = model(View::Home, vec![info("a.x", Some("1"))], "{}");
    model.pending.borrow_mut().insert("a.x".into());

    let home_value = |epoch: HomeEpoch| OptionsMsg::HomeValue {
        name: "a.x".into(),
        result: Ok(json!(9_i32)),
        generation: Generation::default(),
        epoch,
    };

    assert!(
        model.handle(OptionsMsg::HomeReset {
            epoch: HomeEpoch(2)
        }) == Refresh::Search
    );
    model.pending.borrow_mut().insert("a.x".into());

    assert!(model.handle(home_value(HomeEpoch(1))) == Refresh::Nothing);
    assert!(model.evaluated.is_empty());
    assert!(model.pending.borrow().contains("a.x"));

    assert!(model.handle(home_value(HomeEpoch(2))) == Refresh::Values);
    assert_eq!(model.evaluated.len(), 1);
}
