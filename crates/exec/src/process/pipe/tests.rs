use std::io::Write as _;

use super::*;

#[test]
fn a_pipe_that_closes_returns_everything() {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all("gr\u{fc}n".as_bytes()).unwrap();
    drop(writer);

    let output = read_pipe(Some(reader)).finish(Duration::from_secs(5), "test", "stdout");

    assert_eq!(output, "gr\u{fc}n");
}

#[test]
fn a_pipe_held_open_keeps_what_was_read() {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all(b"partial").unwrap();

    let collector = read_pipe(Some(reader));
    let output = collector.finish(Duration::from_millis(300), "test", "stdout");

    assert_eq!(output, "partial");
    drop(writer);
}

#[test]
fn invalid_utf8_is_replaced() {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all(&[b'a', 0xff, b'b']).unwrap();
    drop(writer);

    let output = read_pipe(Some(reader)).finish(Duration::from_secs(5), "test", "stdout");

    assert_eq!(output, "a\u{fffd}b");
}
