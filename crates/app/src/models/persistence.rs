mod state;
mod tree;
mod view;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use drakeflake_flake::Persistence;
use drakeflake_plan::ManagedState;
use qtbridge::qobject;
use serde::Deserialize as _;
use serde_json::{Value, json};
use tree::EphemeralRow;
use view::PersistedRow;

use crate::hub::Hub;
use crate::hub::persistence::{PersistenceMsg, Scan};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Load {
    #[default]
    Idle,
    Loading,
    Done,
    Reloading,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Shown {
    #[default]
    System,
    Home,
}

#[derive(Default)]
pub struct PersistenceModel {
    hub: Arc<Hub>,
    load: Load,
    error: String,
    data: Option<Persistence>,
    saved: ManagedState,
    plan: ManagedState,
    scope: Shown,
    user: String,
    storage: String,
    scanning: bool,
    scan_error: String,
    scan: Option<Scan>,
    expanded: BTreeSet<String>,
    rows: Vec<PersistedRow>,
    ephemeral: Vec<EphemeralRow>,
    ephemeral_size: u64,
    ephemeral_files: u64,
    snapshot_offered: bool,
}

impl Load {
    const fn is_loading(self) -> bool {
        match self {
            Self::Loading | Self::Reloading => true,
            Self::Idle | Self::Done => false,
        }
    }

    const fn is_loaded(self) -> bool {
        match self {
            Self::Done | Self::Reloading => true,
            Self::Idle | Self::Loading => false,
        }
    }

    const fn started(self) -> Self {
        match self {
            Self::Idle | Self::Loading => Self::Loading,
            Self::Done | Self::Reloading => Self::Reloading,
        }
    }
}

impl Shown {
    fn parse(scope: &str) -> Self {
        if scope == "home" {
            Self::Home
        } else {
            Self::System
        }
    }
}

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::System => "system",
            Self::Home => "home",
        })
    }
}

impl PersistenceModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            snapshot_offered: crate::hub::persistence::snapshot_offered(),
            ..Self::default()
        }
    }
}

#[qobject]
impl PersistenceModel {
    qproperty!("loading", Read = loading, Notify = state_changed);
    qproperty!("loaded", Read = loaded, Notify = state_changed);
    qproperty!("available", Read = available, Notify = state_changed);
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("storages", Read = storages, Notify = state_changed);
    qproperty!("storage", Read = storage, Notify = state_changed);
    qproperty!("scope", Read = scope, Notify = state_changed);
    qproperty!("home", Read = home, Notify = state_changed);
    qproperty!("persisted", Read = persisted, Notify = state_changed);
    qproperty!("scanning", Read = scanning, Notify = state_changed);
    qproperty!("scanned", Read = scanned, Notify = state_changed);
    qproperty!("scan_error", Read = scan_error, Notify = state_changed);
    qproperty!("file_system", Read = file_system, Notify = state_changed);
    qproperty!("compared", Read = compared, Notify = state_changed);
    qproperty!(
        "snapshot_offered",
        Read = snapshot_offered,
        Notify = state_changed
    );
    qproperty!("ephemeral", Read = ephemeral, Notify = state_changed);
    qproperty!(
        "ephemeral_total",
        Read = ephemeral_total,
        Notify = state_changed
    );

    pub const fn loading(&self) -> bool {
        self.load.is_loading()
    }

    pub const fn loaded(&self) -> bool {
        self.load.is_loaded()
    }

    pub const fn available(&self) -> bool {
        self.data.is_some()
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }

    pub fn storages(&self) -> Value {
        json!(
            self.data
                .as_ref()
                .map(|data| data.storages.as_slice())
                .unwrap_or_default()
        )
    }

    pub fn storage(&self) -> String {
        self.storage.clone()
    }

    pub fn scope(&self) -> String {
        self.scope.to_string()
    }

    pub fn home(&self) -> String {
        match self.scope {
            Shown::System => String::new(),
            Shown::Home => self.base_path().unwrap_or_default().to_owned(),
        }
    }

    pub fn persisted(&self) -> Value {
        let rows: Vec<&PersistedRow> = self.rows.iter().filter(|row| self.in_scope(row)).collect();
        json!(rows)
    }

    pub const fn scanning(&self) -> bool {
        self.scanning
    }

    pub const fn scanned(&self) -> bool {
        self.scan.is_some()
    }

    pub fn scan_error(&self) -> String {
        self.scan_error.clone()
    }

    pub fn file_system(&self) -> String {
        self.scan
            .as_ref()
            .and_then(|scan| scan.file_system.clone())
            .unwrap_or_default()
    }

    pub const fn snapshot_offered(&self) -> bool {
        self.snapshot_offered
    }

    pub fn compared(&self) -> bool {
        self.scan.as_ref().is_some_and(|scan| scan.snapshot)
    }

    pub fn ephemeral(&self) -> Value {
        json!(self.ephemeral)
    }

    pub fn ephemeral_total(&self) -> Value {
        json!({ "size": self.ephemeral_size, "files": self.ephemeral_files })
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().persistence.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.rebuild();
    }

    #[qslot]
    pub fn load(&mut self) {
        self.load = self.load.started();
        self.hub.load_persistence();
        self.state_changed();
    }

    #[qslot]
    pub fn scan(&mut self, subvolume: String) {
        if self.scanning {
            return;
        }

        let name = subvolume.trim();

        self.scanning = true;
        self.hub
            .scan_ephemeral((!name.is_empty()).then(|| name.to_owned()));
        self.state_changed();
    }

    #[qslot]
    pub fn set_scope(&mut self, scope: String, user: String) {
        self.scope = Shown::parse(&scope);
        self.user = user;
        self.expanded.clear();
        self.rebuild();
    }

    #[qslot]
    pub fn set_storage(&mut self, storage: String) {
        self.storage = storage;
        self.state_changed();
    }

    #[qslot]
    pub fn set_changes(&mut self, saved: Value, plan: Value) {
        let states = ManagedState::deserialize(&saved)
            .and_then(|saved_state| Ok((saved_state, ManagedState::deserialize(&plan)?)));

        match states {
            Ok((saved_state, plan_state)) => {
                self.saved = saved_state;
                self.plan = plan_state;
                self.rebuild();
            }
            // An empty state in its place would drop the other staged entries on the next edit.
            Err(error) => log::warn!("keeping the last staged changes: {error}"),
        }
    }

    #[qslot]
    pub fn toggle(&mut self, path: String) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }

        self.rebuild();
    }

    #[qslot]
    pub fn persist(&self, path: String, kind: String) -> Value {
        let placed = state::persist_kind(&kind).and_then(|parsed| self.place_for(&path, parsed));

        match placed {
            Ok((place, entry)) => self.edit(&place.option(), |entries| {
                state::push_absent(entries, entry);
            }),
            Err(error) => json!({ "error": error }),
        }
    }

    #[qslot]
    pub fn keep(&self, option: String, entry: String) -> Value {
        self.edit(&option, |entries| state::push_absent(entries, entry))
    }

    #[qslot]
    pub fn unpersist(&self, option: String, entry: String) -> Value {
        self.edit(&option, |entries| entries.retain(|kept| *kept != entry))
    }
}
