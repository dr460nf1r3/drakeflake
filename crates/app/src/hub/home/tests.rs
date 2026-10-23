use std::fs;

use super::super::test_support::local_target;
use super::*;
use crate::hub::Generation;
use crate::hub::load::Loader;

#[test]
fn the_shown_user_stays_while_the_host_still_has_it() {
    let users = vec![UserName::from("alice"), UserName::from("bob")];
    let some = |name: &str| Some(UserName::from(name));
    let cases = [
        (some("bob"), some("alice"), "bob"),
        (some("gone"), some("alice"), "alice"),
        (some("gone"), some("carol"), "alice"),
        (None, some("bob"), "bob"),
        (None, None, "alice"),
    ];

    for (previous, login, expected) in cases {
        assert_eq!(
            pick_home_user(&users, previous.clone(), login.clone()),
            expected,
            "{previous:?} {login:?}"
        );
    }

    assert_eq!(pick_home_user(&[], some("bob"), some("bob")), "");
}

#[test]
fn a_superseded_user_posts_nothing() {
    let hub = Hub::default();
    hub.session().select_home_user(Some(UserName::from("bob")));

    let nix = hub.nix();
    let target = Target::default();
    let loader = Loader::new(hub.services(), &nix, Generation::default(), &target);

    assert!(!loader.post_home_for("alice", vec![OptionsMsg::Reset]));
    assert!(loader.post_home_for("bob", vec![OptionsMsg::Reset]));
    assert_eq!(hub.mail.home.take().len(), 1);
}

#[test]
fn every_user_has_cache_files_of_their_own() {
    let alice = HomeFiles::of(&UserName::from("alice"));
    let bob = HomeFiles::of(&UserName::from("bob"));
    let odd = HomeFiles::of(&UserName::from("../../etc"));

    assert_ne!(alice.docs, bob.docs);
    assert_ne!(alice.values, bob.values);
    let names = [
        &odd.docs,
        &odd.values,
        &odd.docs_key,
        &odd.docs_skip,
        &odd.values_skip,
    ];

    assert!(
        names.iter().all(|name| !name.contains('/')),
        "a user name escapes the cache folder: {names:?}"
    );
    assert_eq!(odd.user, "../../etc");
}

#[test]
fn reused_home_data_comes_from_the_cache_alone() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    let files = HomeFiles::of(&UserName::from("alice"));
    fs::write(dir.path().join(&files.docs), "{\"docs\":1}").unwrap();
    fs::write(dir.path().join(&files.values), "{\"values\":1}").unwrap();

    let hub = Hub::default();
    let (docs, values) = Loader::new(
        hub.services(),
        &hub.nix(),
        Generation::default(),
        &local_target(dir.path()),
    )
    .home_data(&cache, &files, HomeData::Cached);

    assert_eq!(docs.unwrap(), "{\"docs\":1}");
    assert_eq!(values.unwrap(), "{\"values\":1}");
}
