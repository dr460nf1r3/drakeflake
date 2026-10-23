use std::process::{Command, Stdio};

/// `busctl` comes with systemd, so no D-Bus library is linked for this.
pub fn notify(summary: &str, body: &str) {
    let status = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
            "susssasa{sv}i",
            "DrakeFlake",
            "0",
            "drakeflake",
            summary,
            body,
            "0",
            "1",
            "desktop-entry",
            "s",
            "drakeflake",
            "-1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => drakeflake_log::debug!("notified: {summary}"),
        Ok(status) => drakeflake_log::warn!("notification failed: busctl {status}"),
        Err(error) => drakeflake_log::warn!("notification failed: cannot run busctl: {error}"),
    }
}
