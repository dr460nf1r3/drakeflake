use super::*;

#[test]
fn the_first_request_starts_a_worker_and_later_ones_reach_it() {
    let queue = Queue::<u32>::default();

    let receiver = queue.send_or_start(1).expect("no worker yet");
    assert!(queue.send_or_start(2).is_none());
    assert_eq!(receiver.try_iter().collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn a_worker_that_is_gone_is_replaced_without_losing_the_request() {
    let queue = Queue::<u32>::default();

    drop(queue.send_or_start(1));
    let receiver = queue.send_or_start(2).expect("the worker was dropped");
    assert_eq!(receiver.try_iter().collect::<Vec<_>>(), vec![2]);
}

#[test]
fn a_newer_load_cancels_the_evaluations_of_the_older_one() {
    let evaluation = Evaluation::default();
    let first = CancelToken::new();
    let second = CancelToken::new();

    evaluation.replace_load(first.clone());
    evaluation.replace_load(second.clone());

    assert!(first.is_cancelled());
    assert!(!second.is_cancelled());

    evaluation.cancel_load();
    assert!(second.is_cancelled());
}

#[test]
fn a_newer_search_makes_the_older_one_stale() {
    let evaluation = Evaluation::default();

    let old = evaluation.next_search();
    assert!(evaluation.search_is_current(old));

    let new = evaluation.next_search();
    assert!(!evaluation.search_is_current(old));
    assert!(evaluation.search_is_current(new));
}
