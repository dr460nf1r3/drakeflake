use std::collections::VecDeque;
use std::fs;
use std::path::{Component, Path, PathBuf};

use log::debug;

use crate::paths::{FOLDER_ENTRY, file_name_of};

/// Bounds the scan of big flakes (vendored trees, checkouts) so it stays quick.
const MAX_SCANNED_ENTRIES: usize = 20_000;
const MAX_SCAN_DEPTH: usize = 4;
/// Never hold configuration, but can be huge.
const SKIPPED_DIRS: [&str; 2] = ["result", "node_modules"];

/// Read from the syntax tree, so comments, strings and mentions of the file name never count as imports.
pub fn imports_file(text: &str, dir: &Path, managed: &Path) -> bool {
    drakeflake_validate::import_paths(text).iter().any(|entry| {
        let target = match entry.strip_prefix("~/") {
            Some(rest) => std::env::var_os("HOME").map(|home| Path::new(&home).join(rest)),
            None if entry.starts_with('/') => Some(PathBuf::from(entry)),
            None => Some(dir.join(entry)),
        };

        target.is_some_and(|target| resolved(&target) == managed)
    })
}

fn resolved(path: &Path) -> PathBuf {
    let file = if path.is_dir() {
        path.join(FOLDER_ENTRY)
    } else {
        path.to_path_buf()
    };

    // Canonicalize the nearest existing ancestor, else a symlink above a new folder keeps two spellings of one file apart.
    file.ancestors()
        .skip(1)
        .find_map(|dir| {
            let real = dir.canonicalize().ok()?;
            Some(lexically_normal(&real.join(file.strip_prefix(dir).ok()?)))
        })
        .unwrap_or_else(|| lexically_normal(&file))
}

fn lexically_normal(path: &Path) -> PathBuf {
    path.components().fold(PathBuf::new(), |mut normal, part| {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => normal.push(part),
        }

        normal
    })
}

/// A cheap filter before parsing, since a folder's default.nix is imported by the folder's name.
fn wanted_names(managed: &Path) -> Vec<String> {
    let name = file_name_of(managed);
    let folder = (name == FOLDER_ENTRY)
        .then(|| managed.parent().and_then(Path::file_name))
        .flatten()
        .map(|folder| folder.to_string_lossy().into_owned());

    std::iter::once(name).chain(folder).collect()
}

/// A link is followed for files only, never for folders (loops).
#[expect(
    clippy::filetype_is_file,
    reason = "only regular files are read: a FIFO would block"
)]
fn is_regular(path: &Path, kind: fs::FileType) -> bool {
    if kind.is_symlink() {
        fs::metadata(path).is_ok_and(|meta| meta.is_file())
    } else {
        kind.is_file()
    }
}

#[must_use = "only checks the imports, nothing is changed"]
pub fn is_imported(flake_dir: &Path, managed: &Path) -> bool {
    let wanted = wanted_names(managed);
    let managed = &resolved(managed);

    // Breadth-first: configurations import from near the top of the flake.
    let mut pending = VecDeque::from([(flake_dir.to_path_buf(), 0_usize)]);
    let mut visited = 0_usize;

    while let Some((dir, depth)) = pending.pop_front() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                debug!(
                    "skipping {} while looking for imports: {err}",
                    dir.display()
                );
                continue;
            }
        };

        let file_dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());

        for entry in entries.flatten() {
            visited += 1;

            if visited > MAX_SCANNED_ENTRIES {
                debug!(
                    "gave up looking for imports of {} after {MAX_SCANNED_ENTRIES} entries",
                    managed.display()
                );
                return false;
            }

            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if name.starts_with('.') || SKIPPED_DIRS.contains(&name.as_ref()) {
                continue;
            }

            let Ok(kind) = entry.file_type() else {
                continue;
            };

            if kind.is_dir() && depth < MAX_SCAN_DEPTH {
                pending.push_back((path, depth + 1));
                continue;
            }

            if is_regular(&path, kind)
                && path.extension().is_some_and(|ext| ext == "nix")
                && path != *managed
                && fs::read_to_string(&path).is_ok_and(|text| {
                    wanted.iter().any(|needle| text.contains(needle.as_str()))
                        && imports_file(&text, &file_dir, managed)
                })
            {
                debug!("{} imports {}", path.display(), managed.display());
                return true;
            }
        }
    }

    false
}
