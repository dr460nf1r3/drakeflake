use drakeflake_file::Staged;
use drakeflake_render::OptionName;
use qtbridge::QModelItem;
use serde::Serialize;
use serde_json::{Value, json};

use super::ChangeModel;
use super::check::CheckState;
use crate::models::{one_line, replace_rows};

const VALUE_WIDTH: usize = 80;

#[derive(Serialize)]
struct ManagedRow<'a> {
    kind: &'static str,
    name: &'a str,
    value: String,
}

#[derive(Clone, Default, PartialEq, Eq, QModelItem)]
pub struct ChangeRow {
    pub key: String,
    pub kind: String,
    pub name: String,
    pub change_type: String,
    pub before: String,
    pub after: String,
    pub check: String,
    pub check_message: String,
    pub priority: String,
    pub mode: String,
    pub host: String,
}

impl ChangeModel {
    pub(super) fn rebuild_rows(&mut self) {
        self.persist();
        let rows = self.build_rows();

        log::debug!("changes list: {} rows", rows.len());
        replace_rows(self, rows);

        crate::macos::window::set_dock_badge(self.row_count());
        self.bump();
    }

    pub(super) fn build_rows(&self) -> Vec<ChangeRow> {
        let options = self
            .changes
            .options()
            .iter()
            .filter_map(|(name, staged)| self.option_row(name, staged));

        let packages = self.changes.packages().iter().filter_map(|(attr, staged)| {
            let action = match staged {
                Staged::Add => "add",
                Staged::Remove => "remove",
                Staged::Set(_) | Staged::Reset => return None,
            };

            Some(ChangeRow {
                key: format!("package:{attr}"),
                kind: "package".to_owned(),
                name: attr.to_string(),
                change_type: action.to_owned(),
                ..ChangeRow::default()
            })
        });

        options.chain(packages).collect()
    }

    fn option_row(&self, name: &OptionName, staged: &Staged) -> Option<ChangeRow> {
        let (action, after, option) = match staged {
            Staged::Set(option) => ("set", one_line(&option.input, VALUE_WIDTH), Some(option)),
            Staged::Reset => ("reset", String::new(), None),
            Staged::Add | Staged::Remove => return None,
        };

        let check = self.check_of(name);

        Some(ChangeRow {
            key: format!("option:{name}"),
            kind: "option".to_owned(),
            name: name.to_string(),
            change_type: action.to_owned(),
            before: self
                .managed
                .options
                .get(name)
                .map(|saved| one_line(&saved.input, VALUE_WIDTH))
                .unwrap_or_default(),
            after,
            check: check.map(ToString::to_string).unwrap_or_default(),
            check_message: check.map(CheckState::message).unwrap_or_default(),
            priority: option
                .and_then(|option| option.priority)
                .map(|priority| priority.to_string())
                .unwrap_or_default(),
            mode: option
                .map(|option| option.mode.to_string())
                .unwrap_or_default(),
            host: option
                .and_then(|option| option.host.clone())
                .map(|host| host.to_string())
                .unwrap_or_default(),
        })
    }

    pub(super) fn managed_rows(&self) -> Value {
        let options = self
            .managed
            .options
            .iter()
            .map(|(name, option)| ManagedRow {
                kind: "option",
                name,
                value: one_line(&option.input, VALUE_WIDTH),
            });

        let packages = self.managed.packages.iter().map(|attr| ManagedRow {
            kind: "package",
            name: attr,
            value: String::new(),
        });

        let rows: Vec<ManagedRow<'_>> = options.chain(packages).collect();

        json!(rows)
    }
}
