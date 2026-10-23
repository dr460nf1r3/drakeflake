use super::*;
use crate::hub::Generation;

fn state(options: &[(&str, &str)], packages: &[&str]) -> ManagedState {
    let options: Map<String, Value> = options
        .iter()
        .map(|(name, input)| {
            (
                (*name).to_owned(),
                serde_json::json!({ "type": "boolean", "input": input }),
            )
        })
        .collect();

    serde_json::from_value(serde_json::json!({ "options": options, "packages": packages })).unwrap()
}

fn names(list: &[&str]) -> BTreeSet<OptionName> {
    list.iter().map(|name| OptionName::from(*name)).collect()
}

#[test]
fn only_changed_options_are_re_evaluated() {
    let before = state(
        &[("a.keep", "true"), ("a.change", "true"), ("a.drop", "1")],
        &["git"],
    );

    let after = state(
        &[("a.keep", "true"), ("a.change", "false"), ("a.new", "2")],
        &["git"],
    );

    assert_eq!(
        changed_options(&before, &after, false),
        names(&["a.change", "a.drop", "a.new"])
    );
    assert_eq!(changed_options(&after, &after, false), BTreeSet::default());
}

#[test]
fn package_and_layout_changes() {
    let before = state(&[("a.keep", "true")], &["git"]);
    let after = state(&[("a.keep", "true")], &["git", "vim"]);

    assert_eq!(
        changed_options(&before, &after, false),
        names(&[PACKAGES_PATH])
    );

    assert_eq!(
        changed_options(&before, &before, true),
        names(&["a.keep", PACKAGES_PATH])
    );
    assert_eq!(
        changed_options(&state(&[], &[]), &state(&[], &[]), true),
        BTreeSet::default()
    );
}

#[test]
fn home_manager_names_are_grouped_by_user() {
    let (system, homes) = split_names(names(&[
        "home-manager.users.alice.programs.git.enable",
        "home-manager.users.\"b.o\".home.stateVersion",
        "services.openssh.enable",
    ]));

    assert_eq!(
        system,
        vec![
            OptionName::from("services.openssh.enable"),
            OptionName::from(HOME_USERS)
        ]
    );
    assert_eq!(
        homes.get("alice").cloned(),
        Some(vec![OptionName::from(
            "home-manager.users.alice.programs.git.enable"
        )])
    );
    assert!(homes.contains_key("b.o"));

    let (only_system, no_homes) = split_names(names(&["networking.hostName"]));
    assert_eq!(only_system, vec![OptionName::from("networking.hostName")]);
    assert_eq!(no_homes, BTreeMap::default());
}

#[test]
fn fresh_entries_replace_cached_ones() {
    let cached = r#"{"a": {"u": ["/x.nix"], "v": 1}, "b": {"u": []}}"#;
    let fresh = r#"{"a": {"u": []}, "c": {"u": ["/y.nix"], "v": true}}"#;
    let requested = [OptionName::from("a"), OptionName::from("c")];
    let merged: Value =
        serde_json::from_str(&merge_snapshot_text(cached, fresh, &requested).unwrap()).unwrap();

    assert_eq!(merged["a"], serde_json::json!({ "u": [] }));
    assert_eq!(merged["b"], serde_json::json!({ "u": [] }));
    assert_eq!(merged["c"]["v"], true);
    merge_snapshot_text("[]", fresh, &requested).unwrap_err();
    merge_snapshot_text(cached, "null", &requested).unwrap_err();
}

#[test]
fn removed_options_are_dropped_from_the_snapshot() {
    let cached = r#"{"a": {"u": []}, "gone": {"u": []}}"#;
    let fresh = r#"{"a": {"u": []}}"#;
    let requested = [OptionName::from("a"), OptionName::from("gone")];
    let merged: Value =
        serde_json::from_str(&merge_snapshot_text(cached, fresh, &requested).unwrap()).unwrap();

    assert!(merged.get("gone").is_none());
    assert!(merged.get("a").is_some());
}

fn temp_cache() -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    (dir, cache)
}

#[test]
fn merged_snapshots_are_written_back_and_failures_leave_the_cache_alone() {
    let (_dir, cache) = temp_cache();

    assert_eq!(
        merge_into(&cache, VALUES_FILE, "{}", &[]).err(),
        Some(format!("{VALUES_FILE} is not cached"))
    );

    cache.write(VALUES_FILE, r#"{"a": {"u": []}}"#);
    merge_into(&cache, VALUES_FILE, "[1]", &[]).unwrap_err();
    assert_eq!(
        cache.read(VALUES_FILE).as_deref(),
        Some(r#"{"a": {"u": []}}"#),
        "a failed merge keeps the cached values"
    );

    merge_into(&cache, VALUES_FILE, r#"{"b": {"u": []}}"#, &[]).unwrap();
    let written: Value = serde_json::from_str(&cache.read(VALUES_FILE).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "a": { "u": [] }, "b": { "u": [] } })
    );
}

#[test]
fn home_manager_users_is_listed_once() {
    let (system, homes) = split_names(names(&[
        HOME_USERS,
        "home-manager.users.alice.home.stateVersion",
    ]));

    assert_eq!(system, vec![OptionName::from(HOME_USERS)]);
    assert_eq!(homes.len(), 1);
}

#[test]
fn unselected_users_cached_home_values_are_dropped() {
    let (_dir, cache) = temp_cache();
    let alice = HomeFiles::of(&UserName::from("alice"));
    let bob = HomeFiles::of(&UserName::from("bob"));
    cache.write(&alice.values, "{}");
    cache.write(&bob.values, "{}");
    let hub = Hub::default();

    let homes: BTreeMap<UserName, Vec<OptionName>> = [
        (
            UserName::from("alice"),
            vec![OptionName::from("home-manager.users.alice.a")],
        ),
        (
            UserName::from("bob"),
            vec![OptionName::from("home-manager.users.bob.b")],
        ),
    ]
    .into();

    assert!(
        Loader::new(
            hub.services(),
            &hub.nix(),
            Generation::default(),
            &Target::default()
        )
        .refresh_home_selected(&cache, &homes)
    );
    assert_eq!(cache.read(&alice.values), None);
    assert_eq!(cache.read(&bob.values), None);
}
