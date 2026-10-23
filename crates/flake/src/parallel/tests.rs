use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::par_map;

#[test]
fn results_keep_the_item_order() {
    let squares = par_map(vec![3_u32, 1, 2, 5], 2, |_, number| number * number, |_| 0);

    assert_eq!(squares, [9, 1, 4, 25]);
}

#[test]
fn no_more_threads_than_asked_run_at_once() {
    let running = AtomicUsize::new(0);
    let most = AtomicUsize::new(0);

    let results = par_map(
        (0..12).collect::<Vec<_>>(),
        3,
        |_, number: u32| {
            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
            most.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(5));
            running.fetch_sub(1, Ordering::SeqCst);
            number
        },
        |_| u32::MAX,
    );

    assert_eq!(results.len(), 12);
    assert!(most.load(Ordering::SeqCst) <= 3);
}

#[test]
fn a_slow_item_does_not_hold_back_the_rest() {
    let started = AtomicUsize::new(0);

    let results = par_map(
        vec![true, false, false, false, false],
        2,
        |_, slow| {
            if slow {
                while started.load(Ordering::SeqCst) < 4 {
                    std::thread::sleep(Duration::from_millis(1));
                }
            } else {
                started.fetch_add(1, Ordering::SeqCst);
            }

            slow
        },
        |_| false,
    );

    assert_eq!(results, [true, false, false, false, false]);
}

#[test]
fn a_panicking_item_is_replaced_and_the_others_finish() {
    let results = par_map(
        vec![1_u32, 2, 3],
        2,
        |_, number| {
            assert_ne!(number, 2, "boom");
            number
        },
        |_| 0,
    );

    assert_eq!(results, [1, 0, 3]);
}

#[test]
fn nothing_to_do_returns_nothing() {
    assert_eq!(
        par_map(Vec::<u32>::new(), 4, |_, number| number, |_| 0),
        Vec::<u32>::new()
    );
}
