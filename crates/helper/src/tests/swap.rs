use std::path::{Path, PathBuf};

use drakeflake_plan::{HEADER, STATE_PREFIX, ValidatedPlan, WritePlan};

use super::refusals::plan_json;
use crate::apply::apply_json;
use crate::error::HelperError;
use crate::{MAX_INPUT_BYTES, secure};

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    names.sort();
    names
}

fn folder() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = secure::canonical(dir.path()).unwrap();
    (dir, root)
}

fn validated(writes: &[(&Path, &str)], removes: &[&Path]) -> ValidatedPlan {
    let plan = WritePlan {
        write: writes
            .iter()
            .map(|(path, text)| (path.to_path_buf(), format!("{HEADER}{text}")))
            .collect(),
        remove: removes.iter().map(|path| path.to_path_buf()).collect(),
    };

    plan.validate_local().unwrap()
}

#[test]
fn a_failed_swap_restores_the_files_already_replaced() {
    let (_dir, root) = folder();
    let first = root.join("a.nix");
    std::fs::write(&first, "old").unwrap();

    let staged = [
        (first.as_path(), "new"),
        (root.join("b.nix").as_path(), "new"),
        (root.join("c.nix").as_path(), "new"),
    ]
    .map(|(path, text)| secure::stage_path(path, text.as_bytes()).unwrap());

    let last = staged.last().unwrap();
    rustix::fs::unlinkat(&last.dir, last.temp.as_str(), rustix::fs::AtFlags::empty()).unwrap();

    secure::swap_all(&staged).unwrap_err();

    assert_eq!(std::fs::read_to_string(&first).unwrap(), "old");
    assert_eq!(names(&root), ["a.nix"]);
}

#[test]
fn a_plan_that_cannot_be_staged_changes_nothing() {
    let (_dir, root) = folder();
    let first = root.join("a.nix");
    std::fs::write(&first, HEADER).unwrap();
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();

    let plan = validated(&[(&first, "new"), (&alias.join("b.nix"), "new")], &[]);

    secure::apply_plan(&plan).unwrap_err();

    assert_eq!(std::fs::read_to_string(&first).unwrap(), HEADER);
    assert_eq!(names(&root), ["a.nix", "alias"]);
}

#[test]
fn a_plan_swaps_in_everything_and_removes_afterwards() {
    let (_dir, root) = folder();
    let old = root.join("old.nix");
    std::fs::write(&old, HEADER).unwrap();
    let part = root.join("part.nix");
    let entry = root.join("entry.nix");

    let plan = validated(&[(&part, "p"), (&entry, "e")], &[&old]);
    secure::apply_plan(&plan).unwrap();

    assert_eq!(names(&root), ["entry.nix", "part.nix"]);
}

#[test]
fn two_spellings_of_one_file_are_refused() {
    let (_dir, root) = folder();
    std::fs::write(root.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let plain = root.join("a.nix");
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    let through_alias = alias.join("a.nix");
    let other = format!("{HEADER}# other\n");

    let result = apply_json(&plan_json(
        &[(&plain, HEADER), (&through_alias, &other)],
        &[],
    ));

    assert!(matches!(result, Err(HelperError::Refused(_))), "{result:?}");
    assert!(!plain.exists());
}

#[test]
fn a_file_cannot_be_written_and_removed_in_one_plan() {
    let (_dir, root) = folder();
    std::fs::write(root.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let file = root.join("a.nix");

    let result = apply_json(&plan_json(&[(&file, HEADER)], &[&file]));

    assert!(matches!(result, Err(HelperError::Refused(_))), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn an_oversized_plan_is_reported_as_too_large() {
    let filler = "x".repeat(usize::try_from(MAX_INPUT_BYTES).unwrap());

    assert!(matches!(
        apply_json(&format!(
            "{{\"write\": {{}}, \"remove\": [], \"pad\": \"{filler}\"}}"
        )),
        Err(HelperError::PlanTooLarge)
    ));
}

#[test]
fn the_entry_file_is_swapped_in_last() {
    let (_dir, root) = folder();
    std::fs::write(root.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let entry = root.join("a-entry.nix");
    let part = root.join("z-part.nix");
    let entry_text = format!("{HEADER}{STATE_PREFIX}{{}}\n{{ }}\n");
    let part_text = format!("{HEADER}{{ }}\n");

    apply_json(&plan_json(
        &[(&entry, &entry_text), (&part, &part_text)],
        &[],
    ))
    .unwrap();

    assert_eq!(names(&root), ["a-entry.nix", "flake.nix", "z-part.nix"]);
}

#[test]
fn a_pipe_as_the_lock_file_is_refused_instead_of_blocking() {
    let (_dir, root) = folder();
    let pipe = root.join("pipe.lock");

    if !std::process::Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .is_ok_and(|status| status.success())
    {
        return;
    }

    let owner = rustix::process::getuid().as_raw();

    secure::read_user_file(&pipe, 1024, Some(owner)).unwrap_err();
}
