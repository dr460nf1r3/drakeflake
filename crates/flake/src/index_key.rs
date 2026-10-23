//! Generated files are left out of the keys so an Apply does not rebuild the option index.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use drakeflake_exec::{Context, SHORT_TIMEOUT};
use drakeflake_fs::{fnv1a, fnv1a_from};

use crate::expr::EVAL_VERSION;
use crate::flake_target::{SystemKind, Target, local_flake_dir};

const FILE_LIMIT: usize = 20_000;

#[must_use = "hashing the sources is wasted if the key is dropped"]
pub fn option_index_key(
    exec: &Context,
    nix: &crate::context::Nix,
    target: &Target,
    generated: &[PathBuf],
) -> Option<String> {
    if let Some(dir) = target.local_dir().and_then(|dir| dir.canonicalize().ok()) {
        let (files, hash) = source_hash(exec, &dir, generated);

        let key = format!(
            "index v{EVAL_VERSION} {}:{} {files} files {hash:016x}",
            target.kind.as_str(),
            target.host
        );

        log::debug!("option index key of {}: {key}", dir.display());
        return Some(key);
    }

    let nar_hash = remote_nar_hash(nix, target)?;

    let key = format!(
        "index v{EVAL_VERSION} {}:{} nar {nar_hash}",
        target.kind.as_str(),
        target.host
    );

    log::debug!("option index key of {}: {key}", target.flake.trim());
    Some(key)
}

fn remote_nar_hash(nix: &crate::context::Nix, target: &Target) -> Option<String> {
    let dir = target.local_dir();

    let stdout = crate::nix::run_nix(
        nix,
        dir.as_deref(),
        ["flake", "metadata", "--json", target.flake.trim()],
        SHORT_TIMEOUT,
    )
    .inspect_err(|error| log::debug!("cannot get narHash of {}: {error}", target.flake.trim()))
    .ok()?;

    let metadata: serde_json::Value = serde_json::from_str(&stdout)
        .inspect_err(|error| {
            log::debug!(
                "flake metadata of {} is not JSON: {error}",
                target.flake.trim()
            );
        })
        .ok()?;

    let nar_hash = metadata
        .pointer("/locked/narHash")
        .and_then(serde_json::Value::as_str)?;

    Some(nar_hash.to_owned())
}

/// The list is ordered by this machine's kind, so that is part of the key too.
#[must_use = "hashing the sources is wasted if the key is dropped"]
pub fn hosts_key(exec: &Context, flake: &str, generated: &[PathBuf]) -> Option<String> {
    let dir = local_flake_dir(flake)?.canonicalize().ok()?;
    let (files, hash) = source_hash(exec, &dir, generated);

    let key = format!(
        "hosts v{EVAL_VERSION} {} {files} files {hash:016x}",
        SystemKind::native().as_str()
    );

    log::debug!("host list key of {}: {key}", dir.display());
    Some(key)
}

fn source_hash(exec: &Context, dir: &Path, generated: &[PathBuf]) -> (usize, u64) {
    let excluded: Vec<PathBuf> = generated
        .iter()
        .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone()))
        .collect();

    let mut files: Vec<PathBuf> = flake_files(exec, dir)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "nix"))
        .filter(|path| {
            let full = dir.join(path);
            !excluded.iter().any(|skip| full.starts_with(skip))
        })
        .collect();

    files.sort();

    let lock = match std::fs::read(dir.join("flake.lock")) {
        Ok(lock) => lock,
        Err(error) => {
            log::warn!("cannot read {}: {error}", dir.join("flake.lock").display());
            Vec::from(b"drakeflake-unreadable-flake-lock")
        }
    };

    let hash = files.iter().fold(fnv1a(&lock), |hash, file| {
        let content = match std::fs::read(dir.join(file)) {
            Ok(content) => content,
            Err(error) => {
                log::warn!("cannot read {}: {error}", dir.join(file).display());
                Vec::from(b"drakeflake-unreadable-file")
            }
        };

        let parts: [&[u8]; 4] = [file.as_os_str().as_encoded_bytes(), &[0], &content, &[0]];
        parts
            .iter()
            .fold(hash, |hash, bytes| fnv1a_from(hash, bytes))
    });

    (files.len(), hash)
}

fn flake_files(exec: &Context, dir: &Path) -> Vec<PathBuf> {
    let tracked = exec
        .probe(
            "git",
            [
                OsStr::new("-C"),
                dir.as_os_str(),
                OsStr::new("ls-files"),
                OsStr::new("-z"),
                OsStr::new("--cached"),
            ],
            SHORT_TIMEOUT,
        )
        .ok()
        .filter(|captured| captured.success);

    if let Some(captured) = tracked {
        return captured
            .stdout
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(PathBuf::from)
            .collect();
    }

    log::debug!("{} is not a git checkout; listing its files", dir.display());
    walk_files(dir)
}

fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut queue = vec![PathBuf::new()];

    while let Some(relative) = queue.pop() {
        let Ok(entries) = std::fs::read_dir(dir.join(&relative)) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let kind = entry.file_type();

            if files.len() >= FILE_LIMIT {
                continue;
            }

            if name == ".git" {
                continue;
            }

            let Ok(kind) = kind else {
                continue;
            };

            if kind.is_dir() {
                queue.push(relative.join(&name));
                continue;
            }

            #[expect(
                clippy::filetype_is_file,
                reason = "only regular files are hashed: a FIFO would block and symlinks stay unfollowed"
            )]
            let regular = kind.is_file();

            if regular {
                files.push(relative.join(&name));
            }
        }
    }

    files
}

#[cfg(test)]
mod tests;
