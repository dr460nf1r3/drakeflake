use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use drakeflake_fs::fnv1a;
use drakeflake_plan::ManagedState;
use serde::Serialize;

/// Backup names carry the save time as seconds since the epoch, at least this many
/// digits, which sets it apart from the sequence number.
const MIN_SECONDS_DIGITS: usize = 9;

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub file: String,
    pub saved: u64,
    pub options: usize,
    pub packages: usize,
    pub changes: Vec<String>,
}

pub fn history_dir(entry: &Path) -> Result<PathBuf, drakeflake_fs::DirsError> {
    Ok(drakeflake_file::backup_dir()?.join(format!(
        "{:016x}",
        fnv1a(entry.to_string_lossy().as_bytes())
    )))
}

#[must_use = "only reads the backups, nothing is restored"]
pub fn list_history(entry: &Path, current: &ManagedState) -> Vec<HistoryEntry> {
    history_dir(entry).map_or_else(
        |err| {
            log::warn!("no history for {}: {err}", entry.display());
            Vec::new()
        },
        |dir| list_backups(&dir, entry, current),
    )
}

fn list_backups(dir: &Path, entry: &Path, current: &ManagedState) -> Vec<HistoryEntry> {
    let files = match std::fs::read_dir(dir) {
        Ok(files) => files,
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                log::debug!("no history for {} yet", entry.display());
            } else {
                log::warn!("cannot list the history in {}: {err}", dir.display());
            }

            return Vec::new();
        }
    };

    let mut entries: Vec<HistoryEntry> = files
        .filter_map(Result::ok)
        .map(|file| file.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bak"))
        .filter_map(|path| {
            let state = read_backup(&path)?;

            Some(HistoryEntry {
                saved: saved_at(&path),
                options: state.options.len(),
                packages: state.packages.len(),
                changes: differences(current, &state),
                file: path.to_string_lossy().into_owned(),
            })
        })
        .collect();

    entries.sort_by_key(|earlier| std::cmp::Reverse(earlier.saved));
    log::debug!("history of {}: {} entries", entry.display(), entries.len());
    entries
}

fn read_backup(path: &Path) -> Option<ManagedState> {
    let text = std::fs::read_to_string(path)
        .inspect_err(|err| log::warn!("cannot read backup {}: {err}", path.display()))
        .ok()?;

    match ManagedState::parse(&text) {
        Ok(Some(state)) => Some(state),
        Ok(None) => {
            log::debug!("backup {} has no state line", path.display());
            None
        }
        Err(err) => {
            log::warn!("cannot parse backup {}: {err}", path.display());
            None
        }
    }
}

fn saved_at(path: &Path) -> u64 {
    path.file_name()
        .and_then(|name| {
            name.to_string_lossy()
                .split('.')
                .find(|part| {
                    part.len() >= MIN_SECONDS_DIGITS && part.chars().all(|ch| ch.is_ascii_digit())
                })
                .and_then(|seconds| seconds.parse().ok())
        })
        .unwrap_or(0)
}

fn differences(current: &ManagedState, earlier: &ManagedState) -> Vec<String> {
    let mut lines = Vec::new();

    for (name, option) in &earlier.options {
        match current.options.get(name) {
            None => lines.push(format!("{name} = {}", option.input)),
            Some(now) if now.input != option.input => {
                lines.push(format!("{name}: {} → {}", now.input, option.input));
            }
            Some(_) => {}
        }
    }

    for name in current.options.keys() {
        if !earlier.options.contains_key(name) {
            lines.push(format!("{name}: no longer set"));
        }
    }

    for attr in earlier.packages.difference(&current.packages) {
        lines.push(format!("+ package {attr}"));
    }

    for attr in current.packages.difference(&earlier.packages) {
        lines.push(format!("− package {attr}"));
    }

    lines
}

#[cfg(test)]
mod tests;
