use std::collections::BTreeSet;

use drakeflake_plan::{ManagedOption, ManagedState};
use drakeflake_render::OptionName;

use super::super::message::{COMMIT_SUBJECT, commit_message};
use super::*;

const LOCK: &str = r#"{
    "nodes": {
        "nixpkgs": {"locked": {"rev": "aaaaaaaabbbb"}},
        "root": {"inputs": {"nixpkgs": "nixpkgs"}}
    },
    "root": "root",
    "version": 7
}"#;

fn option(input: &str) -> ManagedOption {
    ManagedOption {
        type_name: "bool".to_owned(),
        input: input.to_owned(),
        ..ManagedOption::default()
    }
}

fn state(entries: &[(&str, &str)]) -> ManagedState {
    ManagedState {
        options: entries
            .iter()
            .map(|(name, input)| (OptionName::from(*name), option(input)))
            .collect(),
        packages: BTreeSet::new(),
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let dir = dir.to_string_lossy();
    let mut full = vec!["-C", &dir];
    full.extend(args);
    let captured = Context::system()
        .command("git", &full, GIT_TIMEOUT)
        .unwrap();

    assert!(captured.success, "{args:?}: {}", captured.stderr);
    captured.stdout
}

fn repo_with_lock() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "test@example.com"]);
    git(path, &["config", "user.name", "Test"]);
    git(path, &["config", "commit.gpgsign", "false"]);
    std::fs::write(path.join("drakeflake.nix"), "{ }\n").unwrap();
    std::fs::write(path.join("flake.lock"), LOCK).unwrap();
    git(path, &["add", "-A"]);
    git(path, &["commit", "-qm", "start"]);
    dir
}

#[test]
fn staging_and_committing_round_trip() {
    let dir = repo_with_lock();
    let path = dir.path();
    std::fs::write(path.join("drakeflake.nix"), "{ a = 1; }\n").unwrap();

    let staged = stage_all(&Context::system(), path).unwrap();

    assert_eq!(staged, vec!["drakeflake.nix".to_owned()]);

    let before = state(&[("a.old", "true")]);
    let after = state(&[("a.old", "true"), ("a.new", "true")]);
    let old_lock = head_lock(&Context::system(), path).unwrap();
    let (subject, mut body) =
        commit_message(Some(&before), &after, Some(&old_lock), &worktree_lock(path));

    if body.is_empty() {
        body = staged
            .iter()
            .map(|file| format!("- {file} updated"))
            .collect::<Vec<_>>()
            .join("\n");
    }

    let summary = commit_staged(&Context::system(), path, &subject, &body).unwrap();

    assert!(summary.contains("drakeflake: update config"), "{summary}");

    let log = git(path, &["log", "-1", "--format=%s%n%b"]);

    assert_eq!(log, "drakeflake: update config\n- a.new added\n\n");
}

