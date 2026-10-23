use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use drakeflake_flake::Nix;

use crate::evaluate::{Spawned, spawn};

/// Editors save in bursts.
const QUIET: Duration = Duration::from_secs(30);
pub const RECHECK: Duration = Duration::from_secs(60);
/// Build outputs change without the configuration changing.
const SKIPPED_FOLDERS: [&str; 2] = ["result", "target"];

#[must_use = "a watch on the old folder or with the switch off must end"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    Continue,
    Restart,
}

pub struct Settle {
    nix: Nix,
    pending: Option<Instant>,
    last_recheck: Instant,
}

impl Settle {
    #[must_use = "the wait state does nothing by itself"]
    pub fn new(nix: Nix, carried: Option<Instant>) -> Self {
        Self {
            nix,
            pending: carried,
            last_recheck: Instant::now(),
        }
    }

    #[must_use = "the time to wait for the next event"]
    pub fn wait(&self) -> Duration {
        self.pending
            .map_or(RECHECK, |since| QUIET.saturating_sub(since.elapsed()))
    }

    pub fn changed(&mut self) {
        self.pending = Some(Instant::now());
    }

    #[must_use = "a waiting change is lost unless the next watch carries it"]
    pub const fn pending(&self) -> Option<Instant> {
        self.pending
    }

    pub fn tick(&mut self, folder: &Path) -> Tick {
        if self.pending.is_some_and(|since| since.elapsed() >= QUIET) {
            self.pending = evaluate(&self.nix);
        }

        if self.last_recheck.elapsed() < RECHECK {
            return Tick::Continue;
        }

        self.last_recheck = Instant::now();

        if watched_folder().as_deref() == Some(folder) {
            Tick::Continue
        } else {
            drakeflake_log::info!("the flake watch settings changed, restarting the watch");
            Tick::Restart
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Quiet,
    Changed,
    RootGone,
}

pub trait Source {
    fn wait(&mut self, timeout: Duration) -> Result<Event, String>;
}

#[must_use = "only reads the settings"]
pub fn watched_folder() -> Option<PathBuf> {
    let (config, _) = drakeflake_config::load();

    drakeflake_flake::local_flake_dir(&config.nix.flake_path).filter(|_| config.tray.watch_flake)
}

#[must_use = "only compares the name"]
pub fn is_skipped(name: &OsStr) -> bool {
    name.as_bytes().starts_with(b".") || SKIPPED_FOLDERS.iter().any(|skipped| name == *skipped)
}

pub fn run<S>(nix: &Nix, open: impl Fn(&Path) -> Result<S, String>) -> !
where
    S: Source,
{
    let mut carried = None;

    loop {
        let Some(folder) = watched_folder() else {
            std::thread::sleep(RECHECK);
            continue;
        };

        drakeflake_log::info!("watching {} for changes", folder.display());

        let mut settle = Settle::new(nix.clone(), carried);
        let ended = watch(&mut settle, &folder, &open);
        carried = settle.pending();

        if let Err(error) = ended {
            drakeflake_log::warn!("flake watch stopped: {error}");
            std::thread::sleep(RECHECK);
        }
    }
}

fn watch<S>(
    settle: &mut Settle,
    folder: &Path,
    open: &impl Fn(&Path) -> Result<S, String>,
) -> Result<(), String>
where
    S: Source,
{
    let mut source = open(folder)?;

    loop {
        match source.wait(settle.wait())? {
            Event::Quiet => {}
            Event::Changed => settle.changed(),
            Event::RootGone => {
                drakeflake_log::info!("the flake folder was moved or removed");
                settle.changed();
                return Ok(());
            }
        }

        if settle.tick(folder) == Tick::Restart {
            return Ok(());
        }
    }
}

/// A start that did not happen restarts the quiet time, so the change is not lost.
fn evaluate(nix: &Nix) -> Option<Instant> {
    let (config, _) = drakeflake_config::load();

    match spawn(nix.clone(), config) {
        Spawned::Started => {
            drakeflake_log::info!("flake files changed, starting the background evaluation");
            None
        }
        Spawned::AlreadyRunning | Spawned::Failed => Some(Instant::now()),
    }
}

#[cfg(test)]
mod tests;
