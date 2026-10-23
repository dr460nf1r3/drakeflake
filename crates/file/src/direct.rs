#[cfg(test)]
mod tests;

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use drakeflake_fs::{sibling, stage};
use drakeflake_plan::ValidatedPlan;
use log::{debug, warn};

struct Staged<'a> {
    path: &'a Path,
    temp: PathBuf,
    parent: PathBuf,
}

/// Entry last, so a failure before the last swap leaves the files as they were.
pub fn apply_direct(plan: &ValidatedPlan) -> io::Result<()> {
    let mut staged = Vec::new();

    for (path, text) in plan.ordered_writes() {
        if let Err(err) = stage_next(path, text, &mut staged) {
            discard(&staged);
            return Err(err);
        }
    }

    swap_all(&staged)?;

    for path in plan.removals() {
        match fs::remove_file(path) {
            Ok(()) => debug!("removed {}", path.display()),
            Err(err) if err.kind() != ErrorKind::NotFound => return Err(err),
            Err(_) => {}
        }

        if let Some(parent) = path.parent() {
            fs::remove_dir(parent).ok();
        }
    }

    Ok(())
}

fn stage_next<'a>(path: &'a Path, text: &str, staged: &mut Vec<Staged<'a>>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp = stage(path, text)?;
    debug!("staged {} as {}", path.display(), temp.display());
    let parent = path
        .parent()
        .map_or_else(|| PathBuf::from("/"), Path::to_path_buf)
        .canonicalize()?;

    staged.push(Staged { path, temp, parent });

    Ok(())
}

fn swap_all(staged: &[Staged<'_>]) -> io::Result<()> {
    let mut done: Vec<Option<PathBuf>> = Vec::new();

    for item in staged {
        match swap(item) {
            Ok(backup) => done.push(backup),
            Err(err) => {
                restore(staged, &done);
                discard(staged);
                return Err(err);
            }
        }
    }

    for backup in done.iter().flatten() {
        fs::remove_file(backup).ok();
    }

    Ok(())
}

fn swap(item: &Staged<'_>) -> io::Result<Option<PathBuf>> {
    check_parent_unchanged(item)?;
    check_not_symlink(item.path)?;

    let backup = sibling(item.path, "old");

    // A hard link keeps the target in place until the rename replaces it.
    let kept = match fs::hard_link(item.path, &backup) {
        Ok(()) => true,
        Err(err) if err.kind() == ErrorKind::NotFound => false,
        Err(_) => match fs::rename(item.path, &backup) {
            Ok(()) => true,
            Err(err) if err.kind() == ErrorKind::NotFound => false,
            Err(err) => return Err(err),
        },
    };

    match fs::rename(&item.temp, item.path) {
        Ok(()) => Ok(kept.then_some(backup)),
        Err(err) => {
            if kept {
                if let Err(undo) = fs::rename(&backup, item.path) {
                    warn!("cannot restore {}: {undo}", item.path.display());
                }

                // Renaming one hard link onto the other leaves both in place.
                fs::remove_file(&backup).ok();
            }

            Err(err)
        }
    }
}

fn restore(staged: &[Staged<'_>], done: &[Option<PathBuf>]) {
    for (item, backup) in staged.iter().zip(done).rev() {
        let undone = backup.as_ref().map_or_else(
            || fs::remove_file(item.path),
            |kept| fs::rename(kept, item.path),
        );

        if let Err(err) = undone {
            warn!("cannot restore {}: {err}", item.path.display());
        }
    }
}

/// Stops a directory swapped for a symlink in between from redirecting the write.
fn check_parent_unchanged(item: &Staged<'_>) -> io::Result<()> {
    let now = item.parent.canonicalize()?;

    if now != item.parent {
        return Err(io::Error::other(format!(
            "{} changed since staging",
            item.parent.display()
        )));
    }

    Ok(())
}

/// The target must not be a symlink, or the rename would replace the link target and bypass validation.
fn check_not_symlink(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(io::Error::other(format!("{} is a symlink", path.display())));
    }

    Ok(())
}

fn discard(staged: &[Staged<'_>]) {
    for item in staged {
        fs::remove_file(&item.temp).ok();
    }
}
