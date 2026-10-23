use super::*;

fn ran(success: bool, stdout: &str, stderr: &str) -> Captured {
    Captured {
        success,
        code: Some(if success { 0 } else { 128 }),
        stdout: stdout.to_owned(),
        stderr: stderr.to_owned(),
    }
}

#[test]
fn only_a_work_tree_goes_on_to_the_tracking_check() {
    let dubious = "fatal: detected dubious ownership in repository at '/etc/nixos'";

    for (probe, expected) in [
        (Ok(ran(true, "true\n", "")), None),
        (Ok(ran(true, "false\n", "")), Some(GitState::NotARepository)),
        (
            Ok(ran(false, "", "fatal: not a git repository")),
            Some(GitState::NotARepository),
        ),
        (Ok(ran(false, "", dubious)), Some(GitState::Refused)),
        // An unreadable repository must not be reported as absent, or a save would run git init inside it.
        (
            Ok(ran(false, "", "fatal: unable to read config file")),
            Some(GitState::Unknown),
        ),
        (
            Err(ExecError::Failed("git: not found".to_owned())),
            Some(GitState::Unknown),
        ),
    ] {
        assert_eq!(
            outside_work_tree(&probe, Path::new("/etc/nixos")),
            expected,
            "{probe:?}"
        );
    }
}

#[test]
fn ls_files_decides_whether_the_file_is_tracked() {
    let tracked = |probe| tracking(&probe, Path::new("/etc/nixos"), "drakeflake.nix");

    assert_eq!(
        tracked(Ok(ran(true, "drakeflake.nix\n", ""))),
        GitState::Tracked
    );
    assert_eq!(
        tracked(Ok(ran(false, "", "error: pathspec did not match"))),
        GitState::Untracked
    );
    assert_eq!(
        tracked(Err(ExecError::Failed("timed out".to_owned()))),
        GitState::Unknown
    );
}
