use std::fs::{self, DirBuilder};
use std::io::ErrorKind;
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _};
use std::path::{Path, PathBuf};

use crate::DirsError;

const OWNER_ONLY: u32 = 0o700;
const GROUP_AND_OTHER: u32 = 0o077;

/// Reused only if it is a real folder of this user nobody else can enter, since the temp dir is shared.
pub fn private_dir(temp: &Path, uid: u32) -> Result<PathBuf, DirsError> {
    let path = temp.join(format!("drakeflake-{uid}"));
    let refuse = |reason: String| DirsError {
        path: path.clone(),
        reason,
    };

    match DirBuilder::new().mode(OWNER_ONLY).create(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => return Err(refuse(format!("cannot be created ({err})"))),
    }

    let meta = fs::symlink_metadata(&path)
        .map_err(|err| refuse(format!("cannot be inspected ({err})")))?;

    if !meta.is_dir() {
        Err(refuse("is not a real folder".to_owned()))
    } else if meta.uid() != uid {
        Err(refuse("belongs to another user".to_owned()))
    } else if meta.mode() & GROUP_AND_OTHER != 0 {
        Err(refuse("can be entered by other users".to_owned()))
    } else {
        Ok(path)
    }
}

#[cfg(test)]
mod tests;
