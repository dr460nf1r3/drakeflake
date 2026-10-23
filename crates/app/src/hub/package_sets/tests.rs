use drakeflake_flake::Target;
use drakeflake_fs::fnv1a;

use super::*;
use crate::hub::Generation;
use crate::hub::load::Loader;

fn temp_cache() -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    (dir, cache)
}

fn posted_sets(hub: &Hub) -> Vec<Vec<String>> {
    hub.mail
        .packages
        .take()
        .into_iter()
        .filter_map(|message| match message {
            PackagesMsg::Sets(sets) => Some(sets),
            PackagesMsg::Details { attr: _, result: _ }
            | PackagesMsg::Reset
            | PackagesMsg::Index(_)
            | PackagesMsg::IndexError(_)
            | PackagesMsg::Installed(_)
            | PackagesMsg::Commands {
                command: _,
                result: _,
            }
            | PackagesMsg::CommandIndexReady
            | PackagesMsg::SetIndex { set: _, result: _ } => None,
        })
        .collect()
}

#[test]
fn stale_set_caches_are_removed_and_current_ones_kept() {
    let (dir, cache) = temp_cache();
    for name in [
        "package-sets-00000000000000aa.json",
        "package-sets-00000000000000bb.json",
        "set-1111111111111111-00000000000000aa.json",
        "set-1111111111111111-00000000000000bb.json",
        "packages.json",
    ] {
        cache.write(name, "[]");
    }

    remove_stale_sets(&cache, "00000000000000aa");

    let mut left: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    left.sort();
    assert_eq!(
        left,
        vec![
            "package-sets-00000000000000aa.json",
            "packages.json",
            "set-1111111111111111-00000000000000aa.json",
        ]
    );
}

#[test]
fn cached_sets_of_this_flake_version_are_posted() {
    let (dir, cache) = temp_cache();
    let version = format!("{:016x}", fnv1a(b"fingerprint"));
    cache.write(
        &format!("package-sets-{version}.json"),
        r#"["haskellPackages","python313Packages"]"#,
    );
    cache.write("package-sets-0000000000000000.json", "[]");
    let hub = Hub::default();

    Loader::new(
        hub.services(),
        &hub.nix(),
        Generation::default(),
        &Target::default(),
    )
    .load_package_sets(&cache, Some("fingerprint"));

    assert_eq!(
        posted_sets(&hub),
        vec![vec!["haskellPackages", "python313Packages"]]
    );
    assert_eq!(hub.session().package_version(), Some(version));
    assert!(
        !dir.path()
            .join("package-sets-0000000000000000.json")
            .exists(),
        "the sets of an older lock are removed"
    );
}

#[test]
fn sets_of_a_superseded_load_are_dropped() {
    let (_dir, cache) = temp_cache();
    let version = format!("{:016x}", fnv1a(b"fingerprint"));
    cache.write(&format!("package-sets-{version}.json"), "[]");
    let hub = Hub::default();
    hub.session()
        .update(|state| state.generation = Generation(2));

    Loader::new(
        hub.services(),
        &hub.nix(),
        Generation(1),
        &Target::default(),
    )
    .load_package_sets(&cache, Some("fingerprint"));

    assert_eq!(posted_sets(&hub), Vec::<Vec<String>>::new());
}

#[test]
fn only_other_versions_are_stale() {
    let current = "00000000000000aa";

    assert!(is_stale_set_file(
        "package-sets-00000000000000bb.json",
        current
    ));
    assert!(is_stale_set_file(
        "set-1111111111111111-00000000000000bb.json",
        current
    ));
    assert!(!is_stale_set_file(
        "package-sets-00000000000000aa.json",
        current
    ));
    assert!(!is_stale_set_file(
        "set-1111111111111111-00000000000000aa.json",
        current
    ));
    assert!(!is_stale_set_file("packages.json", current));
    assert!(!is_stale_set_file("options-skip.json", current));
}

#[test]
fn without_a_fingerprint_nothing_is_cached_or_read() {
    let (dir, cache) = temp_cache();
    cache.write(
        "package-sets-0000000000000000.json",
        r#"["fromTheEmptyHash"]"#,
    );
    let hub = Hub::default();

    Loader::new(
        hub.services(),
        &hub.nix(),
        Generation::default(),
        &Target::default(),
    )
    .load_package_sets(&cache, None);

    assert_eq!(hub.session().package_version(), None);
    assert!(
        posted_sets(&hub)
            .iter()
            .all(|sets| !sets.contains(&"fromTheEmptyHash".to_owned()))
    );
    assert!(
        dir.path()
            .join("package-sets-0000000000000000.json")
            .exists()
    );
}
