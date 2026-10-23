mod mounts;
mod walk;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

pub use mounts::{mount_points, mount_source, parse_mountinfo, root_file_system};
use serde::{Deserialize, Serialize};
use walk::Walk;

#[derive(Debug, Clone, Copy)]
pub struct ScanLimits {
    pub depth: usize,
    pub per_dir: usize,
    pub nodes: usize,
    /// Bounds the memory a directory with millions of entries can use.
    pub visit_per_dir: usize,
    pub visit_total: usize,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            depth: 8,
            per_dir: 200,
            nodes: 20_000,
            visit_per_dir: 200_000,
            visit_total: 1_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Dir,
    File,
    Link,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanNode {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    pub files: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub hidden: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub denied: bool,
}

/// Level by level so one huge subtree cannot use up the node budget.
fn trim(root: &mut ScanNode, limits: &ScanLimits) {
    let mut listed = 0_usize;
    let mut level: Vec<&mut ScanNode> = vec![root];

    for depth in 0..=limits.depth {
        let mut next = Vec::new();

        for node in level {
            if depth >= limits.depth {
                node.hidden += u64::try_from(node.children.len()).unwrap_or(u64::MAX);
                node.children.clear();
                continue;
            }

            let allowed = limits.per_dir.min(limits.nodes.saturating_sub(listed));

            if node.children.len() > allowed {
                node.children.sort_by(|one, other| {
                    other
                        .size
                        .cmp(&one.size)
                        .then_with(|| one.name.cmp(&other.name))
                });
                node.hidden += u64::try_from(node.children.len() - allowed).unwrap_or(u64::MAX);
                node.children.truncate(allowed);
            }

            listed += node.children.len();
            node.children
                .sort_by(|one, other| one.name.cmp(&other.name));
            next.extend(node.children.iter_mut());
        }

        level = next;
    }
}

pub fn scan(
    root: &Path,
    snapshot: Option<&Path>,
    mounts: &[PathBuf],
    limits: ScanLimits,
) -> Result<ScanNode, String> {
    let meta =
        fs::metadata(root).map_err(|err| format!("cannot read {}: {err}", root.display()))?;

    let relative: HashSet<PathBuf> = mounts
        .iter()
        .filter_map(|mount| mount.strip_prefix(root).ok())
        .filter(|rel| !rel.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .collect();

    let walk = Walk::new(meta.dev(), &relative, limits);

    let mut node = walk.dir(root, Path::new(""), snapshot, 0);
    trim(&mut node, &limits);
    "/".clone_into(&mut node.name);
    Ok(node)
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_zero(value: &u64) -> bool {
    *value == 0
}
