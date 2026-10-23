use super::*;

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    names.sort();
    names
}

#[test]
fn a_failed_swap_restores_the_files_already_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.nix");
    let created = dir.path().join("b.nix");
    let lost = dir.path().join("c.nix");
    fs::write(&first, "old").unwrap();

    let mut staged = Vec::new();

    for (path, text) in [(&first, "new"), (&created, "new"), (&lost, "new")] {
        stage_next(path, text, &mut staged).unwrap();
    }

    fs::remove_file(&staged.last().unwrap().temp).unwrap();

    swap_all(&staged).unwrap_err();

    assert_eq!(fs::read_to_string(&first).unwrap(), "old");
    assert_eq!(names(dir.path()), ["a.nix"]);
}

#[test]
fn a_failed_stage_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let ok = dir.path().join("a.nix");
    let mut staged = Vec::new();

    stage_next(&ok, "text", &mut staged).unwrap();

    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "").unwrap();
    let nested = blocker.join("b.nix");
    stage_next(&nested, "text", &mut staged).unwrap_err();
    discard(&staged);

    assert_eq!(names(dir.path()), ["blocker"]);
}

#[test]
fn a_swap_keeps_no_backup_behind() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("a.nix");
    fs::write(&target, "old").unwrap();
    let mut staged = Vec::new();

    stage_next(&target, "new", &mut staged).unwrap();
    swap_all(&staged).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    assert_eq!(names(dir.path()), ["a.nix"]);
}
