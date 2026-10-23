use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::*;

#[test]
fn steps_run_in_parallel_and_all_must_succeed() {
    let nix = Nix::system();
    let running = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let step = |ok: bool| {
        let (running, peak) = (&running, &peak);
        move |_: &Nix| {
            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
            running.fetch_sub(1, Ordering::SeqCst);
            ok
        }
    };

    assert!(run(
        &nix,
        vec![
            Step::new("a", Urgency::Urgent, step(true)),
            Step::new("b", Urgency::Normal, step(true))
        ]
    ));
    assert_eq!(peak.load(Ordering::SeqCst), 2, "the steps did not overlap");
    assert!(!run(
        &nix,
        vec![
            Step::new("a", Urgency::Urgent, step(true)),
            Step::new("b", Urgency::Normal, step(false))
        ]
    ));
    assert!(run(&nix, Vec::new()));
}

#[test]
fn steps_keep_the_cancel_token_and_get_their_own_priority() {
    let nix = Nix::system();
    nix.cancel();

    let mut seen = (false, false);
    run(
        &nix,
        vec![Step::new("token", Urgency::Urgent, |step| {
            seen = (step.is_cancelled(), true);
            true
        })],
    );

    assert_eq!(seen, (true, true));
}

#[test]
fn each_step_has_the_priority_it_was_given() {
    let nix = Nix::system().with_priority(Priority::urgent());
    let seen = std::sync::Mutex::new(Vec::new());

    run(
        &nix,
        vec![
            Step::new("urgent", Urgency::Urgent, |step| {
                seen.lock().unwrap().push(("urgent", step.is_urgent()));
                true
            }),
            Step::new("normal", Urgency::Normal, |step| {
                seen.lock().unwrap().push(("normal", step.is_urgent()));
                true
            }),
        ],
    );

    let mut seen = seen.into_inner().unwrap();
    seen.sort_unstable();
    assert_eq!(seen, [("normal", false), ("urgent", true)]);
}

#[test]
fn join_returns_both_results() {
    let nix = Nix::system().with_priority(Priority::urgent());
    nix.cancel();

    let ((cancelled, urgent), here) = join(
        &nix,
        |first| (first.is_cancelled(), first.is_urgent()),
        || 7_i32,
    );

    assert!(cancelled, "the cancel token was not passed on");
    assert!(urgent, "urgency was not passed on");
    assert_eq!(here, 7_i32);
}
