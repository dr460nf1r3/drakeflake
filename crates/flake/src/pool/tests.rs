use super::*;

fn cancelled_nix(priority: Priority) -> Nix {
    let nix = Nix::system().with_priority(priority);
    nix.cancel();
    nix
}

fn with(priority: Priority) -> Nix {
    Nix::system().with_priority(priority)
}

#[test]
fn normal_work_yields_to_waiting_urgent_work() {
    let urgent_waiting = UrgentWaiter::register();
    let limit = eval_limit();
    let take = || eval_slot(&with(Priority::urgent())).unwrap();
    let held: Vec<EvalSlot> = std::iter::repeat_with(take)
        .take(limit.saturating_sub(1))
        .collect();

    let normal = eval_slot(&cancelled_nix(Priority::normal()));
    assert!(
        normal.is_none(),
        "normal work took the last slot urgent work waits for"
    );

    drop(held);
    drop(urgent_waiting);
}

#[test]
fn normal_work_uses_free_slots_despite_waiting_urgent_work() {
    if eval_limit() < 2 {
        return;
    }

    let _waiting = UrgentWaiter::register();
    let slot = eval_slot(&cancelled_nix(Priority::normal()));

    assert!(slot.is_some(), "normal work waited while slots were free");
}

#[test]
fn a_full_pool_makes_everyone_wait_until_a_slot_is_freed() {
    let limit = eval_limit();
    let take = || eval_slot(&with(Priority::urgent())).unwrap();
    let last = take();
    let _held: Vec<EvalSlot> = std::iter::repeat_with(take).take(limit - 1).collect();

    let full = eval_slot(&cancelled_nix(Priority::urgent()));
    assert!(
        full.is_none(),
        "a slot beyond the limit of {limit} was handed out"
    );

    drop(last);
    let freed = eval_slot(&cancelled_nix(Priority::urgent()));
    assert!(freed.is_some(), "a dropped slot was not released");
}

#[test]
fn cancelled_work_does_not_run_in_a_slot() {
    let mut ran = false;
    let result = with_eval_slot(&cancelled_nix(Priority::normal()), || ran = true);

    assert_eq!(result, Err(ExecError::Cancelled(None)));
    assert!(!ran);
    assert_eq!(with_eval_slot(&Nix::system(), || 7_i32).unwrap(), 7_i32);
}

#[test]
fn workers_inherit_the_priority_and_the_waiting_registration() {
    let urgent = with(Priority::urgent());
    let workers = urgent.workers(3);

    assert!(workers.iter().all(|worker| {
        worker.priority().is_urgent() && lock(&worker.priority().pending).is_some()
    }));

    assert!(lock(&urgent.priority().pending).is_none());

    let normal = with(Priority::normal());
    let idle = normal.workers(2);

    assert!(idle.iter().all(|worker| {
        !worker.priority().is_urgent() && lock(&worker.priority().pending).is_none()
    }));
}

#[test]
fn urgent_work_waits_from_the_start_and_gives_it_up_with_its_first_slot() {
    let before = URGENT_WAITING.load(Ordering::SeqCst);
    let urgent = with(Priority::urgent());

    assert!(URGENT_WAITING.load(Ordering::SeqCst) > before.saturating_sub(1));
    assert!(lock(&urgent.priority().pending).is_some());

    drop(eval_slot(&urgent));
    assert!(lock(&urgent.priority().pending).is_none());
    assert!(!with(Priority::normal()).priority().is_urgent());
}

#[test]
fn parallelism_follows_threads_and_memory() {
    const GIB: u64 = 1 << 20;

    assert_eq!(jobs_for(16, Some(64 * GIB)), MAX_EVAL_JOBS);
    assert_eq!(jobs_for(16, Some(12 * GIB)), 2);
    assert_eq!(jobs_for(16, Some(6 * GIB)), 1);
    assert_eq!(jobs_for(4, Some(64 * GIB)), 4);
    assert_eq!(jobs_for(16, Some(1)), 1);
    assert_eq!(jobs_for(0, None), 1);
    assert_eq!(jobs_for(3, None), 3);
    assert!(eval_jobs() >= 1);
}
