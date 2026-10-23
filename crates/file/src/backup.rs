use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use drakeflake_fs::DirsError;
use drakeflake_plan::{FILE_NAME, next_serial};
use log::{debug, error};

use crate::error::FileError;

const MAX_NAME_RETRIES: u32 = 100;

/// User-owned and outside the flake, so backups are never imported or committed.
pub fn backup_dir() -> Result<PathBuf, DirsError> {
    Ok(drakeflake_fs::state_dir()?.join("backups"))
}

pub fn staged_dir() -> Result<PathBuf, DirsError> {
    Ok(drakeflake_fs::state_dir()?.join("staged"))
}

pub fn backup_to(path: &Path, dir: &Path) -> Result<Option<PathBuf>, FileError> {
    if !path.exists() {
        return Ok(None);
    }

    fs::create_dir_all(dir).map_err(|source| {
        error!("cannot create backup dir {}: {source}", dir.display());
        FileError::BackupDir {
            dir: dir.to_path_buf(),
            source,
        }
    })?;

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |age| age.as_secs());

    let target = copy_unique(path, dir, seconds).map_err(|source| {
        error!(
            "cannot back up {} into {}: {source}",
            path.display(),
            dir.display()
        );
        FileError::Backup {
            path: path.to_path_buf(),
            source,
        }
    })?;

    debug!("backed up {} to {}", path.display(), target.display());

    Ok(Some(target))
}

/// Another process may pick the same second, so the name is claimed with `create_new` and retried.
fn copy_unique(path: &Path, dir: &Path, seconds: u64) -> std::io::Result<PathBuf> {
    let mut source = File::open(path)?;
    let mut attempts = 0_u32;

    loop {
        let target = dir.join(format!(
            "{FILE_NAME}.{seconds}.{}.{}.bak",
            std::process::id(),
            next_serial()
        ));

        match File::create_new(&target) {
            Ok(mut backup) => {
                return std::io::copy(&mut source, &mut backup)
                    .and_then(|_| backup.sync_all())
                    .map(|()| target.clone())
                    .inspect_err(|_| {
                        fs::remove_file(&target).ok();
                    });
            }
            Err(err) if err.kind() == ErrorKind::AlreadyExists && attempts < MAX_NAME_RETRIES => {
                attempts += 1;
            }
            Err(err) => return Err(err),
        }
    }
}
