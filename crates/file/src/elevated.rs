use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use drakeflake_exec::{Captured, Context};
use drakeflake_plan::{WritePlan, exit};

use crate::elevation::{Prompt, find_elevation};
use crate::error::FileError;
use crate::write::WriteMethod;

const HELPER_TIMEOUT: Duration = Duration::from_secs(300);

pub fn write_elevated(
    exec: &Context,
    plan: &WritePlan,
    helper: Option<&Path>,
) -> Result<WriteMethod, FileError> {
    let helper = helper.ok_or(FileError::NoHelper)?;

    // No terminal is attached, so only a dialog can ask.
    let elevation = find_elevation(Prompt::DialogOnly).ok_or(FileError::NoElevation)?;

    let json = serde_json::to_string(plan).map_err(FileError::Encode)?;
    let mut command = elevation.prefix();
    command.extend([helper.as_os_str().to_owned(), OsString::from("apply")]);
    let (program, args) = command.split_first().ok_or(FileError::NoElevation)?;

    let captured = exec.command_with_input(program, args, &json, HELPER_TIMEOUT)?;

    if captured.success {
        Ok(WriteMethod::Elevated)
    } else {
        Err(helper_failure(&captured))
    }
}

fn helper_failure(captured: &Captured) -> FileError {
    let stderr = captured.stderr.trim().to_owned();

    match captured.code.and_then(|code| u8::try_from(code).ok()) {
        Some(exit::REFUSED) => FileError::HelperRefused(stderr),
        Some(exit::PKEXEC_NOT_AUTHORIZED | exit::PKEXEC_DISMISSED) => {
            FileError::AuthenticationDismissed(stderr)
        }
        Some(_) | None => FileError::HelperFailed(stderr),
    }
}

#[cfg(test)]
mod tests;
