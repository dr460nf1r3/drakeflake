use std::fs;
use std::path::{Path, PathBuf};

use super::*;

fn kirigami(root: &Path) -> PathBuf {
    let dir = root.join("org/kde/kirigami");
    fs::create_dir_all(dir.join("styles/org.kde.desktop")).unwrap();
    fs::write(dir.join("qmldir"), "module org.kde.kirigami\n").unwrap();
    fs::write(dir.join("Kirigami.qml"), "").unwrap();
    fs::write(dir.join("styles/org.kde.desktop/Theme.qml"), "").unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    names.sort();
    names
}

#[test]
fn the_mirror_links_kirigami_and_adds_our_theme() {
    let store = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let original = kirigami(store.path());

    let mirror = kirigami_mirror(&original, cache.path()).unwrap();
    let root = mirror.join("org/kde/kirigami");

    assert_eq!(
        fs::read_link(root.join("qmldir")).unwrap(),
        original.join("qmldir")
    );
    assert_eq!(
        fs::read_link(root.join("styles/org.kde.desktop")).unwrap(),
        original.join("styles/org.kde.desktop")
    );
    assert_eq!(
        fs::read_to_string(root.join("styles").join(CONTROLS_STYLE).join("Theme.qml")).unwrap(),
        KIRIGAMI_THEME
    );
    assert_eq!(
        names(cache.path()),
        [mirror.file_name().unwrap().to_string_lossy().into_owned()],
        "no half-built folder is left behind"
    );
}

#[test]
fn a_finished_mirror_is_reused_as_it_is() {
    let store = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let original = kirigami(store.path());
    let first = kirigami_mirror(&original, cache.path()).unwrap();
    fs::write(first.join("marker"), "").unwrap();

    let second = kirigami_mirror(&original, cache.path()).unwrap();

    assert_eq!(second, first);
    assert!(second.join("marker").is_file(), "the mirror was rebuilt");
}

#[test]
fn another_kirigami_gets_its_own_mirror_and_broken_ones_go() {
    let store = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let old = kirigami_mirror(&kirigami(&store.path().join("old")), cache.path()).unwrap();
    fs::create_dir_all(cache.path().join("collected")).unwrap();
    fs::create_dir_all(cache.path().join(".building-1")).unwrap();

    let new = kirigami_mirror(&kirigami(&store.path().join("new")), cache.path()).unwrap();

    assert_ne!(new, old);
    assert!(
        old.join("org/kde/kirigami/qmldir").exists(),
        "a running app may use it"
    );
    assert!(
        !cache.path().join("collected").exists(),
        "a mirror whose Kirigami is gone is removed"
    );
    assert!(
        cache.path().join(".building-1").is_dir(),
        "another instance is still building there"
    );
}

#[test]
fn an_unfinished_build_of_this_process_is_started_over() {
    let store = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let leftover = cache
        .path()
        .join(format!(".building-{}", std::process::id()));

    fs::create_dir_all(leftover.join("org/kde/kirigami")).unwrap();
    fs::write(leftover.join("org/kde/kirigami/junk"), "").unwrap();

    let mirror = kirigami_mirror(&kirigami(store.path()), cache.path()).unwrap();

    assert!(!mirror.join("org/kde/kirigami/junk").exists());
    assert!(!leftover.exists());
}

#[test]
fn a_missing_kirigami_is_an_error_and_leaves_no_mirror() {
    let cache = tempfile::tempdir().unwrap();

    kirigami_mirror(Path::new("/nonexistent/org/kde/kirigami"), cache.path()).unwrap_err();

    assert_eq!(names(cache.path()), Vec::<String>::new());
}

#[cfg(target_os = "macos")]
#[test]
fn plain_text_has_a_default_app() {
    let (app, bundle) = default_text_app().unwrap();

    assert!(
        app.extension().is_some_and(|ext| ext == "app"),
        "{}",
        app.display()
    );
    assert_ne!(bundle, String::new());
}

#[cfg(target_os = "macos")]
#[test]
fn symbols_render_at_retina_size() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("go-next.png");

    assert!(render_symbol("chevron.forward", 16, 2, true, &file));

    let png = fs::read(&file).unwrap();

    assert_eq!(png.get(..8), Some(&b"\x89PNG\r\n\x1a\n"[..]));

    assert_eq!(png.get(16..24), Some(&[0, 0, 0, 32, 0, 0, 0, 32][..]));
}

#[cfg(target_os = "macos")]
#[test]
fn unknown_symbols_are_reported() {
    let dir = tempfile::tempdir().unwrap();

    assert!(!render_symbol(
        "no.such.symbol.x",
        16,
        1,
        false,
        &dir.path().join("x.png")
    ));
}

#[test]
fn a_changed_style_drops_the_qml_cache_once() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("state/style-resources");
    let cache = dir.path().join("qmlcache");
    let fill = || {
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("ItemDelegate.qmlc"), "").unwrap();
    };

    fill();
    drop_stale_qml_cache(&marker, &cache, b"style 1");

    assert!(
        !cache.exists(),
        "first start keeps a cache of unknown origin"
    );

    fill();
    drop_stale_qml_cache(&marker, &cache, b"style 1");

    assert!(cache.exists(), "same style dropped the cache");

    drop_stale_qml_cache(&marker, &cache, b"style 2");

    assert!(!cache.exists(), "changed style kept the cache");
}
