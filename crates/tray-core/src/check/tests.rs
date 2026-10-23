use std::cell::RefCell;

use super::*;

fn update(name: &str) -> InputUpdate {
    InputUpdate {
        name: name.to_owned(),
        locked_at: None,
        link: None,
    }
}

#[test]
fn notifies_once_per_new_set_of_updates() {
    let shown = RefCell::new(Vec::new());
    let notify = |summary: &str, body: &str| shown.borrow_mut().push(format!("{summary}: {body}"));
    let mut notified = Vec::new();

    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);
    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);
    report(&Err(CheckError::NotLocal), &mut notified, notify);
    report(&Ok(Vec::new()), &mut notified, notify);
    report(
        &Ok(vec![update("nixpkgs"), update("sops-nix")]),
        &mut notified,
        notify,
    );

    assert_eq!(
        *shown.borrow(),
        [
            "Flake updates available: nixpkgs can be updated. Open DrakeFlake → Inputs to update.",
            "Flake updates available: nixpkgs, sops-nix can be updated. Open DrakeFlake → Inputs to update.",
        ]
    );
}

#[test]
fn an_input_updated_in_between_is_announced_again() {
    let shown = RefCell::new(0_u32);
    let notify = |_: &str, _: &str| *shown.borrow_mut() += 1;
    let mut notified = Vec::new();

    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);
    report(&Ok(Vec::new()), &mut notified, notify);
    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);

    assert_eq!(*shown.borrow(), 2);
}

#[test]
fn a_failed_check_keeps_what_was_announced() {
    let shown = RefCell::new(0_u32);
    let notify = |_: &str, _: &str| *shown.borrow_mut() += 1;
    let mut notified = Vec::new();

    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);
    report(&Err(CheckError::NotLocal), &mut notified, notify);
    report(&Ok(vec![update("nixpkgs")]), &mut notified, notify);

    assert_eq!(*shown.borrow(), 1);
}

#[test]
fn input_labels_show_the_lock_age() {
    let now = 1_800_000_000;
    let locked = |seconds_ago: i64| InputUpdate {
        locked_at: Some(now - seconds_ago),
        ..update("nixpkgs")
    };

    assert_eq!(update("nixpkgs").label(now), "nixpkgs");
    assert_eq!(locked(3600).label(now), "nixpkgs — locked today");
    assert_eq!(
        locked(DAY_SECONDS + 5).label(now),
        "nixpkgs — locked 1 day ago"
    );
    assert_eq!(
        locked(9 * DAY_SECONDS).label(now),
        "nixpkgs — locked 9 days ago"
    );

    assert_eq!(locked(-60).label(now), "nixpkgs — locked today");
}

#[test]
fn updates_link_to_the_changes_or_the_repository() {
    let input = |web_url: Option<&str>| LockedInput {
        name: "nixpkgs".to_owned(),
        last_modified: Some(5),
        web_url: web_url.map(str::to_owned),
        ..LockedInput::default()
    };

    let available = |compare_url: Option<&str>| UpdateCheck::Available {
        rev: "new".to_owned(),
        compare_url: compare_url.map(str::to_owned),
        lock_url: None,
    };

    let compared = InputUpdate::new(&input(Some("https://repo")), available(Some("https://cmp")));
    assert_eq!(
        compared.and_then(|update| update.link).as_deref(),
        Some("https://cmp")
    );
    let repo_only = InputUpdate::new(&input(Some("https://repo")), available(None));
    assert_eq!(
        repo_only.and_then(|update| update.link).as_deref(),
        Some("https://repo")
    );
    assert_eq!(InputUpdate::new(&input(None), UpdateCheck::UpToDate), None);
}
