//! One tray per user session: two would notify twice.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write as _;
use std::path::Path;

enum NotStarted {
    AlreadyRunning,
    NoLock(String),
}

fn claim_single_instance() -> Result<File, NotStarted> {
    let path = drakeflake_config::tray_lock_path()
        .map_err(|error| NotStarted::NoLock(error.to_string()))?;

    claim(&path)
}

fn claim(path: &Path) -> Result<File, NotStarted> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| {
            NotStarted::NoLock(format!("cannot create {}: {error}", dir.display()))
        })?;
    }

    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| NotStarted::NoLock(format!("cannot open {}: {error}", path.display())))?;

    match file.try_lock() {
        Ok(()) => {
            drakeflake_log::debug!("holding the tray lock {}", path.display());
            Ok(file)
        }
        Err(TryLockError::WouldBlock) => Err(NotStarted::AlreadyRunning),
        Err(TryLockError::Error(error)) => Err(NotStarted::NoLock(format!(
            "cannot lock {}: {error}",
            path.display()
        ))),
    }
}

#[must_use = "dropping the file releases the tray lock"]
pub fn single_instance_or_exit() -> File {
    match claim_single_instance() {
        Ok(lock) => lock,
        Err(NotStarted::AlreadyRunning) => {
            drakeflake_log::info!("another tray is already running; exiting");

            // The log already has it, so a closed stderr loses nothing.
            writeln!(std::io::stderr(), "drakeflake-tray: already running").ok();
            #[expect(
                clippy::exit,
                reason = "a second tray must not run, and its caller has nothing to clean up"
            )]
            std::process::exit(0);
        }
        Err(NotStarted::NoLock(reason)) => {
            drakeflake_log::error!("not starting: {reason}");
            writeln!(std::io::stderr(), "drakeflake-tray: not starting: {reason}").ok();
            #[expect(
                clippy::exit,
                reason = "an unguarded tray must not run, and its caller has nothing to clean up"
            )]
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests;
