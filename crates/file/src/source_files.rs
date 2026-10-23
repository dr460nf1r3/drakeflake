use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use drakeflake_exec::Context;

const GIT_TIMEOUT: Duration = Duration::from_secs(20);

/// More files than this are no config, and listing them would stall typing.
const MAX_FILES: usize = 5000;

/// Git's list where there is a repository, since Nix sees the flake through it.
#[must_use = "only lists files"]
pub fn nix_files(exec: &Context, dir: &Path) -> Vec<PathBuf> {
    let mut files = git_nix_files(exec, dir).unwrap_or_else(|| {
        let mut found = Vec::new();
        walk(dir, &mut found);
        found
    });

    files.sort();
    files.truncate(MAX_FILES);
    files
}

fn git_nix_files(exec: &Context, dir: &Path) -> Option<Vec<PathBuf>> {
    let captured = exec
        .probe(
            "git",
            [
                OsStr::new("-C"),
                dir.as_os_str(),
                OsStr::new("ls-files"),
                OsStr::new("-z"),
                OsStr::new("--cached"),
                OsStr::new("--others"),
                OsStr::new("--exclude-standard"),
                OsStr::new("--deduplicate"),
                OsStr::new("--"),
                OsStr::new("*.nix"),
            ],
            GIT_TIMEOUT,
        )
        .ok()
        .filter(|captured| captured.success)?;

    Some(
        captured
            .stdout
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(|name| dir.join(name))
            .filter(|path| path.is_file())
            .collect(),
    )
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        log::debug!("cannot list {} for the source search", dir.display());
        return;
    };

    for entry in entries.flatten() {
        if found.len() >= MAX_FILES {
            return;
        }

        let path = entry.path();
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        let Ok(kind) = entry.file_type() else {
            continue;
        };

        if hidden || kind.is_symlink() {
            continue;
        }

        if kind.is_dir() {
            walk(&path, found);
            continue;
        }

        if path.extension().is_some_and(|extension| extension == "nix") {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests;
