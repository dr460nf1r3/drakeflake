use rustix::fs::inotify::{self, CreateFlags};

use super::Tree;

#[test]
fn the_watch_covers_the_flake_folders_but_not_hidden_or_build_ones() {
    let root = tempfile::tempdir().unwrap();
    for folder in ["modules/sub", ".git/objects", "target", "result", ".direnv"] {
        std::fs::create_dir_all(root.path().join(folder)).unwrap();
    }

    let fd = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap();
    let mut tree = Tree::default();
    tree.add(&fd, root.path()).unwrap();

    let mut watched: Vec<String> = tree
        .folders
        .values()
        .map(|path| {
            path.strip_prefix(root.path())
                .unwrap()
                .display()
                .to_string()
        })
        .collect();

    watched.sort();

    assert_eq!(watched, ["", "modules", "modules/sub"]);
}

#[test]
fn a_folder_that_vanished_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    let fd = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap();
    let mut tree = Tree::default();

    tree.add(&fd, &root.path().join("gone")).unwrap();

    assert!(tree.folders.is_empty());
}
