use std::collections::BTreeSet;

use super::*;

#[test]
fn suffixes_never_repeat_within_a_process() {
    let suffixes: BTreeSet<String> = std::iter::repeat_with(unique_suffix).take(1000).collect();
    assert_eq!(suffixes.len(), 1000);
}

#[test]
fn writes_replace_the_file_and_leave_no_temporary_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.toml");
    fs::write(&path, "old").unwrap();

    atomic_write(&path, "new").unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_failed_write_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("a.toml");

    atomic_write(&path, "text").unwrap_err();

    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_sibling_is_hidden_and_named_after_its_target() {
    let name = sibling(Path::new("/x/a.nix"), "old");
    let text = name.to_string_lossy();

    assert!(text.starts_with("/x/.a.nix.old-"), "{text}");
}
