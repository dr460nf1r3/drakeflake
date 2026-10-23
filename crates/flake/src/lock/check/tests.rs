use drakeflake_exec::testing::{ScriptedRunner, failure};

use super::*;

const OLD: &str = "aaaaaaa1";
const NEW: &str = "bbbbbbb2";

fn github(pin_base: Option<&str>) -> LockedInput {
    LockedInput {
        name: "nixpkgs".to_owned(),
        kind: InputKind::Github,
        rev: Some(OLD.to_owned()),
        web_url: Some("https://github.com/NixOS/nixpkgs".to_owned()),
        pin_base: pin_base.map(str::to_owned),
        ..LockedInput::default()
    }
}

fn tarball(kind: &str, rev: Option<&str>, locked_url: Option<&str>) -> LockedInput {
    LockedInput {
        name: "nixpkgs".to_owned(),
        kind: InputKind::from(kind),
        rev: rev.map(str::to_owned),
        web_url: Some("https://github.com/NixOS/nixpkgs".to_owned()),
        locked_url: locked_url.map(str::to_owned),
        ..LockedInput::default()
    }
}

fn reason(check: &UpdateCheck) -> &str {
    match check {
        UpdateCheck::NotCheckable { reason } => reason,
        other @ (UpdateCheck::UpToDate
        | UpdateCheck::Available {
            rev: _,
            compare_url: _,
            lock_url: _,
        }
        | UpdateCheck::Failed { error: _ }) => panic!("expected NotCheckable, got {other:?}"),
    }
}

#[test]
fn an_option_like_input_name_is_not_checkable() {
    let input = LockedInput {
        name: "--commit-lock-file".to_owned(),
        ..github(Some("github:NixOS/nixpkgs"))
    };

    let check = check_update(&Context::system(), &input);

    assert_eq!(reason(&check), "the input name is not a plain identifier");
}

#[test]
fn a_moved_branch_is_an_update_locked_through_the_pin_base() {
    let input = github(Some("github:NixOS/nixpkgs"));

    assert_eq!(
        compare_rev(&input, OLD, Ok(OLD.to_owned())),
        UpdateCheck::UpToDate
    );
    assert_eq!(
        compare_rev(&input, OLD, Ok(NEW.to_owned())),
        UpdateCheck::Available {
            rev: NEW.to_owned(),
            compare_url: Some(format!(
                "https://github.com/NixOS/nixpkgs/compare/{OLD}...{NEW}"
            )),
            lock_url: Some(format!("github:NixOS/nixpkgs/{NEW}")),
        }
    );
    assert_eq!(
        compare_rev(
            &input,
            OLD,
            Err(LockError::Git("repository not found".to_owned()))
        ),
        UpdateCheck::Failed {
            error: "repository not found".to_owned()
        }
    );

    let unpinnable = compare_rev(&github(None), OLD, Ok(NEW.to_owned()));
    assert!(
        matches!(
            unpinnable,
            UpdateCheck::Available {
                rev: _,
                compare_url: _,
                lock_url: None
            }
        ),
        "{unpinnable:?}"
    );
}

#[test]
fn tarball_versions_compare_by_rev_then_by_url() {
    let link = |rev: &str| {
        format!(
            "https://releases.nixos.org/nixos/unstable/nixos-26.11pre1.{rev}/nixexprs.tar.zst?rev={rev}"
        )
    };

    let release = "https://releases.nixos.org/nixos/unstable/nixos-26.11pre2/nixexprs.tar.zst";

    assert_eq!(
        compare_tarball(&tarball("tarball", Some(OLD), None), Ok(Some(link(OLD)))),
        UpdateCheck::UpToDate
    );
    assert_eq!(
        compare_tarball(&tarball("tarball", Some(OLD), None), Ok(Some(link(NEW)))),
        UpdateCheck::Available {
            rev: NEW.to_owned(),
            compare_url: Some(format!(
                "https://github.com/NixOS/nixpkgs/compare/{OLD}...{NEW}"
            )),
            lock_url: Some(link(NEW)),
        }
    );

    let by_url = tarball("tarball", None, Some(&format!("{release}?narHash=x")));
    assert_eq!(
        compare_tarball(&by_url, Ok(Some(release.to_owned()))),
        UpdateCheck::UpToDate
    );
    let elsewhere = tarball("tarball", None, Some("https://example.org/old.tar.gz"));
    assert_eq!(
        compare_tarball(&elsewhere, Ok(Some(release.to_owned()))),
        UpdateCheck::Available {
            rev: "nixos-26.11pre2".to_owned(),
            compare_url: None,
            lock_url: Some(release.to_owned()),
        }
    );
    assert!(
        matches!(
            compare_tarball(
                &tarball("tarball", None, None),
                Ok(Some(release.to_owned()))
            ),
            UpdateCheck::Available {
                rev: _,
                compare_url: _,
                lock_url: _
            }
        ),
        "a tarball without a locked URL can never be current"
    );
}

#[test]
fn http_and_https_tarballs_are_locked_by_their_link() {
    let lock_url = |kind: &str, link: &str| match compare_tarball(
        &tarball(kind, Some(OLD), None),
        Ok(Some(format!("{link}?rev={NEW}"))),
    ) {
        UpdateCheck::Available {
            rev: _,
            compare_url: _,
            lock_url,
        } => lock_url,
        other @ (UpdateCheck::UpToDate
        | UpdateCheck::NotCheckable { reason: _ }
        | UpdateCheck::Failed { error: _ }) => panic!("expected an update, got {other:?}"),
    };

    assert!(lock_url("tarball", "https://example.org/x.tar.gz").is_some());
    assert!(lock_url("tarball", "http://example.org/x.tar.gz").is_some());
    assert_eq!(lock_url("file", "https://example.org/x.tar.gz"), None);
}

