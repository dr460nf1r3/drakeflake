use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::HelperError;

pub fn resolve(path: &Path) -> Result<PathBuf, HelperError> {
    path.canonicalize().map_err(|source| HelperError::Resolve {
        path: path.to_path_buf(),
        source,
    })
}

pub fn run_command(program: &OsStr, command: &mut Command) -> Result<(), HelperError> {
    let name = program.to_string_lossy().into_owned();
    let status = command.status().map_err(|source| HelperError::Spawn {
        program: name.clone(),
        source,
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(HelperError::Failed {
            program: name,
            status,
        })
    }
}

pub fn run(
    program: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<(), HelperError> {
    let program = program.as_ref();

    run_command(program, Command::new(program).args(args))
}
