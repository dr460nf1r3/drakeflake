use std::fs;

use super::*;

fn git(dir: &Path, args: &[&str]) {
    let dir = dir.to_string_lossy();
    let mut full = vec!["-C", &dir];
    full.extend(args);
    let captured = Context::system()
        .command("git", &full, GIT_TIMEOUT)
        .unwrap();

    assert!(captured.success, "{args:?}: {}", captured.stderr);
}

fn names(dir: &Path) -> Vec<String> {
    nix_files(&Context::system(), dir)
        .iter()
        .map(|path| {
            path.strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn flake() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    fs::create_dir_all(path.join("hosts")).unwrap();
    fs::create_dir_all(path.join(".hidden")).unwrap();
    fs::write(path.join("flake.nix"), "{ }").unwrap();
    fs::write(path.join("hosts/laptop.nix"), "{ }").unwrap();
    fs::write(path.join(".hidden/x.nix"), "{ }").unwrap();
    fs::write(path.join("README.md"), "").unwrap();
    std::os::unix::fs::symlink(path.join("hosts"), path.join("result")).unwrap();
    dir
}

#[test]
fn a_plain_folder_is_walked_without_hidden_folders_and_links() {
    let dir = flake();

    assert_eq!(names(dir.path()), vec!["flake.nix", "hosts/laptop.nix"]);
}

#[test]
fn a_repository_lists_tracked_and_new_files_but_not_ignored_ones() {
    let dir = flake();
    let path = dir.path();
    git(path, &["init", "-q"]);
    fs::write(path.join(".gitignore"), "ignored.nix\nresult\n").unwrap();
    fs::write(path.join("ignored.nix"), "{ }").unwrap();
    git(path, &["add", "flake.nix"]);

    assert_eq!(
        names(path),
        vec![".hidden/x.nix", "flake.nix", "hosts/laptop.nix"]
    );
}
