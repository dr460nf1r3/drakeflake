use std::path::{Path, PathBuf};

use crate::error::HelperError;
use crate::exec::resolve;
use crate::{MAX_INPUT_BYTES, secure};

pub fn checked_flake_dir(dir: &Path) -> Result<PathBuf, HelperError> {
    if !dir.is_absolute() {
        return Err(HelperError::NotAbsolute(dir.to_path_buf()));
    }

    let resolved = resolve(dir)?;

    if resolved.starts_with("/nix/store") || !resolved.join("flake.nix").is_file() {
        return Err(HelperError::NotAFlake(resolved));
    }

    Ok(resolved)
}

pub fn check_lock(content: &[u8]) -> Result<(), HelperError> {
    let lock: serde_json::Value =
        serde_json::from_slice(content).map_err(|err| HelperError::InvalidLock(err.to_string()))?;

    let valid = lock.get("version").is_some_and(serde_json::Value::is_u64)
        && lock.get("root").is_some_and(serde_json::Value::is_string)
        && lock.get("nodes").is_some_and(serde_json::Value::is_object);

    if valid {
        Ok(())
    } else {
        Err(HelperError::InvalidLock(
            "missing version, root or nodes".to_owned(),
        ))
    }
}

/// pkexec sets `PKEXEC_UID` itself and drops the caller's environment.
pub fn caller_uid() -> Option<u32> {
    if rustix::process::geteuid().as_raw() != 0 {
        return None;
    }

    ["PKEXEC_UID", "SUDO_UID"]
        .iter()
        .find_map(|name| std::env::var(name).ok()?.parse().ok())
}

pub fn install_lock(dir: &Path, lock_file: &Path, owner: Option<u32>) -> Result<(), HelperError> {
    let owner = owner.ok_or(HelperError::UnknownCaller)?;
    let dir = checked_flake_dir(dir)?;
    let content = secure::canonical(lock_file)
        .and_then(|path| secure::read_user_file(&path, MAX_INPUT_BYTES, Some(owner)))
        .map_err(|source| HelperError::ReadLock {
            path: lock_file.to_path_buf(),
            source,
        })?;

    check_lock(&content)?;

    let target = dir.join("flake.lock");
    secure::write_file(&target, &content).map_err(|source| HelperError::WriteLock {
        path: target,
        source,
    })
}
