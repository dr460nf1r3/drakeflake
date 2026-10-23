use std::collections::BTreeMap;

use super::{PersistPlace, flake_relative};
use crate::{PersistEntry, PersistKind, PersistScope, PersistSource, Persistence};

const SYSTEM_DIRS: &str = "environment.persistence.\"/persist\".directories";
const HOME_FILES: &str = "home-manager.users.alice.home.persistence.\"/persist\".files";

fn data() -> Persistence {
    Persistence {
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
        ..Persistence::default()
    }
}

fn place(scope: PersistScope, user: &str, storage: &str, kind: PersistKind) -> PersistPlace {
    PersistPlace {
        scope,
        user: user.to_owned(),
        storage: storage.to_owned(),
        kind,
    }
}

#[test]
fn option_names_quote_the_storage_path() {
    let option = |scope, user: &str, kind| place(scope, user, "/persist", kind).option();

    assert_eq!(
        option(PersistScope::System, "", PersistKind::Dir),
        SYSTEM_DIRS
    );
    assert_eq!(
        option(PersistScope::User, "bob", PersistKind::File),
        "environment.persistence.\"/persist\".users.bob.files"
    );
    assert_eq!(
        option(PersistScope::Home, "alice", PersistKind::File),
        HOME_FILES
    );
    assert_eq!(
        drakeflake_render::split_option_name(SYSTEM_DIRS),
        ["environment", "persistence", "/persist", "directories"]
    );
}

#[test]
fn quoted_names_escape_what_would_end_or_interpolate_the_string() {
    let storage = "/mnt/a\"b\\c${d}.$e";
    let option = place(PersistScope::User, "o'neil", storage, PersistKind::Dir).option();

    assert_eq!(
        option,
        "environment.persistence.\"/mnt/a\\\"b\\\\c\\${d}.\\$e\".users.o'neil.directories"
    );
    assert_eq!(
        drakeflake_render::split_option_name(&option),
        [
            "environment",
            "persistence",
            storage,
            "users",
            "o'neil",
            "directories"
        ]
    );
}

#[test]
fn store_paths_are_shown_relative_to_the_flake() {
    assert_eq!(flake_relative("/nix/store/abc-source/a/b.nix"), "a/b.nix");
    assert_eq!(flake_relative("/etc/nixos/a.nix"), "/etc/nixos/a.nix");

    let entry = PersistEntry {
        kind: PersistKind::Dir,
        path: "/var/log".to_owned(),
        entry: "/var/log".to_owned(),
        storage: "/persist".to_owned(),
        scope: PersistScope::System,
        user: String::new(),
    };

    assert_eq!(data().source_of(&entry), "hosts/base.nix");
}

#[test]
fn entries_resolve_to_absolute_paths_and_every_place_is_offered() {
    let data = data();
    let home = place(PersistScope::Home, "alice", "/persist", PersistKind::File);
    let unknown = place(PersistScope::User, "bob", "/persist", PersistKind::File);

    assert_eq!(data.absolute(&home, ".profile"), "/home/alice/.profile");
    assert_eq!(data.absolute(&unknown, "/x"), "/home/bob/x");
    assert_eq!(
        data.absolute(
            &place(PersistScope::System, "", "/persist", PersistKind::Dir),
            "/srv"
        ),
        "/srv"
    );
    assert_eq!(data.places().len(), 2 * 3);
}

#[test]
fn new_entries_go_to_the_system_or_the_home_list() {
    let data = data();
    let home = Some(("alice", "/home/alice"));

    let (place, entry) = data
        .new_place(None, "/persist", "/var/lib/bar", PersistKind::Dir)
        .unwrap();

    assert_eq!(
        (place.option(), entry.as_str()),
        (SYSTEM_DIRS.to_owned(), "/var/lib/bar")
    );

    let (home_place, home_entry) = data
        .new_place(home, "/persist", "/home/alice/.profile", PersistKind::File)
        .unwrap();

    assert_eq!(
        (home_place.option(), home_entry.as_str()),
        (HOME_FILES.to_owned(), ".profile")
    );

    let refused = |user_home, storage, path| {
        data.new_place(user_home, storage, path, PersistKind::File)
            .unwrap_err()
    };

    refused(home, "/persist", "/home/alicex/a");
    refused(home, "/persist", "/home/alice");
    refused(None, "/persist", "/a\nb");
    refused(None, "/persist", "/a\tb");
    refused(None, "/persist", "/a\u{7f}b");
    refused(None, "/persist", "relative");
    refused(None, "", "/var/lib/bar");
}
