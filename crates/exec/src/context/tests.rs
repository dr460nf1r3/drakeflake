use std::time::Duration;

use super::*;
use crate::testing::{ScriptedRunner, success};

#[test]
fn a_run_carries_the_contexts_token_and_the_invocation_fields() {
    let (runner, context) = ScriptedRunner::answering(vec![Ok(success("out"))]);

    let invocation = Invocation::new("tool", ["a"], Duration::from_secs(3)).input("in");
    let captured = context.run(invocation).unwrap();

    assert_eq!(captured.stdout, "out");

    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program, "tool");
    assert_eq!(calls[0].args, ["a"]);
    assert_eq!(calls[0].input.as_deref(), Some("in"));
    assert_eq!(calls[0].timeout, Duration::from_secs(3));
    assert!(!calls[0].is_cancelled());

    context.cancel();
    assert!(calls[0].is_cancelled());
}

#[test]
fn a_child_context_is_cancelled_with_its_parent_only() {
    let (_, context) = ScriptedRunner::answering(Vec::new());
    let child = context.child();

    child.cancel();
    assert!(!context.is_cancelled());

    let second = context.child();
    context.cancel();
    assert!(second.is_cancelled());
}

#[test]
fn an_exhausted_script_fails_the_call() {
    let (_, context) = ScriptedRunner::answering(Vec::new());

    let error = context
        .command("tool", Vec::<&str>::new(), Duration::from_secs(1))
        .unwrap_err();

    assert!(error.to_string().contains("no scripted reply"), "{error}");
}
