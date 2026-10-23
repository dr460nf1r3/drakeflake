mod scanned;

use std::collections::BTreeMap;

use drakeflake_file::{LIST_TYPE, list_input};
use drakeflake_flake::{PersistEntry, PersistKind, PersistScope, PersistSource, Persistence};
use drakeflake_plan::{ManagedOption, ManagedState};
use drakeflake_render::OptionName;
use serde_json::json;

use super::state::{persist_kind, push_absent};
use super::view::{self, RowState};
use super::{Load, Shown};

const SYSTEM_DIRS: &str = "environment.persistence.\"/persist\".directories";
const HOME_FILES: &str = "home-manager.users.alice.home.persistence.\"/persist\".files";

fn entry(
    scope: PersistScope,
    user: &str,
    kind: PersistKind,
    entry: &str,
    path: &str,
) -> PersistEntry {
    PersistEntry {
        kind,
        path: path.to_owned(),
        entry: entry.to_owned(),
        storage: "/persist".to_owned(),
        scope,
        user: user.to_owned(),
    }
}

fn data() -> Persistence {
    Persistence {
        entries: vec![
            entry(
                PersistScope::System,
                "",
                PersistKind::Dir,
                "/var/log",
                "/var/log",
            ),
            entry(
                PersistScope::System,
                "",
                PersistKind::Dir,
                "/var/lib/foo",
                "/var/lib/foo",
            ),
            entry(
                PersistScope::Home,
                "alice",
                PersistKind::File,
                ".bashrc",
                "/home/alice/.bashrc",
            ),
        ],
        storages: vec!["/persist".to_owned()],
        homes: BTreeMap::from([("alice".to_owned(), "/home/alice".to_owned())]),
        home_module: BTreeMap::from([("alice".to_owned(), true)]),
        sources: vec![PersistSource {
            scope: PersistScope::System,
            user: String::new(),
            storage: "/persist".to_owned(),
            entry: "/var/log".to_owned(),
            file: "/nix/store/abc-source/hosts/base.nix".to_owned(),
        }],
    }
}

fn state(lists: &[(&str, &[&str])]) -> ManagedState {
    let mut state = ManagedState::default();

    for (option, entries) in lists {
        let entries: Vec<String> = entries.iter().map(|&text| text.to_owned()).collect();
        state.options.insert(
            OptionName::from(*option),
            ManagedOption::typed(LIST_TYPE, &list_input(&entries)),
        );
    }

    state
}

#[test]
fn rows_tell_generated_entries_from_locked_and_staged_ones() {
    let saved = state(&[(SYSTEM_DIRS, &["/var/lib/foo"])]);
    let plan = state(&[
        (SYSTEM_DIRS, &["/srv"]),
        (HOME_FILES, &[".bashrc", ".profile"]),
    ]);

    let rows = view::persisted_rows(&data(), &saved, &plan);

    let row = |path: &str| rows.iter().find(|row| row.path == path).unwrap();

    let log = row("/var/log");
    assert!(log.locked);
    assert_eq!(log.source, "hosts/base.nix");
    assert_eq!(log.state, RowState::Kept);

    let foo = row("/var/lib/foo");
    assert!(!foo.locked);
    assert_eq!(foo.state, RowState::Remove);

    let srv = row("/srv");
    assert_eq!((srv.state, srv.kind), (RowState::Add, PersistKind::Dir));
    assert_eq!(srv.option, SYSTEM_DIRS);

    let profile = row("/home/alice/.profile");
    assert_eq!(
        (profile.state, profile.entry.as_str(), profile.scope),
        (RowState::Add, ".profile", PersistScope::Home)
    );

    assert!(row("/home/alice/.bashrc").locked);
    assert_eq!(rows.len(), 5);
}

#[test]
fn rows_reach_qml_with_the_strings_it_compares() {
    let saved = state(&[(SYSTEM_DIRS, &["/var/lib/foo"])]);
    let plan = state(&[(HOME_FILES, &[".profile"])]);
    let rows = json!(view::persisted_rows(&data(), &saved, &plan));

    let field = |path: &str, name: &str| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == path)
            .unwrap()[name]
            .clone()
    };

    assert_eq!(field("/var/log", "state"), "");
    assert_eq!(field("/var/log", "kind"), "dir");
    assert_eq!(field("/var/log", "scope"), "system");
    assert_eq!(field("/var/lib/foo", "state"), "remove");
    assert_eq!(field("/home/alice/.profile", "state"), "add");
    assert_eq!(field("/home/alice/.profile", "kind"), "file");
    assert_eq!(field("/home/alice/.profile", "scope"), "home");
    assert_eq!(json!(PersistScope::User), "user");
}

#[test]
fn slot_strings_are_parsed_into_kinds_and_scopes() {
    assert_eq!(persist_kind("dir"), Ok(PersistKind::Dir));
    assert_eq!(persist_kind("file"), Ok(PersistKind::File));
    assert_eq!(persist_kind("link"), Ok(PersistKind::File));
    assert_eq!(persist_kind("other"), Ok(PersistKind::File));
    persist_kind("folder").unwrap_err();
    persist_kind("").unwrap_err();

    assert_eq!(Shown::parse("home"), Shown::Home);
    assert_eq!(Shown::parse("system"), Shown::System);
    assert_eq!(Shown::parse("else"), Shown::System);
    assert_eq!(
        (Shown::Home.to_string(), Shown::System.to_string()),
        ("home".to_owned(), "system".to_owned())
    );
}

#[test]
fn a_reload_keeps_the_page_loaded() {
    let first = Load::Idle.started();
    assert_eq!((first.is_loading(), first.is_loaded()), (true, false));

    let again = Load::Done.started();
    assert_eq!((again.is_loading(), again.is_loaded()), (true, true));
    assert_eq!(
        (Load::Done.is_loading(), Load::Done.is_loaded()),
        (false, true)
    );
}

#[test]
fn an_entry_is_listed_once() {
    let mut entries = vec!["/a".to_owned()];

    push_absent(&mut entries, "/a".to_owned());
    push_absent(&mut entries, "/b".to_owned());

    assert_eq!(entries, ["/a", "/b"]);
}
