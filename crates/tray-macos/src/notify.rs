//! Notifications go through `osascript`, because the unsigned nix build cannot use `UNUserNotificationCenter`.
//! The texts go in as arguments, never into the script itself.
use std::process::{Command, Stdio};

const SCRIPT: [&str; 6] = [
    "-e",
    "on run argv",
    "-e",
    "display notification (item 2 of argv) with title \"DrakeFlake\" subtitle (item 1 of argv) sound name \"Glass\"",
    "-e",
    "end run",
];

pub fn notify(summary: &str, body: &str) {
    let status = Command::new("/usr/bin/osascript")
        .args(SCRIPT)
        .args([summary, body])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => drakeflake_log::debug!("notified: {summary}"),
        Ok(status) => drakeflake_log::warn!("notification failed: osascript {status}"),
        Err(error) => drakeflake_log::warn!("notification failed: cannot run osascript: {error}"),
    }
}
