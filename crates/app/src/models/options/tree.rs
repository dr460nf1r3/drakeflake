//! Tree View rows, built in Rust: converting the whole tree (about 25 000 rows) to JavaScript on every fold would be too slow.

use std::collections::HashSet;
use std::sync::Arc;

use qtbridge::QListModelBase as _;

use super::OptionModel;
use super::rows::OptionRow;
use super::tree_job::BuiltTree;

/// Built only once its page shows it, so startup does not pay for it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum TreeBuild {
    #[default]
    Unseen,
    Stale,
    Building(u64),
    Shown,
}

impl OptionModel {
    pub(super) fn refresh_tree(&mut self) {
        match self.tree_build {
            TreeBuild::Shown | TreeBuild::Building(_) => self.start_tree_build(),
            TreeBuild::Unseen | TreeBuild::Stale => self.tree_build = TreeBuild::Stale,
        }
    }

    pub(super) fn show_first_time(&mut self) {
        if !self.view.is_tree() {
            return;
        }

        match self.tree_build {
            TreeBuild::Stale => self.start_tree_build(),
            TreeBuild::Unseen => self.tree_build = TreeBuild::Shown,
            TreeBuild::Building(_) | TreeBuild::Shown => {}
        }
    }

    /// Values evaluated while the build ran are missing from it.
    pub(super) fn apply_built_tree(&mut self, number: u64, built: Arc<BuiltTree>) {
        if self.tree_build != TreeBuild::Building(number) {
            log::debug!("{:?}: tree build {number} superseded", self.view);
            return;
        }

        let built = Arc::try_unwrap(built).unwrap_or_else(|shared| (*shared).clone());
        self.tree_build = TreeBuild::Shown;
        self.tree_nodes = built.nodes;
        self.tree_positions = built.positions;
        self.changed_values = self.evaluated.keys().cloned().collect();
        self.show_tree();
        self.update_tree_values();
    }

    pub(super) fn update_tree_values(&mut self) {
        for name in std::mem::take(&mut self.changed_values) {
            let Some(&position) = self.tree_positions.get(&name) else {
                continue;
            };

            let Some(info) = self.index.get(&name) else {
                continue;
            };

            let value = self.value_text(info);
            let user_set = self.snapshot.is_user_set(&name);
            let Some(node) = self.tree_nodes.get_mut(position) else {
                continue;
            };

            node.value = value;
            node.user_set = user_set;
            let node = node.clone();

            let shown = self.rows.iter().position(|row| row.name == name);
            if let Some((row, open)) = shown.and_then(|row| Some((row, self.rows.get(row)?.open))) {
                self.set(row, OptionRow { open, ..node });
            }
        }

        self.bump();
    }

    pub(super) fn show_tree(&mut self) {
        if !self.view.is_tree() {
            return;
        }

        // `reset_unnotified` swaps these in between the reset signals.
        self.next_rows = visible_nodes(&self.tree_nodes, &self.collapsed);
        self.reset();
        self.bump();
    }

    pub(super) fn top_folders(&self) -> impl Iterator<Item = String> + '_ {
        self.tree_nodes
            .iter()
            .filter(|node| node.depth == 0_i32 && node.has_children)
            .map(|node| node.name.clone())
    }
}

pub(super) fn visible_nodes(nodes: &[OptionRow], collapsed: &HashSet<String>) -> Vec<OptionRow> {
    let mut hidden_below: Option<&str> = None;
    let mut visible = Vec::new();

    for node in nodes {
        if let Some(folder) = hidden_below
            && node
                .name
                .strip_prefix(folder)
                .is_some_and(|rest| rest.starts_with('.'))
        {
            continue;
        }

        hidden_below = None;
        let open = !collapsed.contains(&node.name);
        if node.has_children && !open {
            hidden_below = Some(&node.name);
        }

        visible.push(OptionRow {
            open,
            ..node.clone()
        });
    }

    visible
}
