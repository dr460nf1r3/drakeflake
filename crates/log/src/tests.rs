use std::fs;
use std::time::{Duration, UNIX_EPOCH};

use crate::file::rotate;
use crate::format::{Style, format_record, timestamp};
use crate::{LEVELS, LevelFilter, parse_level};

#[test]
fn levels_parse() {
    assert_eq!(parse_level("Debug"), Some(LevelFilter::Debug));
    assert_eq!(parse_level("warning"), Some(LevelFilter::Warn));
    assert_eq!(parse_level("loud"), None);
    assert_eq!(parse_level(" OFF\n"), Some(LevelFilter::Off));
    assert_eq!(parse_level(""), None);
    assert!(LEVELS.iter().all(|name| parse_level(name).is_some()));
}

#[test]
fn timestamps_are_utc_iso() {
    let time = UNIX_EPOCH + Duration::from_millis(1_791_106_229_123);
    assert_eq!(timestamp(time), "2026-10-04T09:30:29.123Z");
    assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
}

#[test]
fn timestamps_get_leap_days_and_century_years_right() {
    for (seconds, expected) in [
        (1_709_251_199, "2024-02-29T23:59:59.000Z"),
        (951_825_600, "2000-02-29T12:00:00.000Z"),
        (4_107_542_400, "2100-03-01T00:00:00.000Z"),
        (946_684_799, "1999-12-31T23:59:59.000Z"),
    ] {
        assert_eq!(
            timestamp(UNIX_EPOCH + Duration::from_secs(seconds)),
            expected
        );
    }
}

#[test]
fn terminal_lines_are_coloured_by_severity() {
    let line = format_record(UNIX_EPOCH, log::Level::Error, "hub", "boom", Style::Color);

    assert_eq!(
        line,
        "\x1b[2m1970-01-01T00:00:00.000Z\x1b[0m \x1b[1;31mERROR\x1b[0m \x1b[36mhub\x1b[0m: boom\n"
    );
    assert!(
        format_record(UNIX_EPOCH, log::Level::Info, "x", "y", Style::Plain)
            .chars()
            .all(|ch| ch != '\x1b')
    );
}

#[test]
fn multi_line_messages_are_indented() {
    let line = format_record(
        UNIX_EPOCH,
        log::Level::Warn,
        "nix",
        "failed\nerror: x\n",
        Style::Plain,
    );

    assert_eq!(
        line,
        "1970-01-01T00:00:00.000Z WARN  nix: failed\n    error: x\n"
    );
}

#[test]
fn rotation_keeps_a_bounded_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");

    for round in 0..5_u32 {
        fs::write(&path, format!("{round}")).unwrap();
        rotate(&path).unwrap();
    }

    let read = |name: &str| fs::read_to_string(dir.path().join(name)).unwrap_or_default();
    assert_eq!(read("app.log.1"), "4");
    assert_eq!(read("app.log.3"), "2");
    assert!(!dir.path().join("app.log.4").exists());
    assert!(!path.exists());
}
