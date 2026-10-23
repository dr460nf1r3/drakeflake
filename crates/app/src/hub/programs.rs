#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use super::Cache;

const NIX_STORE: &str = "/nix/store";

pub(super) fn build_dir() -> Result<PathBuf, String> {
    let dir = Cache::root().map_err(|err| err.to_string())?.join("build");
    if let Err(err) = fs::create_dir_all(&dir) {
        log::warn!("cannot create {}: {err}", dir.display());
    }

    Ok(dir)
}

pub fn helper_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("DRAKEFLAKE_HELPER").map(PathBuf::from) {
        let usable = path.is_absolute() && path.is_file();
        if !usable {
            log::warn!(
                "DRAKEFLAKE_HELPER={} is not an absolute path to a file; ignored",
                path.display()
            );
        }

        return usable.then_some(path);
    }

    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    [
        dir.join("../libexec/drakeflake-helper"),
        dir.join("drakeflake-helper"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
    .map(|found| found.canonicalize().unwrap_or(found))
}

pub fn find_program(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;

    let paths = std::env::var_os("PATH")?;

    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|candidate| {
            fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

pub fn find_root_program(name: &str) -> Result<PathBuf, String> {
    let found = find_program(name).ok_or_else(|| format!("{name} is not installed."))?;
    in_store(&found)
}

pub fn in_store(path: &Path) -> Result<PathBuf, String> {
    let resolved = path
        .canonicalize()
        .map_err(|err| format!("Cannot resolve {}: {err}", path.display()))?;

    if resolved.starts_with(NIX_STORE) {
        Ok(resolved)
    } else {
        Err(format!(
            "Not running {} as root: it is not in the Nix store.",
            resolved.display()
        ))
    }
}
