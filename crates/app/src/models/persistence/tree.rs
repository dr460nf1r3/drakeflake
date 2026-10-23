use std::collections::{BTreeSet, HashSet};

use drakeflake_flake::PersistKind;
use drakeflake_scan::{EntryKind, ScanNode};
use serde::Serialize;

#[derive(Debug, Default)]
pub struct Kept<'a> {
    dirs: HashSet<&'a str>,
    files: HashSet<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EphemeralRow {
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    pub files: u64,
    pub depth: usize,
    pub expandable: bool,
    pub expanded: bool,
    pub hidden: u64,
    pub denied: bool,
}

impl<'a> Kept<'a> {
    pub fn new(paths: impl IntoIterator<Item = (&'a str, PersistKind)>) -> Self {
        let mut kept = Self::default();

        for (path, kind) in paths {
            let set = match kind {
                PersistKind::Dir => &mut kept.dirs,
                PersistKind::File => &mut kept.files,
            };

            set.insert(path);
        }

        kept
    }

    pub fn contains(&self, path: &str) -> bool {
        let mut ancestors = std::iter::successors(Some(path), |at| {
            let cut = at.rfind('/')?;

            at.get(..cut)
        });

        self.files.contains(path) || ancestors.any(|at| self.dirs.contains(at))
    }
}

pub fn child_path(parent: &str, name: &str) -> String {
    if parent.ends_with('/') {
        format!("{parent}{name}")
    } else {
        format!("{parent}/{name}")
    }
}

pub fn without_kept(node: &ScanNode, path: &str, kept: &Kept<'_>) -> Option<ScanNode> {
    if kept.contains(path) {
        return None;
    }

    let mut out = ScanNode {
        name: node.name.clone(),
        kind: node.kind,
        size: node.size,
        files: node.files,
        children: Vec::with_capacity(node.children.len()),
        hidden: node.hidden,
        denied: node.denied,
    };

    for child in &node.children {
        let remaining = without_kept(child, &child_path(path, &child.name), kept);
        let (size, files) = remaining
            .as_ref()
            .map_or((0, 0), |left| (left.size, left.files));

        out.size = out.size.saturating_sub(child.size.saturating_sub(size));
        out.files = out.files.saturating_sub(child.files.saturating_sub(files));
        out.children.extend(remaining);
    }

    let emptied = !node.children.is_empty()
        && out.children.is_empty()
        && out.files == 0
        && node.hidden == 0
        && !node.denied;

    (!emptied).then_some(out)
}

pub fn find<'a>(root: &'a ScanNode, path: &str) -> Option<&'a ScanNode> {
    path.split('/')
        .filter(|part| !part.is_empty())
        .try_fold(root, |node, part| {
            node.children.iter().find(|child| child.name == part)
        })
}

pub fn rows(base: &ScanNode, base_path: &str, expanded: &BTreeSet<String>) -> Vec<EphemeralRow> {
    let mut rows = Vec::new();
    push_rows(&mut rows, base, base_path, 0, expanded);
    rows
}

fn push_rows(
    rows: &mut Vec<EphemeralRow>,
    node: &ScanNode,
    path: &str,
    depth: usize,
    expanded: &BTreeSet<String>,
) {
    for child in &node.children {
        let at = child_path(path, &child.name);
        let open = expanded.contains(&at);

        rows.push(EphemeralRow {
            name: child.name.clone(),
            kind: child.kind,
            size: child.size,
            files: child.files,
            depth,
            expandable: !child.children.is_empty(),
            expanded: open,
            hidden: child.hidden,
            denied: child.denied,
            path: at.clone(),
        });

        if open {
            push_rows(rows, child, &at, depth + 1, expanded);
        }
    }
}
