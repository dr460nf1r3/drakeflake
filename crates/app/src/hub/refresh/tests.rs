use std::path::Path;

use super::super::Hub;
use super::super::test_support::local_target;
use super::*;
use crate::hub::Generation;

#[test]
fn skipped_docs_are_told_apart_from_skipped_options() {
    let skip = |names: &[&str]| {
        names
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>()
    };

    assert_eq!(skipped_docs_warnings(&[]), Vec::<String>::new());

    let only_docs = skipped_docs_warnings(&skip(&["~a.b", "~c"]));
    assert_eq!(only_docs.len(), 1);
    assert!(
        only_docs
            .iter()
            .all(|line| line.starts_with("2 options have documentation"))
    );

    assert_eq!(
        skipped_docs_warnings(&skip(&["~a.b", "x.y", "z"])).last(),
        Some(&"Skipped options that fail to evaluate: x.y, z".to_owned())
    );
}

#[test]
fn a_reusable_option_index_is_kept_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().join("cache"));

    let hub = Hub::default();
    let index = IndexKey {
        key: Some("key"),
        reusable: true,
    };

    assert!(
        Loader::new(
            hub.services(),
            &hub.nix(),
            Generation(1),
            &local_target(dir.path())
        )
        .refresh_options_index(&cache, index)
    );
    assert_eq!(
        hub.mail.options.take().len(),
        0,
        "the shown index was replaced"
    );
    assert!(
        !cache.dir().is_some_and(Path::exists),
        "the cache was rewritten"
    );
}
