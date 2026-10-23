mod search;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use drakeflake_render::PackageAttr;
use drakeflake_store::PackageIndex;
use qtbridge::qobject;
use search::{CommandState, PackageRow};
use serde_json::{Value, json};

use crate::hub::Hub;
use crate::models::count_i32;
use crate::models::vec_model::{count_arg, top_json, vec_list_model};

#[derive(Default)]
pub struct PackageModel {
    hub: Arc<Hub>,
    index: Arc<PackageIndex>,
    set: String,
    sets: Vec<String>,
    set_indexes: HashMap<String, Arc<PackageIndex>>,
    set_error: String,
    command_mode: bool,
    command_attrs: Option<Vec<PackageAttr>>,
    command_state: CommandState,
    overview_command: String,
    overview_commands: Vec<PackageAttr>,
    installed: HashSet<PackageAttr>,
    details: HashMap<PackageAttr, Result<Value, String>>,
    pending: HashSet<PackageAttr>,
    matches: Vec<usize>,
    rows: Vec<PackageRow>,
    query: String,
    only_installed: bool,
    error: String,
    revision: i32,
}

vec_list_model!(
    PackageModel,
    PackageRow,
    rows,
    notify = state_changed,
    page = page
);

impl PackageModel {
    #[must_use = "the model is only built"]
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }
}

#[qobject(Base = QListModel)]
impl PackageModel {
    qproperty!("total", Read = total, Notify = state_changed);
    qproperty!("size", Read = size, Notify = state_changed);
    qproperty!(
        "installed_count",
        Read = installed_count,
        Notify = state_changed
    );
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("revision", Read = revision, Notify = state_changed);
    qproperty!("has_more", Read = has_more, Notify = state_changed);
    qproperty!("package_set", Read = package_set, Notify = state_changed);
    qproperty!("set_loading", Read = set_loading, Notify = state_changed);
    qproperty!("set_error", Read = set_error, Notify = state_changed);
    qproperty!("command_mode", Read = command_mode, Notify = state_changed);
    qproperty!(
        "command_state",
        Read = command_state,
        Notify = state_changed
    );
    qproperty!(
        "command_error",
        Read = command_error,
        Notify = state_changed
    );
    qproperty!(
        "overview_commands",
        Read = overview_commands,
        Notify = state_changed
    );

    pub fn total(&self) -> i32 {
        count_i32(self.matches.len())
    }

    pub fn size(&self) -> i32 {
        count_i32(self.active().len())
    }

    pub const fn revision(&self) -> i32 {
        self.revision
    }

    pub fn installed_count(&self) -> i32 {
        count_i32(self.installed.len())
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }

    pub fn package_set(&self) -> String {
        self.set.clone()
    }

    pub fn set_loading(&self) -> bool {
        !self.set.is_empty()
            && !self.set_indexes.contains_key(&self.set)
            && self.set_error.is_empty()
    }

    pub const fn command_mode(&self) -> bool {
        self.command_mode
    }

    pub fn command_state(&self) -> String {
        self.command_state.to_string()
    }

    pub fn command_error(&self) -> String {
        self.command_state.error().to_owned()
    }

    pub fn overview_commands(&self) -> Value {
        json!({ "command": self.overview_command, "attrs": self.overview_commands })
    }

    pub fn set_error(&self) -> String {
        self.set_error.clone()
    }

    pub const fn has_more(&self) -> bool {
        self.rows.len() < self.matches.len()
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().packages.take();

        if messages.is_empty() {
            return;
        }

        log::debug!("packages mailbox: {} messages", messages.len());

        let mut research = false;

        for message in messages {
            research |= self.handle(message);
        }

        if research {
            self.refresh();
        } else {
            self.bump();
        }
    }

    #[qslot]
    pub fn top(&self, query: String, count: i32) -> Value {
        top_json(self.top_matches(query.trim(), count_arg(count)))
    }

    #[qslot]
    pub fn find_overview_command(&mut self, command: String) {
        let command = command.trim().to_owned();
        if command == self.overview_command {
            return;
        }

        self.overview_commands.clear();
        self.overview_command.clone_from(&command);
        self.state_changed();

        if !command.is_empty()
            && !command.contains('/')
            && !command.contains(char::is_whitespace)
            && Hub::commands_ready()
        {
            self.hub.find_command(command);
        }
    }

    #[qslot]
    pub fn search(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            self.lookup_command();
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_command_mode(&mut self, enabled: bool) {
        if self.command_mode != enabled {
            log::debug!("search by command: {enabled}");
            self.command_mode = enabled;
            self.command_attrs = None;
            self.command_state = CommandState::Idle;
            self.lookup_command();
            self.refresh();
        }
    }

    #[qslot]
    pub fn download_command_index(&mut self) {
        log::info!("downloading the command index");
        self.command_state = CommandState::Downloading;
        self.hub.download_command_index();
        self.bump();
    }

    #[qslot]
    pub fn set_only_installed(&mut self, only: bool) {
        if self.only_installed != only {
            log::debug!("only installed packages: {only}");
            self.only_installed = only;
            self.refresh();
        }
    }

    #[qslot]
    pub fn request_details(&mut self, attr: String) {
        if attr.is_empty() {
            return;
        }

        let attr = PackageAttr::from(attr);

        if self.pending.contains(&attr) || self.details.contains_key(&attr) {
            return;
        }

        self.pending.insert(attr.clone());
        self.hub.request_package(attr);
        self.bump();
    }

    #[qslot]
    pub fn details(&self, attr: String) -> Value {
        let attr = PackageAttr::from(attr);
        let active = self.active();
        let summary = active
            .entry_by_attr(&attr)
            .or_else(|| self.index.entry_by_attr(&attr))
            .map(|info| json!({ "version": info.version, "description": info.description }));

        let (state, meta, error) = match self.details.get(&attr) {
            Some(Ok(meta)) => ("ready", meta.clone(), String::new()),
            Some(Err(error)) => ("error", Value::Null, error.clone()),
            None if self.pending.contains(&attr) => ("loading", Value::Null, String::new()),
            None => ("unknown", Value::Null, String::new()),
        };

        json!({
            "attr": attr,
            "known": summary.is_some(),
            "summary": summary,
            "installed": self.installed.contains(&attr),
            "state": state,
            "meta": meta,
            "error": error,
        })
    }

    #[qslot]
    pub fn package_sets(&self) -> Value {
        json!(self.sets)
    }

    #[qslot]
    pub fn set_package_set(&mut self, set: String) {
        if set == self.set {
            return;
        }

        if !set.is_empty() && !self.sets.contains(&set) {
            log::warn!("ignoring unknown package set {set:?}");
            return;
        }

        log::debug!("package set selected: {set:?}");

        self.set_error.clear();

        if !set.is_empty() && !self.set_indexes.contains_key(&set) {
            self.hub.request_package_set(set.clone());
        }

        self.set = set;
        self.refresh();
    }

    #[qslot]
    pub fn load_more(&mut self) {
        self.append_page();
    }

    #[qslot]
    pub fn is_known(&self, attr: String) -> bool {
        let attr = PackageAttr::from(attr);
        self.index.contains(&attr) || self.active().contains(&attr)
    }

    #[qslot]
    pub fn try_package(&self, attr: String) -> String {
        log::info!("trying package {attr}");
        self.hub
            .try_package(attr)
            .err()
            .inspect(|error| log::error!("cannot try the package: {error}"))
            .map(|error| error.to_string())
            .unwrap_or_default()
    }
}