#[test]
fn tarball_servers_without_a_link_or_reachable_are_reported() {
    let input = tarball("tarball", Some(OLD), None);

    assert!(matches!(
        compare_tarball(&input, Ok(None)),
        UpdateCheck::NotCheckable { reason: _ }
    ));
    assert_eq!(
        compare_tarball(
            &input,
            Err(LockError::Curl("Could not resolve host".to_owned()))
        ),
        UpdateCheck::Failed {
            error: "Could not resolve host".to_owned()
        }
    );
}

#[test]
fn inputs_without_an_upstream_say_why_without_asking() {
    let cases = [
        ("path", "local path"),
        ("git", "local repository"),
        ("indirect", "flake registry entry"),
        ("mercurial", "source type cannot be checked"),
        ("tarball", "not a web address"),
        ("file", "not a web address"),
    ];

    for (kind, expected) in cases {
        let input = LockedInput {
            name: "input".to_owned(),
            kind: InputKind::from(kind),
            rev: Some(OLD.to_owned()),
            ..LockedInput::default()
        };

        assert_eq!(
            reason(&check_update(&Context::system(), &input)),
            expected,
            "{kind}"
        );
    }

    let unlocked = LockedInput {
        name: "input".to_owned(),
        kind: InputKind::Github,
        remote: Some("https://github.com/o/r.git".to_owned()),
        ..LockedInput::default()
    };

    assert_eq!(
        reason(&check_update(&Context::system(), &unlocked)),
        "source type cannot be checked"
    );

    let follows = LockedInput {
        follows: Some("nixpkgs".to_owned()),
        pinned: true,
        ..github(None)
    };

    assert_eq!(
        reason(&check_update(&Context::system(), &follows)),
        "follows another input"
    );
    let pinned = LockedInput {
        pinned: true,
        remote: Some("https://github.com/o/r.git".to_owned()),
        ..github(None)
    };

    assert_eq!(
        reason(&check_update(&Context::system(), &pinned)),
        "pinned to a fixed revision"
    );
}

#[test]
fn results_keep_the_input_order_across_parallel_batches() {
    let kinds = ["path", "indirect", "git"];
    let inputs: Vec<LockedInput> = (0..MAX_PARALLEL_CHECKS * 2 + 3)
        .map(|index| LockedInput {
            name: format!("input{index}"),
            kind: InputKind::from(kinds[index % kinds.len()]),
            ..LockedInput::default()
        })
        .collect();

    let checks = check_updates(&Context::system(), &inputs);

    assert_eq!(checks.len(), inputs.len());
    for ((input, check), expected) in checks.iter().zip(&inputs) {
        assert_eq!(input.name, expected.name);
        assert_eq!(
            *check,
            check_update(&Context::system(), input),
            "{}",
            input.name
        );
    }
}

#[test]
fn compare_links_need_two_revisions_and_a_known_forge() {
    let input = |kind: &str, web: Option<&str>, rev: Option<&str>| LockedInput {
        kind: InputKind::from(kind),
        web_url: web.map(str::to_owned),
        rev: rev.map(str::to_owned),
        ..LockedInput::default()
    };

    let enterprise = "https://github.example.com/o/r";
    let gitlab = "https://git.example.com/group/r";
    let cases = [
        (
            input("github", Some(enterprise), Some(OLD)),
            NEW,
            Some(format!("{enterprise}/compare/{OLD}...{NEW}")),
        ),
        (
            input("gitlab", Some(gitlab), Some(OLD)),
            NEW,
            Some(format!("{gitlab}/-/compare/{OLD}...{NEW}")),
        ),
        (
            input("sourcehut", Some("https://git.sr.ht/~o/r"), Some(OLD)),
            NEW,
            None,
        ),
        (input("github", Some(enterprise), Some("abc123")), NEW, None),
        (
            input("github", Some(enterprise), Some(OLD)),
            "nixos-26.11pre2",
            None,
        ),
        (input("github", None, Some(OLD)), NEW, None),
        (input("github", Some(enterprise), None), NEW, None),
    ];

    for (locked, new, expected) in cases {
        assert_eq!(compare_url(&locked, new), expected, "{locked:?} -> {new}");
    }
}

#[test]
fn the_update_checks_of_every_input_run_under_the_callers_cancel_token() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);

    let (_, exec) = ScriptedRunner::start(move |call| {
        recorder.lock().unwrap().push(call.is_cancelled());
        Ok(failure("fatal: cancelled"))
    });

    let inputs: Vec<LockedInput> = (0..MAX_PARALLEL_CHECKS + 2)
        .map(|index| LockedInput {
            remote: Some(format!("https://example.invalid/o/r{index}.git")),
            ..github(Some("github:NixOS/nixpkgs"))
        })
        .collect();

    exec.cancel();

    let checks = check_updates(&exec, &inputs);

    assert_eq!(checks.len(), inputs.len());
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), inputs.len());
    assert!(seen.iter().all(|cancelled| *cancelled), "{seen:?}");
}
