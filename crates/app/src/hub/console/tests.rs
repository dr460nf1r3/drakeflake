use std::sync::Arc;

use super::*;

fn console() -> (Console, Arc<Mailboxes>) {
    let mail = Arc::new(Mailboxes::default());
    (Console::new(Arc::clone(&mail)), mail)
}

fn posted(mail: &Mailboxes) -> Vec<Vec<String>> {
    mail.system
        .take()
        .into_iter()
        .filter_map(|message| {
            if let SystemMsg::Activities(list) = message {
                Some(list)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn guard_clears_on_drop() {
    let (console, mail) = console();
    {
        let activity = console.activity("test", "Working…");
        assert_eq!(posted(&mail).last(), Some(&vec!["Working…".to_owned()]));
        activity.update("Still working…".to_owned());
        assert_eq!(
            posted(&mail).last(),
            Some(&vec!["Still working…".to_owned()])
        );
    }

    assert_eq!(posted(&mail).last(), Some(&Vec::new()));
}

#[test]
fn guard_clears_on_early_return() {
    fn work(console: &Console, fail: bool) -> Result<(), String> {
        let _activity = console.activity("test", "Working…");
        if fail {
            return Err("failed".to_owned());
        }

        Ok(())
    }

    let (console, mail) = console();
    assert!(work(&console, true).is_err());
    assert_eq!(posted(&mail).last(), Some(&Vec::new()));
}

#[test]
fn a_guard_only_removes_its_own_entry() {
    let (console, mail) = console();

    let first = console.activity("lock", "Checking inputs…");
    let second = console.activity("lock", "Checking inputs again…");
    drop(first);

    assert_eq!(
        posted(&mail).last(),
        Some(&vec!["Checking inputs again…".to_owned()]),
        "the second job lost its entry to the first one's guard"
    );

    drop(second);
    assert_eq!(posted(&mail).last(), Some(&Vec::new()));
}

#[test]
fn a_replaced_loads_guards_remove_nothing_of_the_next_one() {
    let (console, mail) = console();

    let old = console.activity("target", "Old load…");
    console.clear_activities();
    let new = console.activity("target", "New load…");
    drop(old);

    assert_eq!(posted(&mail).last(), Some(&vec!["New load…".to_owned()]));
    drop(new);
}

#[test]
fn snapshots_arrive_in_the_order_they_were_made() {
    let (console, mail) = console();
    let console = Arc::new(console);

    let mut threads = Vec::new();
    for _ in 0_u8..4 {
        let console = Arc::clone(&console);
        threads.push(std::thread::spawn(move || {
            for _ in 0_u16..200 {
                let _activity = console.activity("work", "Working…");
            }
        }));
    }

    for thread in threads {
        thread.join().unwrap();
    }

    let snapshots = posted(&mail);
    assert_eq!(snapshots.len(), 4 * 200 * 2);
    assert_eq!(snapshots.first().map(Vec::len), Some(1));
    assert_eq!(snapshots.last(), Some(&Vec::new()));

    for pair in snapshots.windows(2) {
        let [before, after] = pair else {
            panic!("windows of two");
        };

        assert_eq!(
            before.len().abs_diff(after.len()),
            1,
            "snapshots were posted out of order"
        );
    }
}

#[test]
fn skip_list_is_kept_and_progress_counted() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    cache.write_list("skip.json", &["old".to_owned()]);
    let (console, mail) = console();

    let (result, skip) = console.evaluate_skipping(
        &cache,
        "skip.json",
        Some(Progress {
            key: "test",
            label: "Evaluating…",
            skipped: "broken",
        }),
        |skip, on_skip| {
            skip.push("new".to_owned());
            on_skip("new", skip.len());
            assert_eq!(
                posted(&mail).last(),
                Some(&vec!["Evaluating… (skipping 2 broken)".to_owned()])
            );
            42_u32
        },
    );

    assert_eq!(result, 42_u32);
    assert_eq!(skip, vec!["old".to_owned(), "new".to_owned()]);
    assert_eq!(cache.read_list("skip.json"), skip);
    assert_eq!(posted(&mail).last(), Some(&Vec::new()));
}

#[test]
fn an_evaluation_without_progress_shows_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::at(dir.path().to_path_buf());

    let (console, mail) = console();
    let (result, skip) = console.evaluate_skipping(&cache, "skip.json", None, |_, _| 7_u32);

    assert_eq!(result, 7_u32);
    assert_eq!(skip, Vec::<String>::new());
    assert_eq!(mail.system.take().len(), 0);
}
