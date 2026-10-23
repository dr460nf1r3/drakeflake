use std::sync::Arc;
use std::time::{Duration, Instant};

use drakeflake_store::OptionIndex;
use serde_json::json;

use super::values::{info, model};
use crate::hub::{Generation, OptionsMsg};
use crate::models::options::{OptionModel, View};

fn tree() -> OptionModel {
    let infos = vec![
        info("boot.enable", Some("false")),
        info("services.<name>.port", None),
        info("services.ssh.enable", Some("false")),
        info("services.ssh.port", Some("22")),
    ];

    let snapshot = r#"{ "services.ssh.port": { "v": 2222, "u": ["/etc/nixos/a.nix"] } }"#;
    let mut model = model(View::Tree, infos, snapshot);
    model.refresh();
    model.open_tree();
    finish_build(&mut model);
    model
}

fn finish_build(model: &mut OptionModel) {
    let deadline = Instant::now() + Duration::from_secs(10);

    while model.tree_building() {
        for message in model.view.mailbox(model.hub.mail()).take() {
            model.handle(message);
        }

        assert!(Instant::now() < deadline, "the tree build never arrived");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_builds(model: &OptionModel, count: usize) -> Vec<(u64, OptionsMsg)> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut builds = Vec::new();

    while builds.len() < count {
        for message in model.view.mailbox(model.hub.mail()).take() {
            if let OptionsMsg::TreeBuilt { number, tree: _ } = &message {
                builds.push((*number, message));
            }
        }

        assert!(
            Instant::now() < deadline,
            "only {} builds arrived",
            builds.len()
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    builds.sort_by_key(|(number, _)| std::cmp::Reverse(*number));
    builds
}

fn labels(model: &OptionModel) -> Vec<&str> {
    model.rows.iter().map(|row| row.label.as_str()).collect()
}

#[test]
fn the_tree_waits_for_its_page() {
    let mut model = model(View::Tree, vec![info("boot.enable", None)], "{}");
    model.refresh();

    assert!(model.rows.is_empty() && !model.tree_building());
    assert!(model.view.mailbox(model.hub.mail()).take().is_empty());

    model.open_tree();
    assert!(model.tree_building());
    finish_build(&mut model);
    assert_eq!(labels(&model), ["boot", "enable"]);
}

#[test]
fn an_older_build_arriving_last_is_dropped() {
    let mut model = tree();
    let index = |names: &[&str]| {
        OptionsMsg::Index(Arc::new(OptionIndex::from_entries(
            names.iter().map(|name| info(name, None)).collect(),
        )))
    };

    model.handle(index(&["old.option"]));
    model.refresh();
    model.handle(index(&["new.a", "new.b"]));
    model.refresh();

    for (_, message) in wait_for_builds(&model, 2) {
        model.handle(message);
    }

    assert!(!model.tree_building());
    assert_eq!(labels(&model), ["new", "a", "b"]);
}

#[test]
fn values_evaluated_during_a_build_are_kept() {
    let mut model = model(View::Tree, vec![info("boot.enable", Some("false"))], "{}");
    model.refresh();
    model.open_tree();

    model.handle(OptionsMsg::Value(
        "boot.enable".into(),
        Ok(json!(true)),
        Generation::default(),
    ));
    model.refresh_values();
    finish_build(&mut model);

    assert_eq!(
        shown(&model).get(1),
        Some(&("boot.enable".to_owned(), "true".to_owned()))
    );
}

fn shown(model: &OptionModel) -> Vec<(String, String)> {
    model
        .rows
        .iter()
        .map(|row| (row.name.clone(), row.value.clone()))
        .collect()
}

#[test]
fn the_tree_lists_folders_and_values_without_templates() {
    let model = tree();
    let node = |name: &str| model.rows.iter().find(|row| row.name == name).unwrap();

    assert_eq!(
        model
            .rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        ["boot", "enable", "services", "ssh", "enable", "port"]
    );
    assert_eq!(node("services.ssh.port").value, "2222");
    assert!(node("services.ssh.port").user_set);
    assert_eq!(node("services.ssh.enable").value, "false");
    assert_eq!(
        (
            node("services.ssh").depth,
            node("services.ssh").option_count
        ),
        (1_i32, 2_i32)
    );
    assert!(!node("services.ssh").is_option);
    assert!(model.rows.iter().all(|row| row.open));
    assert_eq!(
        model.top_folders().collect::<Vec<_>>(),
        ["boot", "services"]
    );
}

#[test]
fn values_arriving_later_update_nodes_in_place() {
    let mut model = tree();
    model.collapsed.insert("boot".to_owned());
    model.show_tree();

    model.handle(OptionsMsg::Value(
        "boot.enable".into(),
        Ok(json!(true)),
        Generation::default(),
    ));
    model.handle(OptionsMsg::Value(
        "services.ssh.enable".into(),
        Ok(json!(true)),
        Generation::default(),
    ));
    model.handle(OptionsMsg::Value(
        "not.in.tree".into(),
        Ok(json!(1_i32)),
        Generation::default(),
    ));
    model.refresh_values();

    assert_eq!(
        shown(&model),
        [
            ("boot".to_owned(), String::new()),
            ("services".to_owned(), String::new()),
            ("services.ssh".to_owned(), String::new()),
            ("services.ssh.enable".to_owned(), "true".to_owned()),
            ("services.ssh.port".to_owned(), "2222".to_owned()),
        ]
    );
    assert!(!model.rows.first().unwrap().open);
    assert_eq!(model.changed_values, Vec::<String>::new());

    model.collapsed.clear();
    model.show_tree();
    assert_eq!(
        shown(&model).get(1),
        Some(&("boot.enable".to_owned(), "true".to_owned()))
    );
}
