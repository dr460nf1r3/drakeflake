use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use ::notify::event::{Event as FsEvent, EventKind};
use ::notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use drakeflake_tray_core::watch::{Event, Source, is_skipped};

pub struct FsEvents {
    events: Receiver<::notify::Result<FsEvent>>,
    roots: [PathBuf; 2],
    _watcher: RecommendedWatcher,
}

impl Source for FsEvents {
    fn wait(&mut self, timeout: Duration) -> Result<Event, String> {
        match self.events.recv_timeout(timeout) {
            Ok(Ok(event)) if is_change(&event, &self.roots) => Ok(Event::Changed),
            Ok(Ok(_)) | Err(RecvTimeoutError::Timeout) => Ok(Event::Quiet),
            Ok(Err(error)) => Err(format!("FSEvents failed: {error}")),
            Err(RecvTimeoutError::Disconnected) => Err("the FSEvents stream ended".to_owned()),
        }
    }
}

pub fn open(folder: &Path) -> Result<FsEvents, String> {
    let (sender, events) = mpsc::channel();

    let mut watcher = ::notify::recommended_watcher(sender)
        .map_err(|error| format!("FSEvents is unavailable: {error}"))?;

    watcher
        .watch(folder, RecursiveMode::Recursive)
        .map_err(|error| format!("cannot watch {}: {error}", folder.display()))?;

    // FSEvents reports resolved paths, `/private/var` for `/var`.
    let resolved = folder.canonicalize().unwrap_or_else(|_| folder.to_owned());

    Ok(FsEvents {
        events,
        roots: [folder.to_owned(), resolved],
        _watcher: watcher,
    })
}

fn is_change(event: &FsEvent, roots: &[PathBuf]) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && event
            .paths
            .iter()
            .any(|path| !in_skipped_folder(path, roots))
}

fn in_skipped_folder(path: &Path, roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .is_some_and(|below| {
            let mut folders = below.parent().into_iter().flat_map(Path::components);

            folders.any(|part| matches!(part, Component::Normal(name) if is_skipped(name)))
        })
}

#[cfg(test)]
mod tests;
