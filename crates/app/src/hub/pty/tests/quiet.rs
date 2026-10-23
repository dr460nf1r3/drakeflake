use std::sync::Arc;

use super::super::*;
use crate::hub::Hub;
use crate::hub::argv::arg;

#[test]
fn quiet_runs_report_only_how_they_ended() {
    let hub = Arc::new(Hub::default());
    let outcome = Exclusive::detached(&hub).stream_pty_quiet(
        OsStr::new("sh"),
        &[arg("-c"), arg("echo noisy; exit 0")],
        Path::new("/"),
    );

    let mut screens = Vec::new();
    let mut lines = Vec::new();

    for message in hub.mail.console.take() {
        match message {
            ConsoleMsg::Screen { session, changes } => screens.push((session, changes)),
            ConsoleMsg::Lines(shown) => lines.extend(shown),
            ConsoleMsg::Prompt(_) => {}
        }
    }

    assert_eq!(outcome, Outcome::Succeeded);
    assert!(screens.is_empty(), "a quiet run shows no output");
    assert_eq!(lines, ["Finished successfully."]);
}

#[test]
fn quiet_failures_show_what_the_program_wrote() {
    let hub = Arc::new(Hub::default());
    let outcome = Exclusive::detached(&hub).stream_pty_quiet(
        OsStr::new("sh"),
        &[arg("-c"), arg("echo boom; exit 1")],
        Path::new("/"),
    );

    let mut lines = Vec::new();

    for message in hub.mail.console.take() {
        if let ConsoleMsg::Lines(shown) = message {
            lines.extend(shown);
        }
    }

    assert_eq!(outcome, Outcome::Failed);
    assert_eq!(lines, ["boom", "Failed (exit status: 1)."]);
}
