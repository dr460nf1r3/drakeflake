use drakeflake_exec::Context;

use super::fixture_copy;
use crate::{GitState, git_state, git_track};

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and git"]
fn vm_git_tracking_roundtrip() {
    let Some((dir, _copy)) = fixture_copy() else {
        return;
    };

    let status = std::process::Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "init", "-q"])
        .status()
        .expect("git init");

    assert!(status.success(), "git init failed");

    let added = dir.join("drakeflake-intent.txt");
    std::fs::write(&added, "-owned\n").unwrap();
    assert_eq!(git_state(&Context::system(), &added), GitState::Untracked);
    git_track(&Context::system(), &added).unwrap();
    assert_eq!(git_state(&Context::system(), &added), GitState::Tracked);
}
