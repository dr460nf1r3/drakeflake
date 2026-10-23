use super::*;
use crate::Nix;

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn index_key(dir: &Path) -> Option<String> {
    let target = Target {
        flake: dir.to_string_lossy().into_owned(),
        host: "box".into(),
        ..Default::default()
    };

    option_index_key(
        &Context::system(),
        &Nix::system(),
        &target,
        &[dir.join("drakeflake.nix"), dir.join("drakeflake")],
    )
}

fn assert_index_key_ignores_generated_files(dir: &Path) {
    let first = index_key(dir).unwrap();
    assert_eq!(index_key(dir).unwrap(), first, "the key is not stable");

    write(dir, "drakeflake.nix", "{ services.openssh.enable = true; }");
    write(dir, "drakeflake/services.nix", "{ x = 2; }");
    assert_eq!(
        index_key(dir).unwrap(),
        first,
        "generated files changed the key"
    );

    write(dir, "hosts/box.nix", "{ y = 1; }");
    let edited = index_key(dir).unwrap();
    assert_ne!(edited, first, "an edit of the configuration kept the key");

    write(dir, "flake.lock", "{\"version\": 8}");
    assert_ne!(
        index_key(dir).unwrap(),
        edited,
        "a lock update kept the key"
    );

    write(dir, "README.md", "not nix");
    write(dir, "drakeflake.nix.bak", "not nix either");
    let lock_updated = index_key(dir).unwrap();
    write(dir, "README.md", "still not nix");
    assert_eq!(index_key(dir).unwrap(), lock_updated);
}

#[test]
fn option_index_key_ignores_generated_files() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "flake.nix", "{ outputs = _: { }; }");
    write(dir.path(), "flake.lock", "{}");
    write(dir.path(), "hosts/box.nix", "{ }");
    write(dir.path(), "drakeflake.nix", "{ }");
    write(dir.path(), "drakeflake/services.nix", "{ x = 1; }");

    assert_index_key_ignores_generated_files(dir.path());
}

#[test]
fn option_index_key_of_a_git_checkout_covers_tracked_files() {
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let mut all = vec!["-C", dir.path().to_str().unwrap()];
        all.extend_from_slice(args);
        Context::system().command("git", &all, SHORT_TIMEOUT)
    };

    if git(&["init", "-q"]).is_err() {
        return;
    }

    write(dir.path(), "flake.nix", "{ outputs = _: { }; }");
    write(dir.path(), "flake.lock", "{}");
    write(dir.path(), "hosts/box.nix", "{ }");
    write(dir.path(), "drakeflake.nix", "{ }");
    assert!(git(&["add", "-A"]).is_ok_and(|done| done.success));
    let tracked = index_key(dir.path()).unwrap();

    write(dir.path(), "scratch.nix", "{ z = 1; }");
    assert_eq!(index_key(dir.path()).unwrap(), tracked);

    assert_index_key_ignores_generated_files(dir.path());
}

#[test]
fn remote_flakes_have_no_index_key() {
    let target = Target {
        flake: "github:owner/repo".to_owned(),
        host: "box".into(),
        ..Default::default()
    };

    assert_eq!(
        option_index_key(&Context::system(), &Nix::system(), &target, &[]),
        None
    );
}

#[test]
fn hosts_key_follows_the_flake_not_the_generated_files() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    write(
        dir,
        "flake.nix",
        "{ outputs = _: { nixosConfigurations = { }; }; }",
    );
    write(dir, "flake.lock", "{}");
    let generated = [dir.join("drakeflake.nix"), dir.join("drakeflake")];
    let key = || hosts_key(&Context::system(), &dir.to_string_lossy(), &generated).unwrap();

    let first = key();
    assert!(first.starts_with("hosts v"));
    write(dir, "drakeflake.nix", "{ services.openssh.enable = true; }");
    assert_eq!(key(), first, "a generated file changed the host key");
    write(dir, "hosts/new.nix", "{ }");
    assert_ne!(key(), first, "a new host file did not change the key");
    assert_eq!(
        hosts_key(&Context::system(), "github:owner/repo", &generated),
        None
    );
}

#[test]
fn a_plain_folder_lists_dotfiles_but_not_git() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    for file in [
        "flake.nix",
        "hosts/box/default.nix",
        ".git/config",
        ".direnv/flake-profile.nix",
        "hosts/.secret.nix",
    ] {
        write(dir, file, "{ }");
    }

    let mut files = walk_files(dir);
    files.sort();

    assert_eq!(
        files,
        [
            PathBuf::from(".direnv/flake-profile.nix"),
            PathBuf::from("flake.nix"),
            PathBuf::from("hosts/.secret.nix"),
            PathBuf::from("hosts/box/default.nix")
        ]
    );
}
