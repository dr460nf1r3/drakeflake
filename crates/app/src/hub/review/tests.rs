use std::time::{Duration, Instant};

use super::super::ConsoleMsg;
use super::*;

fn console_lines(hub: &Hub) -> Vec<String> {
    hub.mail
        .console
        .take()
        .into_iter()
        .flat_map(|message| match message {
            ConsoleMsg::Lines(lines) => lines,
            ConsoleMsg::Screen {
                session: _,
                changes: _,
            }
            | ConsoleMsg::Prompt(_) => Vec::new(),
        })
        .collect()
}

#[test]
fn only_activating_modes_can_be_reviewed() {
    let hub = Arc::new(Hub::default());

    for mode in [Mode::Save, Mode::Build] {
        assert_eq!(
            hub.review_apply(ManagedState::default(), mode)
                .map_err(|error| error.to_string()),
            Err("Only switch, boot and test can be reviewed".to_owned()),
            "{mode:?}"
        );
    }

    assert!(!hub.operations().is_running(), "nothing was started");
}

#[test]
fn reviewing_without_a_system_fails_and_ends_the_operation() {
    let hub = Arc::new(Hub::default());

    hub.review_apply(ManagedState::default(), Mode::Switch)
        .unwrap();
    let started = Instant::now();
    while hub.operations().is_running() && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(!hub.operations().is_running());
    assert_eq!(console_lines(&hub), vec!["No system is selected yet."]);
}
