use std::sync::atomic::AtomicBool;
use std::time::Duration;

use drakeflake_exec::testing::{Call, ScriptedRunner, failure, success};

use super::*;

#[test]
fn failing_attribute_parsing() {
    let lix = "error:\n       … while evaluating attribute 'hardware.nvidia.open'\n       … while evaluating attribute 'inner'";
    assert_eq!(failing_attribute(lix), Some("hardware.nvidia.open".into()));

    let cpp = "… while evaluating the attribute 'services.tor.settings'";
    assert_eq!(failing_attribute(cpp), Some("services.tor.settings".into()));

    let quoted = "… while evaluating attribute '\"hardware.nvidia.gsp.enable\"'\n  … x";
    assert_eq!(
        failing_attribute(quoted),
        Some("hardware.nvidia.gsp.enable".into())
    );

    let nested =
        "… while evaluating attribute '\"boot.kernel.sysctl.\\\"net.ipv4.ip_forward\\\"\"'\n";

    assert_eq!(
        failing_attribute(nested),
        Some("boot.kernel.sysctl.\"net.ipv4.ip_forward\"".into())
    );

    let chain = "… while evaluating the attribute 'services'\n\
                 … while evaluating the attribute 'value'\n\
                 … while evaluating the attribute '\"services.tailscale\"'\n\
                 … while evaluating the attribute '\"services.tailscale.description\"'\n\
                 … while evaluating the attribute 'options'\n";

    assert_eq!(
        failing_attribute(chain),
        Some("services.tailscale.description".into())
    );

    let flat =
        "… while evaluating the attribute 'colmena'\n… while evaluating the attribute 'meta'\n";

    assert_eq!(failing_attribute(flat), Some("colmena".into()));

    assert_eq!(failing_attribute("error: something"), None);
    assert_eq!(
        failing_attribute("while evaluating attribute 'unterminated"),
        None
    );
}

#[test]
fn degrading_skips() {
    assert_eq!(next_skip(&[], "a.b", true), Some("~a.b".into()));
    assert_eq!(next_skip(&["~a.b".into()], "a.b", true), Some("a.b".into()));
    assert_eq!(next_skip(&["~a.b".into(), "a.b".into()], "a.b", true), None);
    assert_eq!(next_skip(&[], "a.b", false), Some("a.b".into()));
    assert_eq!(next_skip(&["a.b".into()], "a.b", false), None);
}

#[test]
fn shard_outputs_merge() {
    let walker = vec![
        r#"{"services":{"services.a":{"_drakeflakeOption":"services.a","v":1}}}"#.to_owned(),
        r#"{"services":{"services.b":{"_drakeflakeOption":"services.b","v":2}}}"#.to_owned(),
    ];

    let merged = merge_shards(&walker, true).unwrap();
    assert_eq!(merged["services.a"]["v"], 1_i32);
    assert_eq!(merged["services.b"]["v"], 2_i32);

    let flat = vec![r#"{"a":{"v":"1"}}"#.to_owned(), r#"{"b":null}"#.to_owned()];
    let packages = merge_shards(&flat, false).unwrap();
    assert!(packages.get("a").is_some() && packages.get("b").is_some());

    merge_shards(&["not json".to_owned()], false).unwrap_err();
}

fn job() -> EvalJob<'static> {
    EvalJob {
        what: "test",
        installable: "nixpkgs#hello",
        cwd: None,
        template: "_: _: { }",
        degrade: false,
        walker: false,
        split: 1,
    }
}

fn wait_until_cancelled(nix: &Nix) {
    while !nix.is_cancelled() {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_cancelled_shard_reports_cancelled() {
    let nix = Nix::system();
    nix.cancel();

    let error =
        eval_with_retries(&nix, job(), &Shard::single(), &mut Vec::new(), &|_| {}).unwrap_err();

    assert!(error.is_cancelled());
    assert_eq!(error.to_string(), "cancelled");
}

#[test]
fn a_failing_shard_stops_its_siblings_and_its_error_wins() {
    let started = Instant::now();
    let workers = Nix::system().child().workers(3);

    let results = run_shards(workers, "test", &|index, worker| {
        if index == 1 {
            return Err(FlakeError::Nix("boom".to_owned()));
        }

        wait_until_cancelled(worker);
        Err(ExecError::Cancelled(None).into())
    });

    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(matches!(
        first_failure(results),
        Err(FlakeError::Nix(message)) if message == "boom"
    ));
}

#[test]
fn the_parent_cancels_every_shard() {
    let parent = Nix::system();
    let workers = parent.child().workers(2);

    parent.cancel();

    let results = run_shards(workers, "test", &|_, worker| {
        wait_until_cancelled(worker);
        Err(ExecError::Cancelled(None).into())
    });

    assert!(first_failure(results).unwrap_err().is_cancelled());
}

#[test]
fn a_shard_that_fails_cancels_the_others_but_not_the_caller() {
    let first = AtomicBool::new(true);

    let (runner, exec) = ScriptedRunner::start(move |call| {
        if first.swap(false, Ordering::SeqCst) {
            return Ok(failure("error: boom"));
        }

        for _ in 0_u32..2_000 {
            if call.is_cancelled() {
                return Err(ExecError::Cancelled(None));
            }

            std::thread::sleep(Duration::from_millis(5));
        }

        Err(ExecError::Failed("never cancelled".to_owned()))
    });

    let nix = Nix::new(exec);
    let job = EvalJob { split: 2, ..job() };

    let error = eval_merged(&nix, job, &mut Vec::new(), &|_, _| {}).unwrap_err();

    assert!(
        matches!(&error, FlakeError::Nix(message) if message == "error: boom"),
        "{error}"
    );
    assert!(!nix.is_cancelled(), "the failure cancelled the caller");

    let calls = runner.calls();
    assert!(!calls.is_empty());
    assert!(calls.iter().skip(1).all(Call::is_cancelled));
}

#[test]
fn a_shard_names_its_slice_and_skip_list_in_the_expression() {
    let (runner, exec) = ScriptedRunner::answering(vec![Ok(success("{}"))]);
    let nix = Nix::new(exec);
    let shard = Shard { index: 1, count: 4 };
    let mut skip = vec!["a.b".to_owned()];

    let output = eval_with_retries(&nix, job(), &shard, &mut skip, &|_| {}).unwrap();
    let calls = runner.calls();

    assert_eq!(output, "{}");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program, "nix");
    assert_eq!(calls[0].timeout, LONG_TIMEOUT);

    let apply = calls[0].arg_after("--apply").unwrap();
    assert!(apply.contains(r#"\"index\":1,\"count\":4"#), "{apply}");
    assert!(apply.contains("a.b"), "{apply}");
}

#[test]
fn a_failing_entry_is_skipped_and_the_shard_retried() {
    let stderr = "error: x\n… while evaluating attribute 'services.broken'";
    let (runner, exec) =
        ScriptedRunner::answering(vec![Ok(failure(stderr)), Ok(success(r#"{"ok":1}"#))]);

    let nix = Nix::new(exec);
    let mut skip = Vec::new();
    let mut seen = Vec::new();
    let seen_cell = Mutex::new(&mut seen);

    let output = eval_with_retries(&nix, job(), &Shard::single(), &mut skip, &|entry| {
        lock(&seen_cell).push(entry.to_owned());
    })
    .unwrap();

    assert_eq!(output, r#"{"ok":1}"#);
    assert_eq!(skip, ["services.broken"]);
    assert_eq!(runner.calls().len(), 2);
}
