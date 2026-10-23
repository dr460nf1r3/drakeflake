#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use drakeflake_config::Tray;

use super::SettingsModel;

const PROGRAM: &str = if cfg!(target_os = "macos") {
    "drakeflake-tray-macos"
} else {
    "drakeflake-tray"
};

const DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=DrakeFlake (tray)\nComment=Checks your system flake for updates\nExec=drakeflake-tray\nIcon=drakeflake\nX-KDE-autostart-after=panel\n";

/// A login shell finds the tray on `PATH`, so package updates (new store paths) need no new agent.
const LAUNCH_AGENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>org.dr460nf1r3.drakeflake.tray</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sh</string>
    <string>-lc</string>
    <string>exec drakeflake-tray-macos</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#;

#[derive(Debug, PartialEq, Eq)]
pub enum TrayEdit {
    Refused,
    Unchanged,
    Changed(Tray),
}

impl SettingsModel {
    pub(super) fn edit_tray_settings(&self, edit: impl FnOnce(&mut Tray) -> bool) -> bool {
        let current = self.config.read(|config| config.tray);

        match edit_tray(current, edit) {
            TrayEdit::Changed(tray) => {
                self.config.update(|config| config.tray = tray);
                true
            }
            TrayEdit::Unchanged => true,
            TrayEdit::Refused => false,
        }
    }
}

pub fn autostart_file() -> Option<PathBuf> {
    autostart_file_in(drakeflake_fs::home_dir(), drakeflake_fs::config_home())
}

fn autostart_file_in(home: Option<PathBuf>, config_home: Option<PathBuf>) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return home
            .map(|home| home.join("Library/LaunchAgents/org.dr460nf1r3.drakeflake.tray.plist"));
    }

    Some(config_home?.join("autostart/drakeflake-tray.desktop"))
}

pub fn start() -> Result<(), String> {
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let dir = exe.parent()?;
            [dir.join(PROGRAM), dir.join("../bin").join(PROGRAM)]
                .into_iter()
                .find(|path| path.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(PROGRAM));

    log::info!("starting the tray: {}", program.display());

    crate::hub::terminal::spawn_detached(&[program.into_os_string()])
        .map_err(|error| format!("Could not start the tray: {error}"))
}

pub fn start_at_launch(report: impl FnOnce(String)) {
    let autostart = autostart_file().is_some_and(|file| file.is_file());

    if wants_start(autostart, drakeflake_config::tray_running())
        && let Err(error) = start()
    {
        report(error);
    }
}

const fn wants_start(autostart: bool, running: bool) -> bool {
    autostart && !running
}

pub fn set_autostart(file: &Path, enabled: bool) -> Result<(), String> {
    let entry = if cfg!(target_os = "macos") {
        LAUNCH_AGENT
    } else {
        DESKTOP_ENTRY
    };

    let result = if enabled {
        file.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(file, entry))
    } else {
        match std::fs::remove_file(file) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    };

    result.map_err(|error| format!("Cannot change {}: {error}", file.display()))?;
    log::info!(
        "tray autostart {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(())
}

pub fn edit_tray(current: Tray, edit: impl FnOnce(&mut Tray) -> bool) -> TrayEdit {
    let mut next = current;

    if !edit(&mut next) {
        TrayEdit::Refused
    } else if next == current {
        TrayEdit::Unchanged
    } else {
        TrayEdit::Changed(next)
    }
}
