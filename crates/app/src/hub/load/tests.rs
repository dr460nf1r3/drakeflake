use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use drakeflake_exec::Captured;
use drakeflake_exec::testing::{Call, ScriptedRunner, failure, success};
use drakeflake_flake::{FlakeError, SystemKind};
use drakeflake_plan::ManagedState;

use super::*;
use crate::hub::{Generation, Mode, SystemMsg};

fn target(flake: &str, host: &str) -> Target {
    Target {
        flake: flake.to_owned(),
        host: host.into(),
        kind: SystemKind::NixOs,
    }
}

#[test]
fn staged_edits_survive_only_a_reload_of_the_same_system() {
    let current = target("/etc/nixos", "box");
    let cases = [
        (Some(&current), "/etc/nixos", None, true),
        (Some(&current), " /etc/nixos ", Some("box"), true),
        (Some(&current), "/etc/nixos", Some("other"), false),
        (Some(&current), "/srv/flake", None, false),
        (None, "/etc/nixos", Some("box"), false),
    ];

    for (loaded, flake, host, same) in cases {
        assert_eq!(
            is_same_system(loaded, flake, host),
            same,
            "{flake} {host:?}"
        );
    }
}

#[test]
fn parsed_json_keeps_its_text_and_errors_pass_through() {
    let parse = |text: &str| text.parse::<u32>();

    assert_eq!(
        parse_json(Ok::<_, EvaluateError>("7".to_owned()), parse).unwrap(),
        ("7".to_owned(), 7)
    );

    let failed = parse_json(
        Err::<String, _>(FlakeError::Nix("eval failed".to_owned())),
        parse,
    )
    .unwrap_err();

    assert_eq!(failed.to_string(), "eval failed");
    assert!(matches!(failed, EvaluateError::Flake(_)));

    let unparsable = parse_json(Ok::<_, EvaluateError>("seven".to_owned()), parse).unwrap_err();
    assert!(matches!(unparsable, EvaluateError::Parse(_)));
}

#[test]
fn the_cache_is_shown_and_reports_whether_it_had_an_index() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    let hub = Hub::default();
    let nix = hub.nix();
    let target = Target::default();
    let loader = Loader::new(hub.services(), &nix, Generation::default(), &target);

    assert!(!loader.load_cached(&cache));
    assert_eq!(
        hub.mail.options.take().len() + hub.mail.packages.take().len(),
        0
    );

    fs::write(dir.path().join(INSTALLED_FILE), r#"["hello"]"#).unwrap();
    fs::write(dir.path().join(OPTIONS_FILE), "{").unwrap();
    fs::write(dir.path().join(VALUES_FILE), "{}").unwrap();
    assert!(
        !loader.load_cached(&cache),
        "a corrupt index counts as shown"
    );
    assert!(matches!(
        hub.mail.options.take().as_slice(),
        [OptionsMsg::Snapshot(_)]
    ));
    assert!(matches!(
        hub.mail.packages.take().as_slice(),
        [PackagesMsg::Installed(names)] if names.contains("hello")
    ));

    fs::write(dir.path().join(OPTIONS_FILE), "{}").unwrap();
    fs::write(dir.path().join(PACKAGES_FILE), "{}").unwrap();
    assert!(loader.load_cached(&cache));
    assert!(matches!(
        hub.mail.options.take().as_slice(),
        [OptionsMsg::Index(_), OptionsMsg::Snapshot(_)]
    ));
    assert!(matches!(
        hub.mail.packages.take().as_slice(),
        [PackagesMsg::Installed(_), PackagesMsg::Index(_)]
    ));
}

fn flake_with_broken_packages(call: &Call) -> Captured {
    let apply = call.arg_after("--apply").unwrap_or_default();

    if call.has_arg("metadata") {
        return success(r#"{"locked":{"narHash":"sha256-test"}}"#);
    }

    if apply.contains("systemPackages") {
        return success(r#"["hello"]"#);
    }

    if apply.contains("p.meta.description") {
        return failure("error: packages exploded");
    }

    if call.program == "nix" && call.has_arg("eval") {
        let output = if apply.contains("home-manager") {
            "null"
        } else {
            "{}"
        };

        return success(output);
    }

    failure("error: not scripted")
}

#[test]
fn a_load_whose_package_step_fails_keeps_the_results_of_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().join("cache"));
    let (runner, exec) = ScriptedRunner::start(|call| Ok(flake_with_broken_packages(call)));
    let hub = Arc::new(Hub::with_context(exec));
    let target = Target {
        flake: dir.path().to_string_lossy().into_owned(),
        host: "box".into(),
        kind: SystemKind::NixOs,
    };

    let nix = hub.nix();
    Loader::new(hub.services(), &nix, Generation::default(), &target)
        .load_data(&cache, CacheUse::Rebuild);

    let packages = hub.mail.packages.take();
    assert!(
        packages
            .iter()
            .any(|message| matches!(message, PackagesMsg::IndexError(error) if error.contains("packages exploded"))),
        "the failed step was not reported"
    );
    assert!(
        packages.iter().any(
            |message| matches!(message, PackagesMsg::Installed(names) if names.contains("hello"))
        ),
        "the installed packages were dropped"
    );

    let options = hub.mail.options.take();
    assert!(
        options
            .iter()
            .any(|message| matches!(message, OptionsMsg::Index(_)))
    );
    assert!(
        options
            .iter()
            .any(|message| matches!(message, OptionsMsg::Snapshot(_)))
    );
    assert!(
        !options.iter().any(|message| matches!(
            message,
            OptionsMsg::IndexError(_) | OptionsMsg::SnapshotError(_)
        )),
        "a step that worked was reported as failed"
    );

    assert_eq!(cache.read_list(INSTALLED_FILE), ["hello"]);
    assert_eq!(
        cache.read(drakeflake_evaluate::cache::FINGERPRINT_FILE),
        None,
        "an incomplete load marked the cache current"
    );
    assert!(!nix.is_cancelled(), "a failed step cancelled the load");
    assert!(!runner.calls().is_empty());
}

