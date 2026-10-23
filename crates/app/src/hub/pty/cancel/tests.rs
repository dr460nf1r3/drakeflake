use super::*;

#[test]
fn escalates_one_stage_at_a_time() {
    let mut cancellation = Cancellation::default();
    let start = Instant::now();

    assert_eq!(cancellation.due(start), Some(Stage::Interrupt));
    assert_eq!(cancellation.due(start), None);
    assert_eq!(cancellation.due(start + TERM_AFTER / 2), None);
    assert_eq!(cancellation.due(start + TERM_AFTER), Some(Stage::Terminate));
    assert_eq!(cancellation.due(start + TERM_AFTER), None);
    assert_eq!(
        cancellation.due(start + TERM_AFTER + KILL_AFTER),
        Some(Stage::Kill)
    );
    assert_eq!(cancellation.due(start + TERM_AFTER + KILL_AFTER * 2), None);
}
