use std::collections::HashSet;
use std::sync::Arc;

use drakeflake_render::PackageAttr;
use drakeflake_store::{PackageIndex, PackageInfo};
use serde_json::json;

use super::PackageModel;
use super::search::CommandState;
use crate::hub::{CommandLookupError, PackagesMsg};
use crate::models::PAGE_SIZE;

fn looked_up(result: Result<Vec<PackageAttr>, CommandLookupError>) -> PackageModel {
    let mut model = PackageModel {
        command_mode: true,
        query: "rg".to_owned(),
        ..PackageModel::default()
    };

    assert!(model.on_commands("rg", result));
    model
}

fn index(attrs: &[&str]) -> Arc<PackageIndex> {
    Arc::new(PackageIndex::from_entries(
        attrs
            .iter()
            .map(|attr| PackageInfo {
                attr: (*attr).into(),
                version: "1.0".to_owned(),
                description: format!("{attr}\n  tool"),
            })
            .collect(),
    ))
}

fn shown(model: &PackageModel) -> Vec<(String, bool)> {
    model
        .rows
        .iter()
        .map(|row| (row.attr.clone(), row.installed))
        .collect()
}

fn with_index() -> PackageModel {
    let mut model = PackageModel::default();
    assert!(model.handle(PackagesMsg::Index(index(&["bat", "fd", "git", "ripgrep"]))));
    assert!(
        !model.handle(PackagesMsg::Installed(HashSet::from([PackageAttr::from(
            "git"
        )])))
    );
    model.refresh();
    model
}

#[test]
fn command_states_for_qml() {
    let found = looked_up(Ok(vec![PackageAttr::from("ripgrep")]));
    assert_eq!(found.command_state, CommandState::Idle);
    assert_eq!(
        (found.command_state(), found.command_error()),
        (String::new(), String::new())
    );

    let missing = looked_up(Err(CommandLookupError::MissingIndex));
    assert_eq!(missing.command_state(), "missing-index");
    assert_eq!(missing.command_error(), "");

    let failed = looked_up(Err(CommandLookupError::Failed("boom".to_owned())));
    assert_eq!(failed.command_state(), "error");
    assert_eq!(failed.command_error(), "boom");
}

#[test]
fn stale_lookups_are_ignored() {
    let mut model = PackageModel {
        command_mode: true,
        query: "fd".to_owned(),
        ..PackageModel::default()
    };

    assert!(!model.on_commands("rg", Err(CommandLookupError::MissingIndex)));
    assert_eq!(model.command_state, CommandState::Idle);
}

#[test]
fn search_modes_pick_their_packages() {
    let mut model = with_index();
    assert_eq!(model.total(), 4_i32);
    assert_eq!(
        model.rows.first().map(|row| row.description.as_str()),
        Some("bat tool")
    );

    model.set_only_installed(true);
    assert_eq!(shown(&model), [("git".to_owned(), true)]);
    model.set_only_installed(false);

    model.set_command_mode(true);
    assert_eq!(
        (model.total(), model.command_state()),
        (0_i32, String::new())
    );
    model.command_attrs = Some(vec![
        PackageAttr::from("ripgrep"),
        PackageAttr::from("gone"),
    ]);
    model.refresh();
    assert_eq!(shown(&model), [("ripgrep".to_owned(), false)]);
}

#[test]
fn installed_flags_follow_the_installed_list() {
    let mut model = with_index();
    let research = model.handle(PackagesMsg::Installed(HashSet::from([PackageAttr::from(
        "fd",
    )])));

    assert!(!research);
    assert_eq!(
        shown(&model),
        [
            ("bat".to_owned(), false),
            ("fd".to_owned(), true),
            ("git".to_owned(), false),
            ("ripgrep".to_owned(), false)
        ]
    );

    model.only_installed = true;
    assert!(model.handle(PackagesMsg::Installed(HashSet::new())));
}

#[test]
fn package_sets_are_searched_once_indexed() {
    let mut model = with_index();
    model.handle(PackagesMsg::Sets(vec!["python3Packages".to_owned()]));
    model.set_package_set("nope".to_owned());
    assert_eq!(model.package_set(), "");

    model.set = "python3Packages".to_owned();
    assert!(model.set_loading());
    assert_eq!(model.active().len(), 0);

    let other = PackagesMsg::SetIndex {
        set: "perlPackages".to_owned(),
        result: Err("broken".to_owned()),
    };

    assert!(!model.handle(other));
    assert_eq!(model.set_error(), "");

    let failed = PackagesMsg::SetIndex {
        set: "python3Packages".to_owned(),
        result: Err("broken".to_owned()),
    };

    assert!(!model.handle(failed));
    assert!(!model.set_loading());
    assert_eq!(model.set_error(), "broken");

    let loaded = PackagesMsg::SetIndex {
        set: "python3Packages".to_owned(),
        result: Ok(index(&["numpy"])),
    };

    assert!(model.handle(loaded));
    assert!(model.is_known("numpy".to_owned()) && model.is_known("git".to_owned()));
    model.refresh();
    assert_eq!(shown(&model), [("numpy".to_owned(), false)]);
}

#[test]
fn details_follow_the_request() {
    let mut model = with_index();
    let state = |packages: &PackageModel| packages.details("git".to_owned())["state"].clone();
    assert_eq!(state(&model), "unknown");
    assert_eq!(model.details("git".to_owned())["installed"], true);
    assert_eq!(model.details("zzz".to_owned())["known"], false);

    model.pending.insert(PackageAttr::from("git"));
    assert_eq!(state(&model), "loading");

    let failed = PackagesMsg::Details {
        attr: PackageAttr::from("git"),
        result: Err("no meta".to_owned()),
    };

    assert!(!model.handle(failed));
    assert_eq!(state(&model), "error");
    assert_eq!(model.pending, HashSet::new());

    model.request_details("git".to_owned());
    assert_eq!(model.pending, HashSet::new());
}

#[test]
fn rows_arrive_one_page_at_a_time() {
    let names: Vec<String> = (0..PAGE_SIZE + 20).map(|n| format!("p{n:03}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut model = PackageModel::default();
    model.handle(PackagesMsg::Index(index(&names)));
    model.refresh();
    assert_eq!(model.rows.len(), PAGE_SIZE);
    assert!(model.has_more());

    model.load_more();
    assert_eq!(model.rows.len(), PAGE_SIZE + 20);
    assert!(!model.has_more());

    assert!(model.handle(PackagesMsg::Reset));
    model.refresh();
    assert_eq!((model.total(), model.size()), (0_i32, 0_i32));
}

#[test]
fn the_overview_lookup_ignores_failures() {
    let mut model = with_index();
    assert_eq!(
        model.top(" ".to_owned(), 3),
        json!({ "total": 0_i32, "rows": [] })
    );
    assert_eq!(model.top("rip".to_owned(), 3)["total"], 1_i32);

    model.find_overview_command("not a program".to_owned());
    assert_eq!(model.overview_command, "not a program");
    assert!(!model.on_commands("rg", Ok(vec![PackageAttr::from("ripgrep")])));
    assert_eq!(model.overview_commands, Vec::<PackageAttr>::new());

    model.overview_command = "rg".to_owned();
    model.on_commands("rg", Ok(vec![PackageAttr::from("ripgrep")]));
    assert_eq!(
        model.overview_commands(),
        json!({ "command": "rg", "attrs": ["ripgrep"] })
    );
    model.on_commands("rg", Err(CommandLookupError::MissingIndex));
    assert_eq!(model.overview_commands, Vec::<PackageAttr>::new());
}