#[test]
fn clean_checkout_stages_nothing() {
    let dir = repo_with_lock();

    assert_eq!(
        stage_all(&Context::system(), dir.path()).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn committed_lock_is_read_back() {
    let dir = repo_with_lock();
    let locked = head_lock(&Context::system(), dir.path()).unwrap();

    assert_eq!(locked.len(), 1);
    assert_eq!(locked[0].name, "nixpkgs");
}

#[test]
fn a_folder_without_repository_gets_one_and_its_first_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    std::fs::write(path.join("flake.nix"), "{ outputs = _: { }; }\n").unwrap();

    assert!(ensure_repository(&Context::system(), path).unwrap());
    assert!(
        !ensure_repository(&Context::system(), path).unwrap(),
        "created twice"
    );

    git(path, &["config", "user.email", "test@example.com"]);
    git(path, &["config", "user.name", "Test"]);
    git(path, &["config", "commit.gpgsign", "false"]);

    assert_eq!(
        stage_all(&Context::system(), path).unwrap(),
        vec!["flake.nix".to_owned()]
    );
    assert_eq!(head_lock(&Context::system(), path), None);

    commit_staged(
        &Context::system(),
        path,
        COMMIT_SUBJECT,
        "- flake.nix updated",
    )
    .unwrap();

    assert_eq!(
        git(path, &["log", "--format=%s"]),
        "drakeflake: update config\n"
    );
}

#[test]
fn only_the_flake_folder_is_staged_and_committed() {
    let dir = repo_with_lock();
    let path = dir.path();
    let flake = path.join("nixos");
    std::fs::create_dir_all(&flake).unwrap();
    std::fs::write(flake.join("flake.nix"), "{ }\n").unwrap();
    std::fs::write(path.join("drakeflake.nix"), "{ a = 1; }\n").unwrap();
    std::fs::write(path.join("notes.txt"), "mine\n").unwrap();
    git(path, &["add", "notes.txt"]);

    assert!(!ensure_repository(&Context::system(), &flake).unwrap());
    assert_eq!(
        stage_all(&Context::system(), &flake).unwrap(),
        vec!["flake.nix".to_owned()]
    );

    commit_staged(
        &Context::system(),
        &flake,
        COMMIT_SUBJECT,
        "- flake.nix updated",
    )
    .unwrap();

    assert_eq!(
        git(path, &["show", "--name-only", "--format="]),
        "nixos/flake.nix\n"
    );

    assert_eq!(
        git(path, &["status", "--porcelain"]),
        " M drakeflake.nix\nA  notes.txt\n"
    );
}

#[test]
fn the_committed_lock_of_a_flake_folder_is_found() {
    let dir = repo_with_lock();
    let path = dir.path();
    let flake = path.join("nixos");
    std::fs::create_dir_all(&flake).unwrap();
    std::fs::write(flake.join("flake.lock"), LOCK).unwrap();
    git(path, &["add", "-A"]);
    git(path, &["commit", "-qm", "nested"]);

    assert_eq!(
        head_lock(&Context::system(), &flake).map(|lock| lock.len()),
        Some(1)
    );
}

#[test]
fn only_missing_identity_parts_fall_back() {
    assert_eq!(
        fallback_identity(true, true, Some("box")),
        Vec::<String>::new()
    );
    assert_eq!(
        fallback_identity(false, true, Some("box")),
        vec!["-c", "user.name=DrakeFlake"]
    );
    assert_eq!(
        fallback_identity(true, false, Some("box")),
        vec!["-c", "user.email=drakeflake@box"]
    );

    assert_eq!(
        fallback_identity(true, false, Some("")),
        vec!["-c", "user.email=drakeflake@localhost"]
    );
}

#[test]
fn a_repository_without_identity_commits_as_drakeflake() {
    let dir = repo_with_lock();
    let path = dir.path();

    git(path, &["config", "user.name", ""]);
    git(path, &["config", "user.email", ""]);
    std::fs::write(path.join("drakeflake.nix"), "{ a = 1; }\n").unwrap();
    stage_all(&Context::system(), path).unwrap();

    commit_staged(
        &Context::system(),
        path,
        COMMIT_SUBJECT,
        "- drakeflake.nix updated",
    )
    .unwrap();

    let author = git(path, &["log", "-1", "--format=%an <%ae>"]);

    assert!(author.starts_with("DrakeFlake <drakeflake@"), "{author}");
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and git"]
fn vm_git_roundtrip() {
    let Ok(source) = std::env::var("DRAKEFLAKE_TEST_FLAKE") else {
        return;
    };

    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("flake");

    for (program, args) in [("cp", ["-r", &source]), ("chmod", ["-R", "u+w"])] {
        let status = std::process::Command::new(program)
            .args(args)
            .arg(&dir)
            .status()
            .unwrap();

        assert!(status.success(), "{program} the fixture copy failed");
    }

    assert!(
        ensure_repository(&Context::system(), &dir).unwrap(),
        "no repository created"
    );
    assert!(
        !ensure_repository(&Context::system(), &dir).unwrap(),
        "existing repository remade"
    );

    std::fs::write(dir.join("drakeflake-test.txt"), "-owned\n").unwrap();
    let staged = stage_all(&Context::system(), &dir).unwrap();
    assert!(
        staged.iter().any(|path| path == "drakeflake-test.txt"),
        "{staged:?}"
    );

    let summary = commit_staged(
        &Context::system(),
        &dir,
        "integration test",
        "vm_git_roundtrip",
    )
    .unwrap();

    assert!(!summary.is_empty(), "empty commit summary");

    let locked = head_lock(&Context::system(), &dir).expect("no lock at HEAD after the commit");

    assert!(
        locked.iter().any(|input| input.name == "nixpkgs"),
        "{locked:?}"
    );
}
