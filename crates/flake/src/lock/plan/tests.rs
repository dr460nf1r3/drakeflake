use super::*;

fn input(name: &str, rev: &str) -> LockedInput {
    LockedInput {
        name: name.to_owned(),
        rev: Some(rev.to_owned()),
        ..LockedInput::default()
    }
}

fn available(rev: &str, lock_url: Option<&str>) -> UpdateCheck {
    UpdateCheck::Available {
        rev: rev.to_owned(),
        compare_url: None,
        lock_url: lock_url.map(str::to_owned),
    }
}

fn plan_of(
    inputs: &[LockedInput],
    checks: impl IntoIterator<Item = UpdateCheck>,
    names: &[String],
) -> UpdatePlan {
    let checked: Vec<_> = inputs.iter().zip(checks).collect();

    UpdatePlan::new(&checked, names)
}

#[test]
fn known_versions_are_locked_directly() {
    let inputs = [
        input("hm", "aaaa"),
        input("nixpkgs", "bbbb"),
        input("sops", "cccc"),
    ];

    let checks = [
        available("dddd", Some("github:nix-community/home-manager/dddd")),
        UpdateCheck::UpToDate,
        available("eeee", None),
    ];

    let plan = plan_of(&inputs, checks, &[]);

    assert_eq!(
        plan.args(),
        [
            "flake",
            "update",
            "sops",
            "--override-input",
            "hm",
            "github:nix-community/home-manager/dddd"
        ]
    );
    assert_eq!(plan.expected, [("hm".to_owned(), "dddd".to_owned())]);
}

#[test]
fn only_overrides_use_flake_lock() {
    let inputs = [input("hm", "aaaa"), input("other", "bbbb")];
    let checks = [
        available("dddd", Some("github:a/b/dddd")),
        available("ffff", Some("github:c/d/ffff")),
    ];

    let plan = plan_of(&inputs, checks, &["hm".to_owned()]);

    assert_eq!(
        plan.args(),
        ["flake", "lock", "--override-input", "hm", "github:a/b/dddd"]
    );
    assert!(plan_of(&inputs, [UpdateCheck::UpToDate, UpdateCheck::UpToDate], &[]).is_empty());
}

#[test]
fn inputs_with_option_like_names_never_enter_a_plan() {
    let inputs = [input("--commit-lock-file", "aaaa"), input("-x", "bbbb")];
    let checks = [
        available("dddd", Some("github:a/b/dddd")),
        UpdateCheck::Failed {
            error: "offline".to_owned(),
        },
    ];

    let plan = plan_of(&inputs, checks.clone(), &[]);

    assert!(plan.is_empty());
    assert_eq!(plan.expected, Vec::<(String, String)>::new());

    let named = plan_of(&inputs, checks, &["-x".to_owned()]);
    assert!(named.is_empty());
}

#[test]
fn inputs_that_did_not_move_are_reported() {
    let inputs = [input("hm", "aaaa")];
    let plan = plan_of(&inputs, [available("dddd", Some("github:a/b/dddd"))], &[]);

    assert_eq!(plan.not_updated(&inputs), ["hm"]);
    assert_eq!(
        plan.not_updated(&[input("hm", "dddd")]),
        Vec::<String>::new()
    );
}

#[test]
fn unknown_or_failed_checks_are_left_to_nix() {
    let not_checkable = || UpdateCheck::NotCheckable {
        reason: "local path".to_owned(),
    };

    let inputs = [
        input("local", "aaaa"),
        LockedInput {
            pinned: true,
            ..input("pinned", "bbbb")
        },
        LockedInput {
            follows: Some("local".to_owned()),
            ..input("stable", "cccc")
        },
        input("offline", "dddd"),
        input("current", "eeee"),
    ];

    let checks = [
        not_checkable(),
        not_checkable(),
        not_checkable(),
        UpdateCheck::Failed {
            error: "no network".to_owned(),
        },
        UpdateCheck::UpToDate,
    ];

    let plan = plan_of(&inputs, checks, &[]);

    assert_eq!(plan.update, ["local", "offline"]);
    assert_eq!(plan.overrides, Vec::<(String, String)>::new());
    assert_eq!(plan.args(), ["flake", "update", "local", "offline"]);
}

#[test]
fn a_tarball_named_by_its_release_is_verified_by_url() {
    let inputs = [LockedInput {
        locked_url: Some("https://releases.nixos.org/a/nixexprs.tar.zst".to_owned()),
        ..input("nixpkgs", "aaaa")
    }];

    let link = "https://releases.nixos.org/nixos/unstable/nixos-26.11pre2/nixexprs.tar.zst";

    let plan = plan_of(&inputs, [available("nixos-26.11pre2", Some(link))], &[]);

    assert_eq!(plan.overrides, [("nixpkgs".to_owned(), link.to_owned())]);
    assert_eq!(plan.expected, Vec::<(String, String)>::new());
    assert_eq!(plan.expected_url, [("nixpkgs".to_owned(), link.to_owned())]);

    assert_eq!(plan.not_updated(&inputs), ["nixpkgs"]);

    let moved = [LockedInput {
        locked_url: Some(link.to_owned()),
        ..input("nixpkgs", "aaaa")
    }];

    assert_eq!(plan.not_updated(&moved), Vec::<String>::new());
}
