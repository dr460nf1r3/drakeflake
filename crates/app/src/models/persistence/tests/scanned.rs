use std::collections::BTreeSet;

use drakeflake_flake::PersistKind;
use drakeflake_scan::{EntryKind, ScanNode};
use serde_json::json;

use super::super::tree::{self, Kept};

fn dir(name: &str, children: Vec<ScanNode>) -> ScanNode {
    ScanNode {
        name: name.to_owned(),
        kind: EntryKind::Dir,
        size: children.iter().map(|child| child.size).sum(),
        files: children.iter().map(|child| child.files).sum(),
        children,
        hidden: 0,
        denied: false,
    }
}

fn file(name: &str, size: u64) -> ScanNode {
    ScanNode {
        name: name.to_owned(),
        kind: EntryKind::File,
        size,
        files: 1,
        children: Vec::new(),
        hidden: 0,
        denied: false,
    }
}

fn scanned() -> ScanNode {
    dir(
        "/",
        vec![
            dir("etc", vec![file("machine-id", 33), file("hosts", 10)]),
            dir(
                "var",
                vec![dir(
                    "lib",
                    vec![dir("foo", vec![file("db", 100)]), file("x", 1)],
                )],
            ),
        ],
    )
}

#[test]
fn kept_paths_leave_the_tree_and_its_sizes() {
    let kept = Kept::new([
        ("/etc/machine-id", PersistKind::File),
        ("/var/lib/foo", PersistKind::Dir),
        ("/var/lib/f", PersistKind::Dir),
    ]);

    let root = tree::without_kept(&scanned(), "/", &kept).unwrap();

    assert_eq!((root.size, root.files), (11, 2));

    let closed = tree::rows(&root, "/", &BTreeSet::new());
    let top: Vec<&str> = closed.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(top, ["/etc", "/var"]);

    let open = BTreeSet::from(["/var".to_owned(), "/var/lib".to_owned()]);
    let rows = tree::rows(&root, "/", &open);
    let shown: Vec<(&str, usize, bool)> = rows
        .iter()
        .map(|row| (row.path.as_str(), row.depth, row.expandable))
        .collect();

    assert_eq!(
        shown,
        [
            ("/etc", 0, true),
            ("/var", 0, true),
            ("/var/lib", 1, true),
            ("/var/lib/x", 2, false)
        ]
    );
    assert_eq!(json!(rows)[3]["kind"], "file");
}

#[test]
fn a_kept_folder_keeps_what_is_below_and_a_file_only_itself() {
    let kept = Kept::new([("/a", PersistKind::File), ("/b", PersistKind::Dir)]);

    assert!(kept.contains("/a"));
    assert!(!kept.contains("/a/x"));
    assert!(kept.contains("/b"));
    assert!(kept.contains("/b/x/y"));
    assert!(!kept.contains("/bc"));
    assert!(!kept.contains("/"));
}

#[test]
fn folders_left_empty_by_kept_entries_disappear() {
    let root = dir(
        "/",
        vec![
            dir(
                "var",
                vec![dir("lib", vec![dir("foo", vec![file("db", 1)])])],
            ),
            dir("empty", Vec::new()),
        ],
    );

    let kept = Kept::new([("/var/lib/foo", PersistKind::Dir)]);
    let left = tree::without_kept(&root, "/", &kept).unwrap();

    let names: Vec<&str> = left
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();

    assert_eq!(names, ["empty"]);
}

#[test]
fn home_folders_are_found_in_the_scan() {
    let root = dir(
        "/",
        vec![dir("home", vec![dir("alice", vec![file(".cache", 5)])])],
    );

    assert_eq!(tree::find(&root, "/home/alice").unwrap().files, 1);
    assert!(tree::find(&root, "/home/bob").is_none());
    assert_eq!(tree::find(&root, "/").unwrap().name, "/");
}
