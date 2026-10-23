use std::fs::File;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::fd::{AsFd as _, OwnedFd};

use log::Level;

const MARK: char = '\u{1}';

#[must_use = "the pattern only takes effect once set in the environment"]
pub fn qt_message_pattern() -> String {
    format!("{MARK}%{{type}}{MARK}%{{if-category}}%{{category}}: %{{endif}}%{{message}}")
}

pub fn capture() {
    let original = match std::io::stderr().as_fd().try_clone_to_owned() {
        Ok(original) => original,
        Err(error) => {
            log::warn!("stderr not captured: cannot duplicate it: {error}");
            return;
        }
    };

    let (reader, writer) = match rustix::pipe::pipe() {
        Ok(pipe) => pipe,
        Err(error) => {
            log::warn!("stderr not captured: no pipe: {error}");
            return;
        }
    };

    // Echoing to the redirected stderr would feed log records back into this pipe forever.
    match original.try_clone() {
        Ok(terminal) => drakeflake_log::set_terminal(File::from(terminal)),
        Err(error) => {
            log::warn!("stderr not captured: cannot duplicate it for the log echo: {error}");
            return;
        }
    }

    if let Err(error) = rustix::stdio::dup2_stderr(&writer) {
        log::warn!("stderr not captured: cannot redirect it: {error}");
        return;
    }

    drop(writer);

    let spawned = std::thread::Builder::new()
        .name("df-stderr".to_owned())
        .spawn(move || copy_lines(reader, original));

    if let Err(error) = spawned {
        // Nobody would drain the pipe: writers would block once it is full.
        log::warn!("stderr not captured: no copying thread ({error}); discarding it");
        restore_unowned();
    }
}

/// A pipe nobody reads would block writers, so stderr goes to /dev/null instead.
fn restore_unowned() {
    let restored = File::options()
        .write(true)
        .open("/dev/null")
        .and_then(|null| rustix::stdio::dup2_stderr(&null).map_err(std::io::Error::from));

    if let Err(error) = restored {
        log::warn!("stderr is a pipe nobody reads ({error}); writes to it may block");
    }
}

fn copy_lines(reader: OwnedFd, original: OwnedFd) {
    let mut terminal = File::from(original);

    let mut input = BufReader::new(File::from(reader));
    let mut bytes = Vec::new();

    loop {
        bytes.clear();
        match input.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                log::warn!("stderr copy stopped: {error}");
                break;
            }
        }

        let line = String::from_utf8_lossy(&bytes);
        let (level, text) = line_level(&line);

        if is_known_noise(text) {
            log::debug!(target: drakeflake_log::STDERR_TARGET, "{text}");
            continue;
        }

        writeln!(terminal, "{text}").ok();
        log::log!(target: drakeflake_log::STDERR_TARGET, level, "{text}");
    }

    if let Err(error) = rustix::stdio::dup2_stderr(&terminal) {
        log::warn!("cannot give stderr back to the terminal: {error}");
    }
}

fn line_level(line: &str) -> (Level, &str) {
    let (tagged, text) = split_tag(line.trim_end_matches(['\n', '\r']));

    (tagged.unwrap_or(Level::Info), text)
}

/// Harmless warnings from Kirigami 6.30 (relative URLs) and Plasma's theme plugin (unknown settings-change types).
fn is_known_noise(text: &str) -> bool {
    text == "qmlRegisterType requires absolute URLs."
        || text.starts_with("Unknown type of change in KGlobalSettings::slotNotifyChange:")
}

fn split_tag(line: &str) -> (Option<Level>, &str) {
    let Some(rest) = line.strip_prefix(MARK) else {
        return (None, line);
    };

    let Some((kind, text)) = rest.split_once(MARK) else {
        return (None, line);
    };

    let level = match kind {
        "debug" => Level::Debug,
        "info" => Level::Info,
        "warning" => Level::Warn,
        _ => Level::Error,
    };

    (Some(level), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qt_tags_become_levels() {
        assert_eq!(
            split_tag("\u{1}warning\u{1}qml: Binding loop"),
            (Some(Level::Warn), "qml: Binding loop")
        );
        assert_eq!(split_tag("\u{1}critical\u{1}x"), (Some(Level::Error), "x"));
        assert_eq!(split_tag("plain text"), (None, "plain text"));
    }

    #[test]
    fn lines_lose_their_ending_and_untagged_ones_are_info() {
        let cases = [
            ("\u{1}warning\u{1}qml: x\r\n", Level::Warn, "qml: x"),
            ("\u{1}debug\u{1}y\n", Level::Debug, "y"),
            ("child output\n", Level::Info, "child output"),
            ("\u{1}warning half\n", Level::Info, "\u{1}warning half"),
            ("\n", Level::Info, ""),
        ];

        for (line, level, text) in cases {
            assert_eq!(line_level(line), (level, text), "{line:?}");
        }
    }

    #[test]
    fn only_exact_known_noise_is_hidden() {
        assert!(is_known_noise("qmlRegisterType requires absolute URLs."));
        assert!(is_known_noise(
            "Unknown type of change in KGlobalSettings::slotNotifyChange:  7"
        ));
        assert!(!is_known_noise("qml: Binding loop detected"));
    }

    #[test]
    fn pattern_matches_the_parser() {
        let rendered = qt_message_pattern()
            .replace("%{type}", "info")
            .replace("%{if-category}%{category}: %{endif}", "")
            .replace("%{message}", "hello");

        assert_eq!(split_tag(&rendered), (Some(Level::Info), "hello"));
    }
}
