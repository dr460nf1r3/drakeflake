use drakeflake_file::{LIST_TYPE, list_input, listed};
use drakeflake_flake::{PersistKind, PersistPlace, PersistScope};
use drakeflake_scan::EntryKind;
use serde::Deserialize as _;
use serde::de::value::{Error as ValueError, StrDeserializer};
use serde_json::{Value, json};

use super::tree::{self, Kept};
use super::view::{self, PersistedRow, RowState};
use super::{Load, PersistenceModel, PersistenceMsg, Shown};

impl PersistenceModel {
    pub(super) fn handle(&mut self, message: PersistenceMsg) {
        match message {
            PersistenceMsg::Loaded(result) => {
                self.load = Load::Done;

                match result {
                    Ok(data) => {
                        self.error.clear();
                        self.data = data;
                    }
                    Err(error) => {
                        self.error = error;
                        self.data = None;
                    }
                }

                let storages = self
                    .data
                    .as_ref()
                    .map(|data| data.storages.as_slice())
                    .unwrap_or_default();

                if !storages.contains(&self.storage) {
                    self.storage = storages.first().cloned().unwrap_or_default();
                }
            }
            PersistenceMsg::Scanned(result) => {
                self.scanning = false;

                match result {
                    Ok(scan) => {
                        self.scan_error.clear();
                        self.scan = Some(scan);
                    }
                    Err(error) => self.scan_error = error,
                }
            }
        }
    }

    pub(super) fn rebuild(&mut self) {
        self.rows = self
            .data
            .as_ref()
            .map(|data| view::persisted_rows(data, &self.saved, &self.plan))
            .unwrap_or_default();

        let base = self.base_path().map(str::to_owned);
        let kept = Kept::new(
            self.rows
                .iter()
                .filter(|row| row.state != RowState::Remove)
                .map(|row| (row.path.as_str(), row.kind)),
        );

        let filtered = self
            .scan
            .as_ref()
            .zip(base.as_deref())
            .and_then(|(scan, at)| {
                let node = tree::find(&scan.tree, at)?;
                Some((tree::without_kept(node, at, &kept)?, at))
            });

        self.ephemeral_size = filtered.as_ref().map_or(0, |(node, _)| node.size);
        self.ephemeral_files = filtered.as_ref().map_or(0, |(node, _)| node.files);
        self.ephemeral = filtered
            .map(|(node, at)| tree::rows(&node, at, &self.expanded))
            .unwrap_or_default();
        self.state_changed();
    }

    pub(super) fn base_path(&self) -> Option<&str> {
        match self.scope {
            Shown::System => Some("/"),
            Shown::Home => self
                .data
                .as_ref()?
                .homes
                .get(&self.user)
                .map(String::as_str),
        }
    }

    pub(super) fn in_scope(&self, row: &PersistedRow) -> bool {
        match (self.scope, row.scope) {
            (Shown::System, PersistScope::System) => true,
            (Shown::Home, PersistScope::User | PersistScope::Home) => row.user == self.user,
            (Shown::System, PersistScope::User | PersistScope::Home)
            | (Shown::Home, PersistScope::System) => false,
        }
    }

    pub(super) fn place_for(
        &self,
        path: &str,
        kind: PersistKind,
    ) -> Result<(PersistPlace, String), String> {
        let data = self
            .data
            .as_ref()
            .ok_or("The impermanence module is not loaded")?;

        let home = match self.scope {
            Shown::System => None,
            Shown::Home => {
                let folder = self
                    .base_path()
                    .ok_or_else(|| format!("The home folder of {} is not known", self.user))?;

                Some((self.user.as_str(), folder))
            }
        };

        data.new_place(home, &self.storage, path, kind)
    }

    pub(super) fn edit(&self, option: &str, change: impl FnOnce(&mut Vec<String>)) -> Value {
        let Some(mut entries) = listed(&self.plan, option) else {
            return json!({
                "error": format!("{option} is set in a form this page cannot change. Edit it on the Options page.")
            });
        };

        change(&mut entries);
        json!({ "name": option, "type": LIST_TYPE, "input": list_input(&entries) })
    }
}

pub(super) fn persist_kind(kind: &str) -> Result<PersistKind, String> {
    let scanned = EntryKind::deserialize(StrDeserializer::<ValueError>::new(kind))
        .map_err(|err| format!("\"{}\" is not a kind of entry: {err}", kind.escape_debug()))?;

    Ok(match scanned {
        EntryKind::Dir => PersistKind::Dir,
        EntryKind::File | EntryKind::Link | EntryKind::Other => PersistKind::File,
    })
}

pub(super) fn push_absent(entries: &mut Vec<String>, entry: String) {
    if !entries.contains(&entry) {
        entries.push(entry);
    }
}
