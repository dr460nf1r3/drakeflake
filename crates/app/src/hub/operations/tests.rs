use std::sync::mpsc;
use std::time::Duration;

use super::super::test_support::{console_lines, hub_with, system_until_finished};
use super::*;

const WAIT: Duration = Duration::from_secs(10);

fn run(
    hub: &Arc<Hub>,
    name: &str,
    job: impl FnOnce(&Exclusive) -> Outcome + Send + 'static,
) -> Result<(), HubError> {
    hub.spawn_exclusive(name, Reporting::Shown, job)
}

#[test]
fn a_panicking_job_still_ends_the_operation() {
    let hub = hub_with(None);

    run(&hub, "boom", |_| panic!("job failed hard")).unwrap();
    let messages = system_until_finished(&hub);

    assert!(matches!(
        messages.as_slice(),
        [
            SystemMsg::Running(true),
            SystemMsg::Finished {
                ok: false,
                backup: None,
                commit: None
            }
        ]
    ));
    assert!(
        !hub.operations().is_running(),
        "the app stays busy for good"
    );

    // The report follows the guard's `Finished` (it runs after the unwind).
    let deadline = std::time::Instant::now() + WAIT;
    let mut lines = Vec::new();
    while !lines
        .iter()
        .any(|line: &String| line.starts_with("Internal error in background task \"boom\""))
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the panic is not reported: {lines:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
        lines.extend(console_lines(&hub));
    }

    run(&hub, "next", |_| Outcome::Succeeded).expect("a new operation is refused after the panic");
    system_until_finished(&hub);
}

#[test]
fn only_one_operation_runs_and_its_backup_is_reported() {
    let hub = hub_with(None);
    let (release, wait) = mpsc::channel::<()>();
    let backup = PathBuf::from("/backups/drakeflake.nix.1.bak");
    let expected = backup.clone();

    run(&hub, "first", move |run| {
        wait.recv_timeout(WAIT).unwrap();
        run.record_backup(backup);
        Outcome::Succeeded
    })
    .unwrap();

    assert!(hub.operations().is_running());
    assert!(matches!(
        run(&hub, "second", |_| Outcome::Succeeded),
        Err(HubError::Busy)
    ));

    release.send(()).unwrap();
    let messages = system_until_finished(&hub);
    assert!(
        matches!(
            messages.last(),
            Some(SystemMsg::Finished { ok: true, backup: Some(path), commit: _ }) if *path == expected
        ),
        "the backup of the operation is lost"
    );
    assert!(!hub.operations().is_running());
}

#[test]
fn a_failed_run_keeps_the_backup_it_made() {
    let hub = hub_with(None);

    run(&hub, "failing", |run| {
        run.record_backup(PathBuf::from("/backups/old.bak"));
        Outcome::Failed
    })
    .unwrap();

    assert!(matches!(
        system_until_finished(&hub).last(),
        Some(SystemMsg::Finished {
            ok: false,
            backup: Some(_),
            commit: _,
        })
    ));
}

#[test]
fn cancelling_reaches_the_running_operation_only() {
    let hub = hub_with(None);
    let (seen_send, seen) = mpsc::channel::<bool>();
    let (release, wait) = mpsc::channel::<()>();

    run(&hub, "cancellable", move |run| {
        seen_send.send(run.is_cancelled()).unwrap();
        wait.recv_timeout(WAIT).unwrap();
        let cancelled = run.is_cancelled();
        seen_send.send(cancelled).unwrap();
        if cancelled {
            Outcome::Cancelled
        } else {
            Outcome::Succeeded
        }
    })
    .unwrap();

    assert!(!seen.recv_timeout(WAIT).unwrap());
    hub.cancel_apply();
    release.send(()).unwrap();
    assert!(
        seen.recv_timeout(WAIT).unwrap(),
        "the job never sees the cancel"
    );
    system_until_finished(&hub);
    assert_eq!(console_lines(&hub), vec!["Cancelling…".to_owned()]);
}

#[test]
fn a_cancel_click_after_an_operation_finished_does_not_cancel_the_next_one() {
    let hub = hub_with(None);

    run(&hub, "first", |_| Outcome::Succeeded).unwrap();
    system_until_finished(&hub);

    hub.cancel_apply();
    assert_eq!(
        console_lines(&hub),
        Vec::<String>::new(),
        "nothing ran, nothing is cancelled"
    );

    let (seen_send, seen) = mpsc::channel::<bool>();
    run(&hub, "second", move |run| {
        seen_send.send(run.is_cancelled()).unwrap();
        Outcome::Succeeded
    })
    .unwrap();

    assert!(
        !seen.recv_timeout(WAIT).unwrap(),
        "the old click cancelled the next operation"
    );
    system_until_finished(&hub);
}

#[test]
fn a_new_operation_is_announced_after_the_end_of_the_old_one() {
    let hub = hub_with(None);
    let mut started = 0_usize;
    let mut seen = Vec::new();

    while started < 200 {
        match run(&hub, "again", |_| Outcome::Succeeded) {
            Ok(()) => started += 1,
            Err(HubError::Busy) => std::thread::yield_now(),
            Err(other) => panic!("{other}"),
        }

        seen.extend(hub.mail().system.take());
    }

    let deadline = std::time::Instant::now() + WAIT;
    while hub.operations().is_running() {
        assert!(std::time::Instant::now() < deadline, "never finished");
        std::thread::sleep(Duration::from_millis(1));
    }

    seen.extend(hub.mail().system.take());

    assert_eq!(seen.len(), started * 2);
    for (index, message) in seen.iter().enumerate() {
        let expected_start = index % 2 == 0;
        assert_eq!(
            matches!(message, SystemMsg::Running(true)),
            expected_start,
            "message {index} overtook another"
        );
    }
}

#[test]
fn a_hidden_operation_holds_the_slot_without_telling_the_pages() {
    let hub = hub_with(None);
    let (release, wait) = mpsc::channel::<()>();

    hub.spawn_exclusive("quiet", Reporting::Hidden, move |_| {
        wait.recv_timeout(WAIT).unwrap();
        Outcome::Succeeded
    })
    .unwrap();

    assert!(matches!(
        run(&hub, "other", |_| Outcome::Succeeded),
        Err(HubError::Busy)
    ));

    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + WAIT;
    while hub.operations().is_running() {
        assert!(std::time::Instant::now() < deadline, "never finished");
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(hub.mail().system.take().is_empty());
}
