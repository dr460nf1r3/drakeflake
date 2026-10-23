use std::path::Path;

use drakeflake_exec::LONG_TIMEOUT;

use crate::context::Nix;
use crate::error::FlakeError;
use crate::flake_target::Target;
use crate::nix::Query;

pub fn module_arg_files(nix: &Nix, target: &Target) -> Result<Vec<String>, FlakeError> {
    target.eval_json(
        nix,
        &Query::new("module file list", LONG_TIMEOUT).attr(".options._module.args.files"),
    )
}

#[must_use = "only checks the list"]
pub fn lists_file(files: &[String], relative: &Path) -> bool {
    // An imported folder is recorded as the folder, not its `default.nix`.
    let folder = relative
        .file_name()
        .is_some_and(|name| name == "default.nix")
        .then(|| relative.parent())
        .flatten();

    files.iter().any(|file| {
        file.strip_prefix("/nix/store/")
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(_, inside)| {
                let inside = Path::new(inside);
                inside == relative || folder == Some(inside)
            })
    })
}

#[cfg(test)]
mod tests;
