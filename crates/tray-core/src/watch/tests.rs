use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use drakeflake_flake::Nix;

use super::{Event, QUIET, RECHECK, Settle, Source, is_skipped, watch};

struct Scripted(std::collections::VecDeque<Result<Event, String>>);

impl Source for Scripted {
    fn wait(&mut self, _timeout: Duration) -> Result<Event, String> {
        self.0.pop_front().unwrap_or(Ok(Event::RootGone))
    }
}

fn scripted(events: Vec<Result<Event, String>>) -> impl Fn(&Path) -> Result<Scripted, String> {
    move |_folder| Ok(Scripted(events.clone().into()))
}

#[test]
fn hidden_folders_and_build_outputs_are_skipped() {
    for name in [".git", ".direnv", "result", "target"] {
        assert!(is_skipped(OsStr::new(name)), "{name}");
    }

    for name in ["modules", "results", "hosts"] {
        assert!(!is_skipped(OsStr::new(name)), "{name}");
    }
}

#[test]
fn a_change_shortens_the_wait_to_the_quiet_time() {
    let mut settle = Settle::new(Nix::system(), None);

    assert_eq!(settle.wait(), RECHECK);
    assert_eq!(settle.pending(), None);

    settle.changed();

    assert!(settle.wait() <= QUIET);
    assert!(settle.pending().is_some());
}

#[test]
fn a_carried_change_that_is_old_enough_waits_no_longer() {
    let long_ago = Instant::now().checked_sub(QUIET + Duration::from_secs(1));
    let settle = Settle::new(Nix::system(), long_ago);

    assert_eq!(settle.wait(), Duration::ZERO);
}

#[test]
fn a_moved_folder_ends_the_watch_with_the_change_waiting() {
    let mut settle = Settle::new(Nix::system(), None);

    watch(
        &mut settle,
        Path::new("/flake"),
        &scripted(vec![Ok(Event::Quiet)]),
    )
    .unwrap();

    assert!(settle.pending().is_some());
}

#[test]
fn a_failing_watch_keeps_the_change_that_was_waiting() {
    let mut settle = Settle::new(Nix::system(), None);

    let ended = watch(
        &mut settle,
        Path::new("/flake"),
        &scripted(vec![Ok(Event::Changed), Err("stream ended".to_owned())]),
    );

    assert_eq!(ended, Err("stream ended".to_owned()));
    assert!(settle.pending().is_some());
}

#[test]
fn a_watch_that_cannot_start_reports_why() {
    let mut settle = Settle::new(Nix::system(), None);

    let ended = watch(&mut settle, Path::new("/flake"), &|_folder: &Path| {
        Err::<Scripted, _>("no inotify".to_owned())
    });

    assert_eq!(ended, Err("no inotify".to_owned()));
    assert_eq!(settle.pending(), None);
}
