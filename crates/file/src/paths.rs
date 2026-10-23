use std::fs;
use std::path::{Path, PathBuf};

use drakeflake_config::LayoutMode;
use drakeflake_plan::{FILE_NAME, generated_files, has_generated_header};

use crate::error::FileError;

/// Split layouts get a folder entry of their own, so the folder can be imported.
pub const FOLDER_ENTRY: &str = "default.nix";
pub const DEFAULT_FOLDER: &str = "drakeflake";

#[must_use = "the folder is only computed"]
pub fn settings_folder(entry: &Path) -> String {
    if entry.file_name().is_some_and(|name| name == FOLDER_ENTRY) {
        return String::new();
    }

    entry.file_stem().map_or_else(
        || DEFAULT_FOLDER.to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    )
}

/// Importing both entries would define everything twice, so the other one is replaced. Only generated files are returned.
#[must_use = "the files are only listed, nothing is removed"]
pub fn superseded_files(entry: &Path) -> Vec<PathBuf> {
    let Some(dir) = entry.parent() else {
        return Vec::new();
    };

    [FILE_NAME, FOLDER_ENTRY]
        .iter()
        .map(|name| dir.join(name))
        .filter(|other| other != entry)
        .filter_map(|other| {
            let text = fs::read_to_string(&other).ok()?;
            has_generated_header(&text).then(|| {
                generated_files(&text)
                    .into_iter()
                    .map(|part| dir.join(part))
                    .chain([other])
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect()
}

#[must_use = "the path is only computed, nothing is created"]
pub fn managed_path(flake_dir: &Path) -> PathBuf {
    flake_dir.join(FILE_NAME)
}

fn expand_home(text: &str) -> PathBuf {
    text.strip_prefix("~/")
        .and_then(|rest| std::env::var_os("HOME").map(|home| Path::new(&home).join(rest)))
        .unwrap_or_else(|| PathBuf::from(text))
}

pub fn resolve_managed_path(
    flake_dir: Option<&Path>,
    configured: Option<&str>,
    mode: LayoutMode,
) -> Result<PathBuf, FileError> {
    let Some(text) = configured.map(str::trim).filter(|text| !text.is_empty()) else {
        return flake_dir.map(managed_path).ok_or(FileError::NoLocalFlake);
    };

    let path = expand_home(text.strip_prefix("file://").unwrap_or(text));

    if !path.is_absolute() {
        return Err(FileError::OutputNotAbsolute(text.to_owned()));
    }

    if text.ends_with('/') {
        let entry = if mode == LayoutMode::Single {
            FILE_NAME
        } else {
            FOLDER_ENTRY
        };

        return Ok(path.join(entry));
    }

    // `symlink_metadata`: a symlinked folder is read as the link, not the
    // target, so the resolved path stays the one that was configured.
    if fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir()) {
        let entry = if mode == LayoutMode::Single {
            FILE_NAME
        } else {
            FOLDER_ENTRY
        };

        return Ok(path.join(entry));
    }

    if path.extension().is_some_and(|ext| ext == "nix") {
        Ok(path)
    } else {
        Err(FileError::OutputNotNix(text.to_owned()))
    }
}

/// Flakes can only import files inside themselves.
#[must_use = "only compares the paths"]
pub fn is_inside(dir: &Path, file: &Path) -> bool {
    let Some(parent) = file.parent() else {
        return false;
    };

    resolve_existing(parent).starts_with(resolve_existing(dir))
}

/// Resolves the nearest existing ancestor, since a managed folder is created on save.
#[must_use = "only resolves the path"]
pub fn resolve_existing(path: &Path) -> PathBuf {
    path.ancestors()
        .find_map(|ancestor| {
            let real = ancestor.canonicalize().ok()?;
            let rest = path.strip_prefix(ancestor).ok()?;
            Some(real.join(rest))
        })
        .unwrap_or_else(|| path.to_path_buf())
}

pub fn file_name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || FILE_NAME.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
