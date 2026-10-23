use std::collections::{BTreeMap, BTreeSet};

use drakeflake_flake::lock::LockedInput;
use drakeflake_plan::ManagedOption;

use super::*;

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

fn locked(name: &str, rev: &str) -> LockedInput {
    LockedInput {
        name: name.to_owned(),
        rev: Some(rev.to_owned()),
        ..LockedInput::default()
    }
}

#[test]
fn added_removed_and_changed_options_become_bullets() {
    let before = state(&[("a.kept", "true"), ("a.gone", "true"), ("a.flip", "true")]);
    let after = state(&[("a.kept", "true"), ("a.flip", "false"), ("a.new", "true")]);

    let (subject, body) = commit_message(Some(&before), &after, Some(&[]), &[]);

    assert_eq!(subject, "drakeflake: update config");
    assert_eq!(body, "- a.flip changed\n- a.gone removed\n- a.new added");
}

#[test]
fn unknown_previous_settings_claim_nothing() {
    let (_, body) = commit_message(None, &state(&[("a.new", "true")]), Some(&[]), &[]);

    assert!(body.is_empty(), "{body}");
}

#[test]
fn package_changes_name_the_package_list() {
    let mut before = state(&[("a.kept", "true")]);
    let mut after = before.clone();
    after.packages.insert("hello".into());

    let (_, body) = commit_message(Some(&before), &after, Some(&[]), &[]);

    assert_eq!(body, "- environment.systemPackages added");
    before.packages.insert("hello".into());
    after.packages.clear();

    let (_, removed) = commit_message(Some(&before), &after, Some(&[]), &[]);

    assert_eq!(removed, "- environment.systemPackages removed");
}

#[test]
fn re_pinned_inputs_show_old_and_new_revisions() {
    let old = vec![locked("nixpkgs", "aaaaaaaabbbb"), locked("home", "cccc")];
    let new = vec![locked("nixpkgs", "ddddddddeeee"), locked("home", "cccc")];

    let (_, body) = commit_message(Some(&state(&[])), &state(&[]), Some(&old), &new);

    assert_eq!(body, "- nixpkgs: aaaaaaa -> ddddddd");
}

#[test]
fn added_and_removed_inputs_become_bullets() {
    let old = vec![locked("gone", "aaaa")];
    let new = vec![locked("gone", "aaaa"), locked("fresh", "bbbb")];

    let (_, body) = commit_message(Some(&state(&[])), &state(&[]), Some(&old), &new);

    assert_eq!(body, "- fresh added");
}

#[test]
fn unknown_committed_lock_claims_nothing() {
    let (_, body) = commit_message(
        Some(&state(&[])),
        &state(&[]),
        None,
        &[locked("nixpkgs", "aaaa")],
    );

    assert!(body.is_empty(), "{body}");
}

#[test]
fn long_lists_are_cut_off_with_a_count() {
    let before = ManagedState::default();
    let after = ManagedState {
        options: (0_i32..25_i32)
            .map(|n| (OptionName::from(format!("a.opt{n:02}")), option("true")))
            .collect::<BTreeMap<_, _>>(),
        packages: BTreeSet::new(),
    };

    let (_, body) = commit_message(Some(&before), &after, Some(&[]), &[]);

    assert_eq!(body.lines().count(), 21);
    assert!(body.ends_with("- … and 5 more changes"), "{body}");
}
