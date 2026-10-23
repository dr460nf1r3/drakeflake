use std::fs;
use std::path::{Path, PathBuf};

use super::*;

fn tree(root: &Path, files: &[&str]) {
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        if file.ends_with('/') {
            fs::create_dir_all(&path).unwrap();
        } else {
            fs::write(&path, "1234").unwrap();
        }
    }
}

fn child<'a>(node: &'a ScanNode, name: &str) -> Option<&'a ScanNode> {
    node.children.iter().find(|child| child.name == name)
}

#[test]
fn lists_entries_with_sizes_and_skips_kernel_folders() {
    let temp = tempfile::tempdir().unwrap();
    tree(
        temp.path(),
        &[
            "etc/machine-id",
            "var/lib/foo/state",
            "var/lib/foo/db",
            "proc/1",
            "nix/store/x",
        ],
    );

    let root = scan(temp.path(), None, &[], ScanLimits::default()).unwrap();

    assert_eq!(root.name, "/");
    assert_eq!(root.files, 3);
    assert_eq!(root.size, 12);
    assert!(child(&root, "proc").is_none());
    assert!(child(&root, "nix").is_none());

    let foo = child(child(child(&root, "var").unwrap(), "lib").unwrap(), "foo").unwrap();
    assert_eq!(foo.kind, EntryKind::Dir);
    assert_eq!(foo.files, 2);
    assert_eq!(child(foo, "db").unwrap().kind, EntryKind::File);
}

#[test]
fn mount_points_are_left_out() {
    let temp = tempfile::tempdir().unwrap();
    tree(temp.path(), &["var/log/syslog", "etc/hosts"]);

    let mounts = vec![temp.path().join("var/log"), PathBuf::from("/elsewhere")];
    let root = scan(temp.path(), None, &mounts, ScanLimits::default()).unwrap();

    let var = child(&root, "var").unwrap();
    assert!(child(var, "log").is_none());
    assert_eq!(root.files, 1);
}

#[test]
fn snapshot_entries_survive_and_new_folders_are_listed_whole() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("live");
    let blank = temp.path().join("blank");
    tree(&live, &["etc/hosts", "etc/added", "srv/data/a", "empty/"]);
    tree(&blank, &["etc/hosts", "empty/"]);

    let root = scan(&live, Some(&blank), &[], ScanLimits::default()).unwrap();

    let etc = child(&root, "etc").unwrap();
    assert!(child(etc, "hosts").is_none());
    assert!(child(etc, "added").is_some());
    assert!(child(&root, "empty").is_none());
    assert_eq!(child(&root, "srv").unwrap().files, 1);
    assert_eq!(root.files, 2);
}

#[test]
fn limits_hide_the_smallest_entries_but_keep_their_size() {
    let temp = tempfile::tempdir().unwrap();
    tree(temp.path(), &["d/a", "d/b", "d/c", "deep/x/y/z"]);
    fs::write(temp.path().join("d/big"), "x".repeat(100)).unwrap();

    let limits = ScanLimits {
        depth: 2,
        per_dir: 2,
        nodes: 100,
        ..ScanLimits::default()
    };

    let root = scan(temp.path(), None, &[], limits).unwrap();

    let dir = child(&root, "d").unwrap();
    assert_eq!(dir.children.len(), 2);
    assert!(child(dir, "big").is_some());
    assert_eq!(dir.hidden, 2);
    assert_eq!(dir.size, 112);

    let deep = child(child(&root, "deep").unwrap(), "x").unwrap();
    assert_eq!(deep.children, []);
    assert_eq!((deep.files, deep.hidden), (1, 1));
}

#[test]
fn mountinfo_mount_points_are_unescaped() {
    let text = "22 1 0:21 / / rw - tmpfs none rw\n\
                30 22 0:30 /persist/var/log /var/log rw - btrfs /dev/sda2 rw\n\
                31 22 0:31 / /mnt/my\\040disk rw - ext4 /dev/sdb1 rw\n";

    assert_eq!(
        parse_mountinfo(text),
        vec![
            PathBuf::from("/"),
            PathBuf::from("/var/log"),
            PathBuf::from("/mnt/my disk")
        ]
    );
}

#[test]
fn the_topmost_mount_names_the_root_file_system() {
    let text = "22 1 0:21 / / rw - tmpfs none rw\n\
                23 22 0:22 /root / rw - btrfs /dev/mapper/crypt rw,subvol=/root\n\
                30 22 0:30 /persist /persist rw - btrfs /dev/mapper/crypt rw\n";

    assert_eq!(
        mount_source(text, Path::new("/")),
        Some(("btrfs".to_owned(), "/dev/mapper/crypt".to_owned()))
    );
    assert_eq!(mount_source(text, Path::new("/boot")), None);
}

