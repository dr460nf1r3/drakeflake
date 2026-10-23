use super::*;
use crate::error::CheckError;

#[test]
fn summaries() {
    assert_eq!(
        State::default().summary(),
        "Checks your flake inputs for updates."
    );
    assert_eq!(
        State {
            checking: true,
            ..State::default()
        }
        .summary(),
        "Checking for updates…"
    );

    let update = |name: &str| InputUpdate {
        name: name.to_owned(),
        locked_at: None,
        link: None,
    };

    let found = State::from_result(Ok(vec![update("nixpkgs"), update("home-manager")]), &[]);
    assert!(found.needs_attention());
    assert_eq!(found.summary(), "Updates available: nixpkgs, home-manager");

    let current = State::from_result(Ok(Vec::new()), &found.updates);
    assert!(!current.needs_attention());
    assert_eq!(current.summary(), "Your flake inputs are up to date.");

    let failed = State::from_result(Err(CheckError::NotLocal), &found.updates);
    assert_eq!(
        failed.summary(),
        "Could not check for updates: the configured flake is not a local directory"
    );
    assert_eq!(failed.updates, found.updates);
    assert!(failed.needs_attention());
}

#[test]
fn health_findings_ask_for_attention() {
    let restart = State {
        health: Health {
            restart_needed: true,
            ..Health::default()
        },
        ..State::default()
    };

    assert!(restart.needs_attention());
}
