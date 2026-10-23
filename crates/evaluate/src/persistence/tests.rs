use super::*;

fn cache_with(text: Option<&str>) -> (tempfile::TempDir, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().join("host"));

    if let Some(text) = text {
        cache.write(PERSISTENCE_FILE, text);
    }

    (dir, cache)
}

#[test]
fn a_matching_fingerprint_reuses_the_entries() {
    let (_dir, cache) = cache_with(Some(
        r#"{"fingerprint":"abc","persistence":{"entries":[],"storages":["/persist"],"homes":{},"homeModule":{},"sources":[]}}"#,
    ));

    let reused = cached(&cache, "abc").expect("current fingerprint");
    assert_eq!(reused.persistence.unwrap().storages, ["/persist"]);
}

#[test]
fn a_host_without_the_module_is_cached_as_none() {
    let (_dir, cache) = cache_with(Some(r#"{"fingerprint":"abc","persistence":null}"#));

    let reused = cached(&cache, "abc").expect("current fingerprint");
    assert!(reused.persistence.is_none());
}

#[test]
fn a_changed_fingerprint_is_evaluated_again() {
    let (_dir, cache) = cache_with(Some(r#"{"fingerprint":"abc","persistence":null}"#));

    assert!(cached(&cache, "other").is_none());
}

#[test]
fn a_corrupt_file_is_evaluated_again() {
    let (_dir, cache) = cache_with(Some("{not json"));

    assert!(cached(&cache, "abc").is_none());
}

#[test]
fn a_missing_file_is_evaluated() {
    let (_dir, cache) = cache_with(None);

    assert!(cached(&cache, "abc").is_none());
}
