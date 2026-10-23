use std::sync::Arc;

use super::super::*;
use crate::hub::Hub;
use crate::hub::test_support::console_lines;

#[test]
fn exits_are_reported_as_success_failure_or_cancel() {
    use std::os::unix::process::ExitStatusExt as _;

    let success = ExitStatus::from_raw(0);
    let failure = ExitStatus::from_raw(1 << 8);

    for (status, cancelled, outcome, line) in [
        (
            Some(success),
            false,
            Outcome::Succeeded,
            Some("Finished successfully."),
        ),
        (
            Some(success),
            true,
            Outcome::Succeeded,
            Some("Finished successfully."),
        ),
        (
            Some(failure),
            false,
            Outcome::Failed,
            Some("Failed (exit status: 1)."),
        ),
        (
            Some(failure),
            true,
            Outcome::Cancelled,
            Some("Cancelled (exit status: 1)."),
        ),
        (None, false, Outcome::Failed, None),
        (None, true, Outcome::Cancelled, Some("Cancelled.")),
    ] {
        let hub = Arc::new(Hub::default());
        let run = Exclusive::detached(&hub);
        if cancelled {
            run.request_cancel();
        }

        assert_eq!(run.report_exit(status), outcome, "{status:?} {cancelled}");
        assert_eq!(
            console_lines(&hub),
            line.map(str::to_owned).into_iter().collect::<Vec<_>>(),
            "{status:?} {cancelled}"
        );
    }
}
