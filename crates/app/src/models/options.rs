mod rows;
#[cfg(test)]
mod tests;
mod tree;
mod tree_job;
mod values;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use drakeflake_render::OptionName;
use drakeflake_store::{OptionIndex, ValueSnapshot};
use qtbridge::qobject;
use rows::OptionRow;
use serde_json::{Value, json};
use tree::TreeBuild;
use values::Refresh;

use crate::hub::{HomeEpoch, Hub, Mailbox, Mailboxes, OptionsMsg};
use crate::models::count_i32;
use crate::models::vec_model::{count_arg, top_json, vec_list_model};

#[derive(serde::Serialize)]
struct NamespaceRow {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Search,
    Configured,
    Home,
    Tree,
    HomeTree,
}

impl View {
    pub const fn is_tree(self) -> bool {
        matches!(self, Self::Tree | Self::HomeTree)
    }

    pub const fn mailbox(self, mail: &Mailboxes) -> &Mailbox<OptionsMsg> {
        match self {
            Self::Search => &mail.options,
            Self::Configured => &mail.configured,
            Self::Home => &mail.home,
            Self::Tree => &mail.tree,
            Self::HomeTree => &mail.home_tree,
        }
    }
}

#[derive(Default)]
pub struct OptionModel {
    hub: Arc<Hub>,
    view: View,
    index: Arc<OptionIndex>,
    snapshot: Arc<ValueSnapshot>,
    has_snapshot: bool,
    evaluated: HashMap<OptionName, Result<Value, String>>,
    pending: RefCell<HashSet<OptionName>>,
    matches: Vec<usize>,
    rows: Vec<OptionRow>,
    query: String,
    namespace: String,
    only_user: bool,
    show_templated: bool,
    index_error: String,
    values_error: String,
    revision: i32,
    tree_nodes: Vec<OptionRow>,
    tree_positions: HashMap<OptionName, usize>,
    collapsed: HashSet<String>,
    next_rows: Vec<OptionRow>,
    tree_build: TreeBuild,
    tree_generation: u64,
    home_epoch: HomeEpoch,
    changed_values: Vec<OptionName>,
}

vec_list_model!(
    OptionModel,
    OptionRow,
    rows,
    notify = state_changed,
    next = next_rows,
    page = page
);

impl OptionModel {
    #[must_use = "the model is only built"]
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    #[must_use = "the model is only built"]
    pub fn home(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            view: View::Home,
            ..Self::default()
        }
    }

    #[must_use = "the model is only built"]
    pub fn tree(hub: Arc<Hub>, view: View) -> Self {
        Self {
            hub,
            view,
            ..Self::default()
        }
    }

    #[must_use = "the model is only built"]
    pub fn configured(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            view: View::Configured,
            only_user: true,
            ..Self::default()
        }
    }
}

#[qobject(Base = QListModel)]
impl OptionModel {
    qproperty!("total", Read = total, Notify = state_changed);
    qproperty!("size", Read = size, Notify = state_changed);
    qproperty!("revision", Read = revision, Notify = state_changed);
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("values_error", Read = values_error, Notify = state_changed);
    qproperty!("values_ready", Read = values_ready, Notify = state_changed);
    qproperty!("has_more", Read = has_more, Notify = state_changed);
    qproperty!("all_open", Read = all_open, Notify = state_changed);
    qproperty!(
        "tree_building",
        Read = tree_building,
        Notify = state_changed
    );

    pub fn total(&self) -> i32 {
        count_i32(self.matches.len())
    }

    pub fn size(&self) -> i32 {
        count_i32(self.index.len())
    }

    pub const fn revision(&self) -> i32 {
        self.revision
    }

    pub fn error(&self) -> String {
        self.index_error.clone()
    }

    pub fn values_error(&self) -> String {
        self.values_error.clone()
    }

    pub const fn values_ready(&self) -> bool {
        self.has_snapshot
    }

    pub const fn has_more(&self) -> bool {
        self.rows.len() < self.matches.len()
    }

    pub fn all_open(&self) -> bool {
        self.collapsed.is_empty()
    }

    pub const fn tree_building(&self) -> bool {
        matches!(self.tree_build, TreeBuild::Building(_))
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.view.mailbox(self.hub.mail()).take();

        if messages.is_empty() {
            return;
        }

        log::debug!(
            "{:?} options mailbox: {} messages",
            self.view,
            messages.len()
        );

        let refresh = messages
            .into_iter()
            .map(|message| self.handle(message))
            .fold(Refresh::Nothing, Refresh::max);

        match refresh {
            Refresh::Search => self.refresh(),
            Refresh::Values => self.refresh_values(),
            Refresh::Nothing => self.bump(),
        }
    }

    #[qslot]
    pub fn search(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_namespace(&mut self, namespace: String) {
        if self.namespace != namespace {
            log::debug!("{:?} options area: {namespace:?}", self.view);
            self.namespace = namespace;
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_only_user(&mut self, only_user: bool) {
        let effective = only_user || self.view == View::Configured;

        if self.only_user != effective {
            log::debug!("{:?} options only set: {effective}", self.view);
            self.only_user = effective;
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_show_templated(&mut self, show_templated: bool) {
        if self.show_templated != show_templated {
            log::debug!("{:?} options templated: {show_templated}", self.view);
            self.show_templated = show_templated;
            self.refresh();
        }
    }

    #[qslot]
    pub fn load_more(&mut self) {
        self.append_page();
    }

    #[qslot]
    pub fn top(&self, query: String, count: i32) -> Value {
        top_json(self.top_matches(query.trim(), count_arg(count)))
    }

    #[qslot]
    pub fn undocumented(&self) -> Value {
        let names: Vec<&str> = self
            .index
            .search("", |info| {
                info.description == drakeflake_flake::UNDOCUMENTED
            })
            .into_iter()
            .filter_map(|position| self.index.entry(position))
            .map(|info| info.name.as_str())
            .collect();

        json!(names)
    }

    #[qslot]
    pub fn namespaces(&self) -> Value {
        let view = self.view;
        namespace_rows(
            self.index
                .namespaces_by(|info| Self::namespace_of(view, info)),
        )
    }

    #[qslot]
    pub fn open_tree(&mut self) {
        self.show_first_time();
    }

    #[qslot]
    pub fn toggle_folder(&mut self, path: String) {
        if !self.collapsed.remove(&path) {
            self.collapsed.insert(path);
        }

        self.show_tree();
    }

    #[qslot]
    pub fn collapse_all(&mut self) {
        self.collapsed = self.top_folders().collect();
        self.show_tree();
    }

    #[qslot]
    pub fn expand_all(&mut self) {
        self.collapsed.clear();
        self.show_tree();
    }

    #[qslot]
    pub fn details(&self, name: String) -> Value {
        self.details_value(&name.into())
    }

    #[qslot]
    pub fn request_value(&mut self, name: String) {
        let name = OptionName::from(name);

        if name.is_empty()
            || self.pending.borrow().contains(&name)
            || self.index.get(&name).is_none()
        {
            return;
        }

        self.evaluated.remove(&name);
        self.pending.borrow_mut().insert(name.clone());
        self.hub.request_value(name);
        self.bump();
    }

    #[qslot]
    pub fn open_in_editor(&self, file: String, line: i32) {
        crate::hub::editor::open_and_report(&self.hub, &file, line);
    }
}

fn namespace_rows(counts: Vec<(String, usize)>) -> Value {
    let rows: Vec<NamespaceRow> = counts
        .into_iter()
        .map(|(name, count)| NamespaceRow { name, count })
        .collect();

    json!(rows)
}
