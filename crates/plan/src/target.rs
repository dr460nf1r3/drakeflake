//! The helper runs as root on a plan from the unprivileged app, so these checks are the security boundary and never follow symlinks or `..` out of a flake.

use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path};

use crate::header::has_generated_header;
use crate::refusal::Refusal;
use crate::root::find_flake_root;
use crate::state::check_state_line;
use crate::syntax::check_syntax;

pub fn check_write_target(target: &Path) -> Result<(), Refusal> {
    check_path_shape(target)?;
    check_in_flake(target)?;
    check_overwritable(target)
}

/// Without the flake rule, because the app's output file may be configured anywhere.
pub fn check_local_write_target(target: &Path) -> Result<(), Refusal> {
    check_path_shape(target)?;
    check_overwritable(target)
}

fn check_path_shape(target: &Path) -> Result<(), Refusal> {
    if !target.is_absolute() {
        return Err(Refusal::NotAbsolute);
    }

    if target
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(Refusal::ParentDir);
    }

    if target.extension().is_none_or(|ext| ext != "nix") {
        return Err(Refusal::NotNixFile);
    }

    Ok(())
}

/// The nearest existing folder is resolved, so a symlinked folder cannot lead out of the flake.
fn check_in_flake(target: &Path) -> Result<(), Refusal> {
    let existing = target
        .ancestors()
        .skip(1)
        .find(|dir| dir.is_dir())
        .ok_or(Refusal::NoExistingParent)?;

    let existing = existing
        .canonicalize()
        .map_err(|err| Refusal::io("resolve", existing, err))?;

    if existing.starts_with("/nix/store") {
        return Err(Refusal::InNixStore(existing));
    }

    if find_flake_root(&existing).is_none() {
        return Err(Refusal::NotInFlake(existing));
    }

    Ok(())
}

fn check_overwritable(target: &Path) -> Result<(), Refusal> {
    // A symlink at the target is refused, not followed.
    match fs::symlink_metadata(target) {
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Refusal::inspect(target, err)),
        #[expect(
            clippy::filetype_is_file,
            reason = "a directory, symlink or special file at the target is refused"
        )]
        Ok(meta) if !meta.file_type().is_file() => Err(Refusal::NotRegularFile),
        Ok(_) => {
            let current =
                fs::read_to_string(target).map_err(|err| Refusal::io("read", target, err))?;

            if current.trim().is_empty() || has_generated_header(&current) {
                Ok(())
            } else {
                Err(Refusal::NotGenerated(target.to_path_buf()))
            }
        }
    }
}

pub fn check_write_content(text: &str) -> Result<(), Refusal> {
    check_generated_content(text)?;
    check_syntax(text)?;

    Ok(())
}

pub fn check_generated_content(text: &str) -> Result<(), Refusal> {
    if !has_generated_header(text) {
        return Err(Refusal::NotModule);
    }

    check_state_line(text)?;

    Ok(())
}

pub fn check_remove_target(target: &Path) -> Result<(), Refusal> {
    check_write_target(target)?;
    check_removable(target)
}

pub fn check_local_remove_target(target: &Path) -> Result<(), Refusal> {
    check_local_write_target(target)?;
    check_removable(target)
}

fn check_removable(target: &Path) -> Result<(), Refusal> {
    match fs::symlink_metadata(target) {
        #[expect(
            clippy::filetype_is_file,
            reason = "only regular files are removed, never a symlink or special file"
        )]
        Ok(meta) if meta.file_type().is_file() => Ok(()),
        Ok(_) => Err(Refusal::NotRemovable),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(Refusal::inspect(target, err)),
    }
}
