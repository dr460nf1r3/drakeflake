use std::time::{Duration, Instant};

use super::*;

fn update(name: &str, link: Option<&str>) -> InputUpdate {
    InputUpdate {
        name: name.to_owned(),
        locked_at: None,
        link: link.map(str::to_owned),
    }
}

#[test]
fn the_first_update_check_waits_for_the_login_to_settle() {
    let started = Instant::now();
    let settings = drakeflake_config::Tray::default();

    let (next_check, next_health, wake) = schedule(settings, started, None, None, started);

    assert_eq!(next_check, Some(started + FIRST_CHECK));
    assert_eq!(next_health, started, "health is read at once");
    assert_eq!(wake, started);
}

#[test]
fn later_runs_follow_the_configured_intervals() {
    let started = Instant::now();
    let settings = drakeflake_config::Tray {
        check_hours: 2,
        health_minutes: 5,
        ..drakeflake_config::Tray::default()
    };

    let checked = started + Duration::from_mins(3);
    let read = started + Duration::from_mins(4);

    let (next_check, next_health, wake) =
        schedule(settings, started, Some(checked), Some(read), read);

    assert_eq!(next_check, Some(checked + Duration::from_hours(2)));
    assert_eq!(next_health, read + Duration::from_mins(5));

    assert_eq!(wake, read + SETTINGS_EVERY);
}

#[test]
fn no_update_check_is_due_when_checks_are_off() {
    let started = Instant::now();
    let settings = drakeflake_config::Tray {
        updates: false,
        ..drakeflake_config::Tray::default()
    };

    let read = started + Duration::from_secs(10);

    let (next_check, next_health, wake) = schedule(settings, started, None, Some(read), read);

    assert_eq!(next_check, None);
    assert_eq!(next_health, read + Duration::from_mins(1));
    assert_eq!(wake, next_health);
}

#[test]
fn check_once_exits_non_zero_only_on_errors() {
    assert_eq!(
        check_once_result(Ok(Vec::new())),
        (0_i32, "up to date".to_owned())
    );
    assert_eq!(
        check_once_result(Ok(vec![update("nixpkgs", None), update("nur", None)])),
        (0_i32, "updates available: nixpkgs, nur".to_owned())
    );
    assert_eq!(
        check_once_result(Err(CheckError::NotLocal)),
        (
            1_i32,
            "error: the configured flake is not a local directory".to_owned()
        )
    );
}

#[test]
fn only_web_links_from_the_lock_file_are_opened() {
    for (url, opened) in [
        ("https://github.com/NixOS/nixpkgs/compare/a...b", true),
        ("http://example.org", true),
        ("file:///etc/passwd", false),
        ("javascript:alert(1)", false),
        ("/home/user/.bashrc", false),
        (" https://example.org", false),
        ("http:/example.org", false),
        ("", false),
    ] {
        assert_eq!(is_web_link(url), opened, "{url:?}");
    }
}

#[test]
fn the_menu_lists_health_notes_and_inputs_with_their_links() {
    let state = State {
        updates: vec![
            update("nixpkgs", Some("https://cmp")),
            update("local", None),
        ],
        health: Health {
            staged: 1,
            ..Health::default()
        },
        ..State::default()
    };

    let (notes, inputs) = menu_lines(&state);

    assert_eq!(notes, ["1 change staged in DrakeFlake, not applied"]);
    assert_eq!(
        inputs,
        [
            ("nixpkgs".to_owned(), Some("https://cmp".to_owned())),
            ("local".to_owned(), None),
        ]
    );
}
