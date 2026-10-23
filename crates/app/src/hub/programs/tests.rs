use super::{find_program, find_root_program, in_store};

#[test]
fn a_program_outside_the_store_never_runs_as_root() {
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("nix");
    std::fs::write(&program, "#!/bin/sh\n").unwrap();

    let refused = in_store(&program).unwrap_err();
    assert!(refused.contains("not in the Nix store"), "{refused}");
}

#[test]
fn a_link_is_judged_by_where_it_leads() {
    let Some(nix) = find_program("nix").and_then(|path| path.canonicalize().ok()) else {
        return;
    };

    if !nix.starts_with("/nix/store") {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("nix");
    std::os::unix::fs::symlink(&nix, &link).unwrap();

    assert_eq!(in_store(&link).unwrap(), nix);
}

#[test]
fn a_missing_program_is_reported_by_name() {
    let missing = find_root_program("drakeflake-no-such-program").unwrap_err();
    assert_eq!(missing, "drakeflake-no-such-program is not installed.");
}
