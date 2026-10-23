use std::cell::Cell;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, Metadata};
use std::io::{self, ErrorKind};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use super::{EntryKind, ScanLimits, ScanNode};

/// Kernel file systems and the store: never part of the root to wipe.
const SKIPPED: [&str; 5] = ["dev", "nix", "proc", "run", "sys"];

/// Hard stop for traversal: display depth only gates listing.
const MAX_DEPTH: usize = 64;

pub struct Walk<'a> {
    device: u64,
    mounts: &'a HashSet<PathBuf>,
    limits: ScanLimits,
    visited: Cell<usize>,
}

impl<'a> Walk<'a> {
    pub const fn new(device: u64, mounts: &'a HashSet<PathBuf>, limits: ScanLimits) -> Self {
        Self {
            device,
            mounts,
            limits,
            visited: Cell::new(0),
        }
    }

    pub fn dir(&self, path: &Path, rel: &Path, snapshot: Option<&Path>, depth: usize) -> ScanNode {
        let mut node = node(EntryKind::Dir, 0, 0);

        if depth > MAX_DEPTH {
            node.hidden += 1;
            return node;
        }

        let Ok(entries) = fs::read_dir(path) else {
            node.denied = true;
            return node;
        };

        let listed = depth < self.limits.depth;

        for (seen, entry) in entries.enumerate() {
            // The entry just read is the proof that something was left out.
            if seen >= self.limits.visit_per_dir || self.visited.get() >= self.limits.visit_total {
                node.hidden += 1;
                break;
            }

            self.visited.set(self.visited.get() + 1);

            match entry.and_then(|found| self.entry(&found, rel, snapshot, depth)) {
                Ok(Some((name, child))) => {
                    node.size += child.size;
                    node.files += child.files;

                    match name.into_string() {
                        Ok(name) if listed => node.children.push(ScanNode { name, ..child }),
                        Ok(_) | Err(_) => node.hidden += 1,
                    }
                }
                Ok(None) => {}
                Err(error) => node.hidden += u64::from(error.kind() != ErrorKind::NotFound),
            }
        }

        node
    }

    fn entry(
        &self,
        entry: &fs::DirEntry,
        rel: &Path,
        snapshot: Option<&Path>,
        depth: usize,
    ) -> io::Result<Option<(OsString, ScanNode)>> {
        let name = entry.file_name();

        if depth == 0 && SKIPPED.iter().any(|skip| name == *skip) {
            return Ok(None);
        }

        let child_rel = rel.join(&name);
        let meta = fs::symlink_metadata(entry.path())?;

        if meta.dev() != self.device || self.mounts.contains(&child_rel) {
            return Ok(None);
        }

        let in_snapshot = snapshot.map(|dir| dir.join(&name));
        let kept = in_snapshot
            .as_deref()
            .and_then(|other| fs::symlink_metadata(other).ok());

        if meta.is_symlink() {
            return Ok(kept.is_none().then(|| (name, leaf(&meta))));
        }

        if !meta.is_dir() {
            return Ok(kept.is_none().then(|| (name, leaf(&meta))));
        }

        let below = kept.filter(Metadata::is_dir).and(in_snapshot.as_deref());
        let child = self.dir(&entry.path(), &child_rel, below, depth + 1);
        let unchanged = below.is_some()
            && child.files == 0
            && child.children.is_empty()
            && child.hidden == 0
            && !child.denied;

        Ok((!unchanged).then_some((name, child)))
    }
}

const fn node(kind: EntryKind, size: u64, files: u64) -> ScanNode {
    ScanNode {
        name: String::new(),
        kind,
        size,
        files,
        children: Vec::new(),
        hidden: 0,
        denied: false,
    }
}

fn leaf(meta: &Metadata) -> ScanNode {
    if meta.is_file() {
        node(EntryKind::File, meta.len(), 1)
    } else if meta.is_symlink() {
        node(EntryKind::Link, 0, 1)
    } else {
        node(EntryKind::Other, 0, 1)
    }
}
