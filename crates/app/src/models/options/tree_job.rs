//! Built off the GUI thread because 25 000 nodes froze the window.

use std::collections::HashMap;
use std::sync::Arc;

use drakeflake_render::OptionName;
use drakeflake_store::{OptionIndex, ValueSnapshot};
use serde_json::Value;

use super::OptionModel;
use super::rows::{OptionRow, RowValues, TreeShape, tree_shape};
use super::tree::TreeBuild;
use crate::hub::OptionsMsg;
use crate::models::count_i32;

pub(super) struct TreeInput {
    pub(super) index: Arc<OptionIndex>,
    pub(super) snapshot: Arc<ValueSnapshot>,
    pub(super) evaluated: HashMap<OptionName, Result<Value, String>>,
    pub(super) has_snapshot: bool,
}

#[derive(Clone, Default)]
pub(super) struct BuiltTree {
    pub(super) nodes: Vec<OptionRow>,
    pub(super) positions: HashMap<OptionName, usize>,
}

impl OptionModel {
    pub(super) fn start_tree_build(&mut self) {
        self.tree_generation += 1;
        let number = self.tree_generation;
        self.tree_build = TreeBuild::Building(number);

        let input = TreeInput {
            index: Arc::clone(&self.index),
            snapshot: Arc::clone(&self.snapshot),
            evaluated: self.evaluated.clone(),
            has_snapshot: self.has_snapshot,
        };

        let view = self.view;

        self.hub.spawn_background("option-tree", move |hub, _| {
            let started = std::time::Instant::now();
            let built = build_tree(&input);
            log::debug!(
                "{view:?}: {} tree nodes in {:?} (build {number})",
                built.nodes.len(),
                started.elapsed()
            );

            view.mailbox(hub.mail()).post(OptionsMsg::TreeBuilt {
                number,
                tree: Arc::new(built),
            });
        });

        self.bump();
    }
}

pub(super) fn build_tree(input: &TreeInput) -> BuiltTree {
    let values = RowValues {
        snapshot: &input.snapshot,
        evaluated: &input.evaluated,
        has_snapshot: input.has_snapshot,
    };

    let names: Vec<&str> = input
        .index
        .search("", |info| !info.templated)
        .into_iter()
        .filter_map(|position| input.index.entry(position))
        .map(|info| info.name.as_str())
        .collect();

    let nodes: Vec<OptionRow> = tree_shape(&names)
        .into_iter()
        .map(|shape| tree_row(&input.index, &values, &shape))
        .collect();

    let positions = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_option)
        .map(|(position, node)| (node.name.clone().into(), position))
        .collect();

    BuiltTree { nodes, positions }
}

fn tree_row(index: &OptionIndex, values: &RowValues<'_>, shape: &TreeShape) -> OptionRow {
    let info = shape
        .is_option
        .then(|| index.get(&OptionName::from(&shape.path)))
        .flatten();

    let row = info.map_or_else(
        || OptionRow {
            name: shape.path.clone(),
            ..OptionRow::default()
        },
        |info| values.row_for(info),
    );

    OptionRow {
        label: shape
            .path
            .rsplit('.')
            .next()
            .unwrap_or(&shape.path)
            .to_owned(),
        depth: count_i32(shape.depth),
        is_option: shape.is_option,
        has_children: shape.has_children,
        option_count: count_i32(shape.option_count),
        open: true,
        ..row
    }
}
