use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use drakeflake_render::OptionName;
use drakeflake_store::{OptionInfo, ValueSnapshot};
use drakeflake_validate::{display_value, parse_type};
use qtbridge::{QListModelBase as _, QModelItem};
use serde_json::Value;

use super::{OptionModel, View};
use crate::models::vec_model::match_rows;
use crate::models::{PAGE_SIZE, one_line, replace_rows};

const VALUE_WIDTH: usize = 60;
const SUMMARY_WIDTH: usize = 160;

#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is a role of the QML list models"
)]
#[derive(Clone, Default, PartialEq, Eq, QModelItem, serde::Serialize)]
pub struct OptionRow {
    pub name: String,
    pub summary: String,
    pub type_name: String,
    pub kind: String,
    pub value: String,
    pub user_set: bool,
    pub read_only: bool,
    pub defined_in: String,
    pub label: String,
    pub depth: i32,
    pub is_option: bool,
    pub has_children: bool,
    pub option_count: i32,
    pub open: bool,
}

pub(super) struct TreeShape {
    pub(super) path: String,
    pub(super) depth: usize,
    pub(super) is_option: bool,
    pub(super) has_children: bool,
    pub(super) option_count: usize,
}

pub(super) struct RowValues<'a> {
    pub(super) snapshot: &'a ValueSnapshot,
    pub(super) evaluated: &'a HashMap<OptionName, Result<Value, String>>,
    pub(super) has_snapshot: bool,
}

impl RowValues<'_> {
    fn current_value(&self, name: &OptionName) -> Option<&Value> {
        match self.evaluated.get(name) {
            Some(Ok(value)) => Some(value),
            Some(Err(_)) => None,
            None => self.snapshot.value(name),
        }
    }

    pub(super) fn value_text(&self, info: &OptionInfo) -> String {
        let undefined = self.has_snapshot
            && !self.snapshot.is_user_set(&info.name)
            && !self.evaluated.contains_key(&info.name);

        match self.current_value(&info.name) {
            Some(value) if !is_error(value) => one_line(&display_value(value), VALUE_WIDTH),
            None if undefined => info
                .default_text
                .as_deref()
                .map(|text| one_line(text, VALUE_WIDTH))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    pub(super) fn row_for(&self, info: &OptionInfo) -> OptionRow {
        let value = self.value_text(info);

        OptionRow {
            name: info.name.to_string(),
            summary: one_line(&info.summary(), SUMMARY_WIDTH),
            type_name: info.type_name.clone(),
            kind: parse_type(&info.type_name).kind().to_owned(),
            value,
            user_set: self.snapshot.is_user_set(&info.name),
            read_only: info.read_only || info.templated,
            defined_in: match self.snapshot.user_files(&info.name) {
                [] => String::new(),
                [only] => short_path(only).to_owned(),
                [first, rest @ ..] => format!("{} (+{})", short_path(first), rest.len()),
            },
            ..OptionRow::default()
        }
    }
}

impl OptionModel {
    pub(super) fn namespace_of(view: View, info: &OptionInfo) -> &str {
        match view {
            View::Home | View::HomeTree => info.name.split('.').nth(3).unwrap_or_default(),
            View::Search | View::Configured | View::Tree => info.namespace(),
        }
    }

    pub(super) fn row_values(&self) -> RowValues<'_> {
        RowValues {
            snapshot: &self.snapshot,
            evaluated: &self.evaluated,
            has_snapshot: self.has_snapshot,
        }
    }

    pub(super) fn value_text(&self, info: &OptionInfo) -> String {
        self.row_values().value_text(info)
    }

    pub(super) fn row_for(&self, info: &OptionInfo) -> OptionRow {
        self.row_values().row_for(info)
    }

    pub(super) fn top_matches(&self, query: &str, count: usize) -> (usize, Vec<OptionRow>) {
        if query.is_empty() {
            return (0, Vec::new());
        }

        let only_user = self.view == View::Configured;
        let matches = self.index.search(query, |info| {
            !info.templated && (!only_user || self.snapshot.is_user_set(&info.name))
        });

        let rows = match_rows(&matches, 0, count, |position| {
            self.index.entry(position).map(|info| self.row_for(info))
        });

        (matches.len(), rows)
    }

    pub(super) fn page(&self, skip: usize) -> Vec<OptionRow> {
        match_rows(&self.matches, skip, PAGE_SIZE, |position| {
            self.index.entry(position).map(|info| self.row_for(info))
        })
    }

    pub(super) fn refresh(&mut self) {
        if self.view.is_tree() {
            self.refresh_tree();
            return;
        }

        let index = Arc::clone(&self.index);
        let snapshot = Arc::clone(&self.snapshot);
        let namespace = self.namespace.clone();
        let only_user = self.only_user;
        let show_templated = self.show_templated;
        let view = self.view;
        let started = std::time::Instant::now();

        self.matches = index.search(&self.query, |info| {
            (show_templated || !info.templated)
                && (namespace.is_empty() || Self::namespace_of(view, info) == namespace)
                && (!only_user || snapshot.is_user_set(&info.name))
        });

        log::debug!(
            "{view:?} options: {:?} matched {} of {} in {:?}",
            self.query,
            self.matches.len(),
            index.len(),
            started.elapsed()
        );
        let rows = self.page(0);
        replace_rows(self, rows);
        self.bump();
    }

    pub(super) fn refresh_values(&mut self) {
        if self.view.is_tree() {
            self.update_tree_values();
            return;
        }

        let updated: Vec<OptionRow> = self
            .rows
            .iter()
            .map(|row| {
                self.index
                    .get(&OptionName::from(&row.name))
                    .map_or_else(|| row.clone(), |info| self.row_for(info))
            })
            .collect();

        for (position, row) in updated.into_iter().enumerate() {
            if self.rows.get(position) != Some(&row) {
                self.set(position, row);
            }
        }

        self.bump();
    }
}

pub(super) fn short_path(path: &str) -> &str {
    path.strip_prefix("/nix/store/")
        .and_then(|rest| rest.split_once('/'))
        .map_or(path, |(_, relative)| relative)
}

pub(super) fn tree_shape(names: &[&str]) -> Vec<TreeShape> {
    let known: HashSet<&str> = names.iter().copied().collect();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut kids: HashSet<&str> = HashSet::new();
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    for name in names {
        for (index, _) in name.match_indices('.') {
            let Some(folder) = name.get(..index) else {
                continue;
            };

            *counts.entry(folder).or_default() += 1;
            kids.insert(folder);
            paths.insert(folder);
        }

        *counts.entry(name).or_default() += 1;
        paths.insert(name);
    }

    paths
        .into_iter()
        .map(|path| TreeShape {
            path: path.to_owned(),
            depth: path.matches('.').count(),
            is_option: known.contains(path),
            has_children: kids.contains(path),
            option_count: counts.get(path).copied().unwrap_or(0),
        })
        .collect()
}

pub(super) fn is_error(value: &Value) -> bool {
    value.get("_drakeflakeError").is_some()
}
