//! Git tracking of the managed file: flakes ignore untracked files.

use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use drakeflake_exec::{Captured, Context, ExecError};
use log::{debug, error, warn};

use crate::error::FileError;
use crate::paths::file_name_of;

const GIT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitState {
    NotARepository,
    Tracked,
    Untracked,
    Unknown,
    /// Git refuses to read a repository it distrusts, so whether the file is tracked is unknown.
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkTree {
    Inside,
    Outside,
    Refused,
    Unknown,
}

#[must_use = "only inspects the repository"]
pub fn git_state(exec: &Context, managed: &Path) -> GitState {
    let Some(parent) = managed.parent() else {
        return GitState::Unknown;
    };

    let name = file_name_of(managed);

    let work_tree = exec.probe(
        "git",
        [
            OsStr::new("-C"),
            parent.as_os_str(),
            OsStr::new("rev-parse"),
            OsStr::new("--is-inside-work-tree"),
        ],
        GIT_TIMEOUT,
    );

    if let Some(state) = outside_work_tree(&work_tree, parent) {
        return state;
    }

    let tracked = exec.probe(
        "git",
        [
            OsStr::new("-C"),
            parent.as_os_str(),
            OsStr::new("ls-files"),
            OsStr::new("--error-unmatch"),
            OsStr::new(&name),
        ],
        GIT_TIMEOUT,
    );

    tracking(&tracked, parent, &name)
}

#[must_use = "only inspects the probe result"]
pub fn outside_work_tree(probe: &Result<Captured, ExecError>, dir: &Path) -> Option<GitState> {
    match probe {
        Ok(captured) => match classify_work_tree(captured) {
            WorkTree::Inside => None,
            WorkTree::Outside => Some(GitState::NotARepository),
            WorkTree::Refused => {
                warn!("git refuses {}: dubious ownership", dir.display());
                Some(GitState::Refused)
            }
            WorkTree::Unknown => Some(GitState::Unknown),
        },
        Err(err) => {
            warn!("cannot check git state of {}: {err}", dir.display());
            Some(GitState::Unknown)
        }
    }
}

/// Git exits 128 both for a folder that is no repository and for one it distrusts.
fn classify_work_tree(captured: &Captured) -> WorkTree {
    let trimmed = captured.stdout.trim();

    if trimmed == "true" {
        WorkTree::Inside
    } else if trimmed == "false" {
        // Inside `.git` itself: there is no work tree to track a file in.
        WorkTree::Outside
    } else if captured.stderr.contains("dubious ownership") {
        WorkTree::Refused
    } else if captured
        .stderr
        .lines()
        .any(|line| line.contains("not a git repository"))
    {
        WorkTree::Outside
    } else {
        // Anything else must not be reported as no repository, or git init would run on it.
        warn!("git did not answer as expected: {}", captured.stderr.trim());
        WorkTree::Unknown
    }
}

fn tracking(probe: &Result<Captured, ExecError>, dir: &Path, name: &str) -> GitState {
    match probe {
        Ok(captured) if captured.success => GitState::Tracked,
        Ok(_) => GitState::Untracked,
        Err(err) => {
            warn!(
                "cannot check whether {name} is tracked in {}: {err}",
                dir.display()
            );
            GitState::Unknown
        }
    }
}

/// Intent-to-add makes the flake see the file without staging its content.
pub fn git_track(exec: &Context, managed: &Path) -> Result<(), FileError> {
    let parent = managed.parent().ok_or(FileError::NoOutputDirectory)?;
    let name = file_name_of(managed);
    let captured = exec.command(
        "git",
        [
            OsStr::new("-C"),
            parent.as_os_str(),
            OsStr::new("add"),
            OsStr::new("--intent-to-add"),
            OsStr::new(&name),
        ],
        GIT_TIMEOUT,
    )?;

    if captured.success {
        debug!("git add --intent-to-add {name} in {}", parent.display());
        Ok(())
    } else {
        let message = captured.stderr.trim();
        error!("cannot git add {name} in {}: {message}", parent.display());
        Err(FileError::Git(message.to_owned()))
    }
}

#[cfg(test)]
mod tests;
