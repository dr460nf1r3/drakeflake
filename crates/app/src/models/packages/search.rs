use std::fmt;
use std::sync::Arc;

use drakeflake_render::PackageAttr;
use drakeflake_store::{PackageIndex, PackageInfo};
use qtbridge::{QListModelBase as _, QModelItem};

use super::PackageModel;
use crate::hub::{CommandLookupError, PackagesMsg};
use crate::models::vec_model::match_rows;
use crate::models::{PAGE_SIZE, one_line, replace_rows};

const DESCRIPTION_WIDTH: usize = 200;

pub const COMMAND_IDLE: &str = "";
pub const COMMAND_SEARCHING: &str = "searching";
pub const COMMAND_DOWNLOADING: &str = "downloading";
pub const COMMAND_MISSING_INDEX: &str = "missing-index";
pub const COMMAND_ERROR: &str = "error";

#[derive(Clone, Default, PartialEq, Eq, QModelItem, serde::Serialize)]
pub struct PackageRow {
    pub attr: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum CommandState {
    #[default]
    Idle,
    Searching,
    Downloading,
    MissingIndex,
    Error(String),
}

impl fmt::Display for CommandState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => COMMAND_IDLE,
            Self::Searching => COMMAND_SEARCHING,
            Self::Downloading => COMMAND_DOWNLOADING,
            Self::MissingIndex => COMMAND_MISSING_INDEX,
            Self::Error(_) => COMMAND_ERROR,
        })
    }
}

impl CommandState {
    #[must_use = "the message is only read"]
    pub fn error(&self) -> &str {
        match self {
            Self::Error(message) => message,
            Self::Idle | Self::Searching | Self::Downloading | Self::MissingIndex => "",
        }
    }
}

impl PackageModel {
    fn row_for(&self, info: &PackageInfo) -> PackageRow {
        PackageRow {
            attr: info.attr.to_string(),
            version: info.version.clone(),
            description: one_line(&info.description, DESCRIPTION_WIDTH),
            installed: self.installed.contains(&info.attr),
        }
    }

    pub(super) fn active(&self) -> Arc<PackageIndex> {
        if self.set.is_empty() {
            Arc::clone(&self.index)
        } else {
            self.set_indexes.get(&self.set).cloned().unwrap_or_default()
        }
    }

    pub(super) fn top_matches(&self, query: &str, count: usize) -> (usize, Vec<PackageRow>) {
        if query.is_empty() {
            return (0, Vec::new());
        }

        let matches = self.index.search(query);
        let rows = match_rows(&matches, 0, count, |position| {
            self.index.entry(position).map(|info| self.row_for(info))
        });

        (matches.len(), rows)
    }

    pub(super) fn page(&self, skip: usize) -> Vec<PackageRow> {
        let index = self.active();

        match_rows(&self.matches, skip, PAGE_SIZE, |position| {
            index.entry(position).map(|info| self.row_for(info))
        })
    }

    pub(super) fn refresh(&mut self) {
        let index = self.active();
        let started = std::time::Instant::now();

        self.matches = if self.command_mode {
            self.command_attrs
                .iter()
                .flatten()
                .filter_map(|attr| index.position(attr))
                .collect()
        } else if self.only_installed {
            index
                .search(&self.query)
                .into_iter()
                .filter(|hit| {
                    index
                        .entry(*hit)
                        .is_some_and(|info| self.installed.contains(&info.attr))
                })
                .collect()
        } else {
            index.search(&self.query)
        };

        log::debug!(
            "packages{}: {:?} matched {} of {} in {:?}",
            if self.set.is_empty() {
                String::new()
            } else {
                format!(" ({})", self.set)
            },
            self.query,
            self.matches.len(),
            index.len(),
            started.elapsed()
        );
        let rows = self.page(0);
        replace_rows(self, rows);
        self.bump();
    }

    fn refresh_installed(&mut self) {
        for position in 0..self.rows.len() {
            let Some(row) = self.rows.get(position) else {
                continue;
            };

            let installed = self.installed.contains(&PackageAttr::from(&row.attr));
            if row.installed == installed {
                continue;
            }

            let updated = PackageRow {
                installed,
                ..row.clone()
            };

            self.set(position, updated);
        }
    }

    pub(super) fn handle(&mut self, message: PackagesMsg) -> bool {
        match message {
            PackagesMsg::Details { attr, result } => {
                self.pending.remove(&attr);
                self.details.insert(attr, result);
                false
            }
            PackagesMsg::Reset => {
                self.index = Arc::default();
                self.set.clear();
                self.sets.clear();
                self.set_indexes.clear();
                self.set_error.clear();
                self.installed.clear();
                self.details.clear();
                self.pending.clear();
                self.error.clear();
                true
            }
            PackagesMsg::Index(index) => {
                log::debug!("package index of {} packages", index.len());
                self.index = index;
                self.error.clear();
                true
            }
            PackagesMsg::IndexError(error) => {
                log::warn!("no package index: {error}");
                self.error = error;
                false
            }
            PackagesMsg::Sets(sets) => {
                self.sets = sets;
                false
            }
            PackagesMsg::Commands { command, result } => self.on_commands(&command, result),
            PackagesMsg::CommandIndexReady => {
                self.lookup_command();
                false
            }
            PackagesMsg::SetIndex { set, result } => match result {
                Ok(index) => {
                    log::debug!("package set {set}: {} packages", index.len());
                    let current = set == self.set;
                    self.set_indexes.insert(set, index);
                    current
                }
                Err(error) => {
                    log::warn!("cannot index package set {set}: {error}");
                    if set == self.set {
                        self.set_error = error;
                    }

                    false
                }
            },
            PackagesMsg::Installed(names) => {
                log::debug!("{} packages installed", names.len());
                self.installed = names;
                self.refresh_installed();
                self.only_installed
            }
        }
    }

    pub(super) fn on_commands(
        &mut self,
        command: &str,
        result: Result<Vec<PackageAttr>, CommandLookupError>,
    ) -> bool {
        if command == self.overview_command {
            self.overview_commands = result
                .as_ref()
                .map(|attrs| attrs.iter().map(PackageAttr::from).collect())
                .unwrap_or_default();
        }

        if !self.command_mode || command != self.query.trim() {
            return false;
        }

        match result {
            Ok(attrs) => {
                self.command_state = CommandState::Idle;
                self.command_attrs = Some(attrs);
            }
            Err(error) => {
                log::debug!("command lookup of {command:?} failed: {error:?}");
                self.command_state = match error {
                    CommandLookupError::MissingIndex => CommandState::MissingIndex,
                    CommandLookupError::Failed(message) => CommandState::Error(message),
                };
                self.command_attrs = Some(Vec::new());
            }
        }

        true
    }

    pub(super) fn lookup_command(&mut self) {
        if !self.command_mode {
            return;
        }

        self.command_attrs = None;
        if self.query.trim().is_empty() {
            self.command_state = CommandState::Idle;
        } else {
            self.command_state = CommandState::Searching;
            self.hub.find_command(self.query.trim().to_owned());
        }
    }
}