/// A flake without a repository: Nix refuses its files until `git init` ran and staged them.
fn flake_without_a_repository(initialized: &Arc<AtomicBool>, call: &Call) -> Captured {
    let ready = initialized.load(Ordering::SeqCst);

    match call.program.to_str().unwrap_or_default() {
        "git" if call.has_arg("rev-parse") && !ready => {
            failure("fatal: not a git repository (or any of the parent directories): .git")
        }
        "git" if call.has_arg("rev-parse") => success("true\n"),
        "git" if call.has_arg("init") => {
            initialized.store(true, Ordering::SeqCst);
            success("")
        }
        "git" | "nix-instantiate" => success(""),
        "nix" if call.has_arg("metadata") => success(r#"{"locked":{"narHash":"sha256-test"}}"#),
        "nix" => nix_answer(call, ready),
        _ => failure("error: not scripted"),
    }
}

fn nix_answer(call: &Call, ready: bool) -> Captured {
    let apply = call.arg_after("--apply").unwrap_or_default();

    if apply == "builtins.attrNames" {
        let hosts = if call
            .args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("nixosConfigurations"))
        {
            r#"["box"]"#
        } else {
            "[]"
        };

        return success(hosts);
    }

    if apply.contains("systemPackages") {
        return success(r#"["hello"]"#);
    }

    if apply.contains("p.meta.description") && !ready {
        return failure(
            "error: Path 'flake.nix' in the Git repository \"/flake\" is not tracked by Git.",
        );
    }

    if apply.contains("p.meta.description") {
        return success(r#"{"hello":{"v":"1","d":"A program"}}"#);
    }

    success(if apply.contains("home-manager") {
        "null"
    } else {
        "{}"
    })
}

fn packages_until(hub: &Hub, done: impl Fn(&PackagesMsg) -> bool) -> Vec<PackagesMsg> {
    let deadline = Instant::now() + std::time::Duration::from_secs(30);
    let mut seen = Vec::new();

    while Instant::now() < deadline {
        seen.extend(hub.mail.packages.take());

        if seen.iter().any(&done) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    seen
}

fn is_index(message: &PackagesMsg) -> bool {
    matches!(message, PackagesMsg::Index(_))
}

fn is_untracked_error(message: &PackagesMsg) -> bool {
    matches!(message, PackagesMsg::IndexError(error) if error.contains("not tracked by Git"))
}

#[test]
fn a_repository_created_by_a_save_restarts_the_load_that_failed_without_it() {
    let flake = tempfile::tempdir().unwrap();
    let initialized = Arc::new(AtomicBool::new(false));
    let scripted = Arc::clone(&initialized);
    let (runner, exec) =
        ScriptedRunner::start(move |call| Ok(flake_without_a_repository(&scripted, call)));

    let hub = Arc::new(Hub::with_context(exec));
    let target = Target {
        flake: flake.path().to_string_lossy().into_owned(),
        host: "box".into(),
        kind: SystemKind::NixOs,
    };

    hub.session().update(|state| {
        state.target = Some(target.clone());
        state.auto_commit = true;
    });

    hub.restart_load();
    let before = packages_until(&hub, is_untracked_error);
    assert!(
        before.iter().any(is_untracked_error),
        "the first load did not fail: {:?}",
        runner.calls().len()
    );
    assert!(!before.iter().any(is_index));

    hub.apply(ManagedState::default(), Mode::Save).unwrap();
    let finished = crate::hub::test_support::system_until_finished(&hub);
    assert!(finished.iter().any(|message| matches!(
        message,
        SystemMsg::Finished {
            ok: true,
            backup: _,
            commit: _,
        }
    )));
    assert!(
        initialized.load(Ordering::SeqCst),
        "no repository was created"
    );

    let after = packages_until(&hub, is_index);
    assert!(
        after.iter().any(is_index),
        "the package index never recovered after {} messages",
        after.len()
    );

    Cache::for_target(&target).clear();
    Cache::for_flake(&target.flake).clear();
}

#[test]
fn a_save_in_an_existing_repository_leaves_the_load_alone() {
    let flake = tempfile::tempdir().unwrap();
    let initialized = Arc::new(AtomicBool::new(true));
    let scripted = Arc::clone(&initialized);
    let (runner, exec) =
        ScriptedRunner::start(move |call| Ok(flake_without_a_repository(&scripted, call)));

    let hub = Arc::new(Hub::with_context(exec));
    let target = Target {
        flake: flake.path().to_string_lossy().into_owned(),
        host: "box".into(),
        kind: SystemKind::NixOs,
    };

    hub.session().update(|state| {
        state.target = Some(target.clone());
        state.auto_commit = true;
    });

    hub.apply(ManagedState::default(), Mode::Save).unwrap();
    crate::hub::test_support::system_until_finished(&hub);

    let generation = hub.session().generation();
    assert_eq!(
        generation,
        Generation::default(),
        "a load was started although the repository existed"
    );
    assert!(
        !runner
            .calls()
            .iter()
            .any(|call| call.program == "git" && call.has_arg("init")),
        "git init ran in a repository"
    );

    Cache::for_target(&target).clear();
}