#[test]
fn the_top_level_is_listed_even_when_a_deep_subtree_is_large() {
    let temp = tempfile::tempdir().unwrap();

    for index in 0_u32..30 {
        tree(temp.path(), &[&format!("aaa/sub{index}/file")]);
    }

    tree(temp.path(), &["zzz/small"]);

    let limits = ScanLimits {
        depth: 4,
        per_dir: 50,
        nodes: 20,
        ..ScanLimits::default()
    };

    let root = scan(temp.path(), None, &[], limits).unwrap();

    assert!(child(&root, "aaa").is_some(), "the large folder is missing");
    assert!(
        child(&root, "zzz").is_some(),
        "the top level has no listing"
    );
    assert_eq!(root.children.len(), 2);
}

#[test]
fn mount_points_keep_bytes_that_are_not_utf8() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let text = "31 22 0:31 / /mnt/\\377x rw - ext4 /dev/sdb1 rw\n";

    assert_eq!(
        parse_mountinfo(text),
        vec![PathBuf::from(OsString::from_vec(b"/mnt/\xFFx".to_vec()))]
    );
}

#[test]
fn only_three_octal_digits_are_an_escape() {
    let text = "31 22 0:31 / /mnt/a\\+12 rw - ext4 /dev/sdb1 rw\n\
                32 22 0:32 / /mnt/b\\089 rw - ext4 /dev/sdc1 rw\n\
                33 22 0:33 / /mnt/c\\04 rw - ext4 /dev/sdd1 rw\n";

    assert_eq!(
        parse_mountinfo(text),
        vec![
            PathBuf::from("/mnt/a\\+12"),
            PathBuf::from("/mnt/b\\089"),
            PathBuf::from("/mnt/c\\04")
        ]
    );
}

#[test]
fn a_new_folder_below_the_depth_limit_still_counts() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("live");
    let blank = temp.path().join("blank");
    tree(&live, &["a/b/new/"]);
    tree(&blank, &["a/b/"]);

    let limits = ScanLimits {
        depth: 1,
        per_dir: 10,
        nodes: 10,
        ..ScanLimits::default()
    };

    let root = scan(&live, Some(&blank), &[], limits).unwrap();

    let listed = child(&root, "a").unwrap();
    assert_eq!(listed.children, []);
    assert_eq!((listed.files, listed.hidden), (0, 1));
}

#[test]
fn a_directory_with_too_many_entries_is_cut_short_and_marked() {
    let temp = tempfile::tempdir().unwrap();
    let files: Vec<String> = (0_u32..10_u32).map(|n| format!("big/f{n}")).collect();
    tree(
        temp.path(),
        &files.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    let limits = ScanLimits {
        visit_per_dir: 3,
        ..ScanLimits::default()
    };

    let root = scan(temp.path(), None, &[], limits).unwrap();

    let big = child(&root, "big").unwrap();

    assert_eq!(big.children.len(), 3);
    assert_eq!(big.files, 3);
    assert!(big.hidden >= 1, "{}", big.hidden);
}

#[test]
fn a_directory_at_exactly_the_limit_is_not_marked() {
    let temp = tempfile::tempdir().unwrap();
    tree(temp.path(), &["big/a", "big/b", "big/c"]);

    let limits = ScanLimits {
        visit_per_dir: 3,
        ..ScanLimits::default()
    };

    let root = scan(temp.path(), None, &[], limits).unwrap();

    assert_eq!(child(&root, "big").unwrap().hidden, 0);
}

#[test]
fn the_total_limit_stops_the_walk_from_descending() {
    let temp = tempfile::tempdir().unwrap();
    let files: Vec<String> = (0_u32..6_u32)
        .flat_map(|dir| (0_u32..6_u32).map(move |file| format!("d{dir}/f{file}")))
        .collect();

    tree(
        temp.path(),
        &files.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    let limits = ScanLimits {
        visit_total: 10,
        ..ScanLimits::default()
    };

    let root = scan(temp.path(), None, &[], limits).unwrap();

    assert!(root.files <= 10, "{}", root.files);
    assert!(root.hidden >= 1 || root.children.iter().any(|dir| dir.hidden >= 1));
}

#[test]
fn symlinks_are_listed_not_followed() {
    let temp = tempfile::tempdir().unwrap();
    tree(temp.path(), &["real/file", "dir/"]);
    std::os::unix::fs::symlink(temp.path().join("real"), temp.path().join("loop")).unwrap();
    std::os::unix::fs::symlink(temp.path().join("real/file"), temp.path().join("alias")).unwrap();

    let root = scan(temp.path(), None, &[], ScanLimits::default()).unwrap();

    assert_eq!(root.files, 3, "{root:?}");
    let names: Vec<&str> = root
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();

    assert!(names.contains(&"loop"), "{names:?}");
    assert!(names.contains(&"alias"), "{names:?}");
    assert_eq!(
        child(&root, "loop").map(|node| node.kind),
        Some(EntryKind::Link)
    );
}
