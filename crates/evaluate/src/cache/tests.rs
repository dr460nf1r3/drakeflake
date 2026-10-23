use super::*;

fn temp_cache() -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().join("host"));

    (dir, cache)
}

fn target(flake: &str, host: &str) -> Target {
    Target {
        flake: flake.to_owned(),
        host: host.into(),
        ..Target::default()
    }
}

#[test]
fn cache_folders_follow_the_trimmed_flake_and_the_host() {
    let laptop = Cache::for_target(&target("/etc/nixos", "laptop")).dir;

    assert_eq!(
        Cache::for_target(&target("  /etc/nixos\n", "laptop")).dir,
        laptop
    );
    assert_ne!(
        Cache::for_target(&target("/etc/nixos", "desktop")).dir,
        laptop
    );
    assert_ne!(
        Cache::for_target(&target("/srv/nixos", "laptop")).dir,
        laptop
    );
    assert_ne!(Cache::for_flake("/etc/nixos").dir, laptop);
    assert_eq!(
        Cache::for_flake(" /etc/nixos ").dir,
        Cache::for_flake("/etc/nixos").dir
    );
}

#[test]
fn cache_is_current_only_when_complete_for_the_same_fingerprint() {
    let (_dir, cache) = temp_cache();
    assert!(!cache.is_current(Some("abc")), "nothing is cached");

    for name in [OPTIONS_FILE, VALUES_FILE, PACKAGES_FILE] {
        cache.write(name, "{}");
    }

    cache.save_fingerprint(Some("abc"));
    assert!(!cache.is_current(Some("abc")), "installed packages missing");

    cache.write(INSTALLED_FILE, "[]");
    assert!(cache.is_current(Some("abc")));
    assert!(!cache.is_current(Some("def")), "the flake changed");
    assert!(!cache.is_current(None), "an unknown flake is never current");

    cache.save_fingerprint(None);
    assert_eq!(cache.read(FINGERPRINT_FILE), None);
    assert!(!cache.is_current(None), "no fingerprint on either side");
}

#[test]
fn corrupt_or_missing_lists_read_as_empty() {
    let (_dir, cache) = temp_cache();
    let items = vec!["a".to_owned(), "b".to_owned()];

    cache.write_list("list.json", &items);
    assert_eq!(cache.read_list("list.json"), items);

    cache.write("list.json", "{ not json");
    assert_eq!(cache.read_list("list.json"), Vec::<String>::new());
    assert_eq!(cache.read_list("missing.json"), Vec::<String>::new());
    assert_eq!(
        cache.read_parsed("list.json", str::parse::<u32>),
        None,
        "a parse error is not a value"
    );
}

#[test]
fn removed_and_cleared_files_are_gone_and_missing_ones_are_fine() {
    let (_dir, cache) = temp_cache();
    cache.write("a", "1");
    cache.write("b", "2");

    cache.remove("a");
    cache.remove("a");
    assert_eq!(cache.read("a"), None);
    assert_eq!(cache.read("b").as_deref(), Some("2"));

    cache.clear();
    assert!(!cache.dir().unwrap().exists());
    cache.clear();
}

#[test]
fn unusable_cache_folders_read_nothing() {
    let (dir, _) = temp_cache();
    let blocked = dir.path().join("file");
    fs::write(&blocked, "not a folder").unwrap();
    let blocked_cache = Cache::at(blocked);

    blocked_cache.write("a", "1");
    assert_eq!(blocked_cache.read("a"), None);

    let (_other, cache) = temp_cache();
    fs::create_dir_all(cache.dir().unwrap().join("folder")).unwrap();
    assert_eq!(cache.read("folder"), None, "a folder is not a cached file");
}

#[test]
fn a_cache_without_a_folder_stores_and_finds_nothing() {
    let cache = Cache { dir: None };

    cache.write("a", "1");
    cache.save_fingerprint(Some("fp"));
    cache.remove("a");
    cache.clear();

    assert_eq!(cache.read("a"), None);
    assert!(!cache.is_current(Some("fp")));
    assert!(cache.dir().is_none());
}
