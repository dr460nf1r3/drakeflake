mod rows;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use drakeflake_functions::{Filter, FunctionIndex, TypeKind};
use qtbridge::qobject;
use rows::FunctionRow;
use serde_json::{Value, json};

use crate::hub::{FunctionsMsg, Hub};
use crate::models::vec_model::{count_arg, match_rows, top_json, vec_list_model};
use crate::models::{PAGE_SIZE, count_i32, replace_rows};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum LoadState {
    #[default]
    Idle,
    Loading,
    Ready,
    Missing,
    Failed,
}

#[derive(Default)]
pub struct FunctionModel {
    hub: Arc<Hub>,
    index: Arc<FunctionIndex>,
    load_state: LoadState,
    error: String,
    query: String,
    filter: Filter,
    matches: Vec<usize>,
    rows: Vec<FunctionRow>,
    revision: i32,
}

vec_list_model!(
    FunctionModel,
    FunctionRow,
    rows,
    notify = state_changed,
    page = page
);

impl FunctionModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    fn handle(&mut self, message: FunctionsMsg) {
        match message {
            FunctionsMsg::Loaded(index) => {
                self.index = index;
                self.load_state = LoadState::Ready;
                self.error.clear();
            }
            FunctionsMsg::Missing(path) => {
                self.load_state = LoadState::Missing;
                self.error = path;
            }
            FunctionsMsg::Failed(error) => {
                self.load_state = LoadState::Failed;
                self.error = error;
            }
        }
    }

    fn browsing(&self) -> bool {
        !self.query.trim().is_empty() || !self.filter.is_empty()
    }

    fn refresh(&mut self) {
        let started = std::time::Instant::now();

        self.matches = if self.browsing() {
            self.index.search(&self.query, &self.filter)
        } else {
            Vec::new()
        };
        log::debug!(
            "functions: {:?} in {:?} matched {} of {} in {:?}",
            self.query,
            self.filter,
            self.matches.len(),
            self.index.len(),
            started.elapsed()
        );

        let rows = self.page(0);
        replace_rows(self, rows);
        self.bump();
    }

    fn page(&self, skip: usize) -> Vec<FunctionRow> {
        match_rows(&self.matches, skip, PAGE_SIZE, |position| {
            self.row(position)
        })
    }

    fn top_matches(&self, query: &str, count: usize) -> (usize, Vec<FunctionRow>) {
        if query.is_empty() {
            return (0, Vec::new());
        }

        let matches = self.index.search(query, &Filter::default());
        let rows = match_rows(&matches, 0, count, |position| self.row(position));

        (matches.len(), rows)
    }

    fn set_kind(current: &mut Option<TypeKind>, name: &str) -> bool {
        let kind = name.parse().ok();
        let changed = *current != kind;
        *current = kind;
        changed
    }
}

#[qobject(Base = QListModel)]
impl FunctionModel {
    qproperty!("state", Read = state, Notify = state_changed);
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("size", Read = size, Notify = state_changed);
    qproperty!("total", Read = total, Notify = state_changed);
    qproperty!("has_more", Read = has_more, Notify = state_changed);
    qproperty!("browsing", Read = is_browsing, Notify = state_changed);
    qproperty!("revision", Read = revision, Notify = state_changed);
    qproperty!("query", Read = query, Notify = state_changed);
    qproperty!("namespace", Read = namespace, Notify = state_changed);
    qproperty!("takes", Read = takes, Notify = state_changed);
    qproperty!("returns", Read = returns, Notify = state_changed);
    qproperty!("source", Read = source, Notify = state_changed);
    qproperty!("namespaces", Read = namespaces, Notify = state_changed);

    pub fn state(&self) -> String {
        match self.load_state {
            LoadState::Idle => "idle",
            LoadState::Loading => "loading",
            LoadState::Ready => "ready",
            LoadState::Missing => "missing",
            LoadState::Failed => "error",
        }
        .to_owned()
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }

    pub fn size(&self) -> i32 {
        count_i32(self.index.len())
    }

    pub const fn revision(&self) -> i32 {
        self.revision
    }

    pub fn total(&self) -> i32 {
        count_i32(self.matches.len())
    }

    pub const fn has_more(&self) -> bool {
        self.rows.len() < self.matches.len()
    }

    pub fn is_browsing(&self) -> bool {
        self.browsing()
    }

    pub fn query(&self) -> String {
        self.query.clone()
    }

    pub fn namespace(&self) -> String {
        self.filter.namespace.clone()
    }

    pub fn takes(&self) -> String {
        self.filter
            .takes
            .map_or_else(String::new, |kind| kind.to_string())
    }

    pub fn returns(&self) -> String {
        self.filter
            .returns
            .map_or_else(String::new, |kind| kind.to_string())
    }

    pub fn source(&self) -> Value {
        rows::source_json(self.index.source())
    }

    pub fn namespaces(&self) -> Value {
        json!(self.index.namespaces())
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().functions.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.refresh();
    }

    #[qslot]
    pub fn load(&mut self) {
        if self.load_state == LoadState::Idle {
            self.load_state = LoadState::Loading;
            self.hub.load_functions();
            self.bump();
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
        if self.filter.namespace != namespace {
            self.filter.namespace = namespace;
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_takes(&mut self, kind: String) {
        if Self::set_kind(&mut self.filter.takes, &kind) {
            self.refresh();
        }
    }

    #[qslot]
    pub fn set_returns(&mut self, kind: String) {
        if Self::set_kind(&mut self.filter.returns, &kind) {
            self.refresh();
        }
    }

    #[qslot]
    pub fn reset(&mut self) {
        if self.browsing() {
            self.query.clear();
            self.filter = Filter::default();
            self.refresh();
        }
    }

    #[qslot]
    pub fn load_more(&mut self) {
        self.append_page();
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn kinds(&self) -> Value {
        json!(TypeKind::ALL.map(|kind| kind.to_string()))
    }

    #[qslot]
    pub fn details(&self, path: String) -> Value {
        self.details_json(&path)
    }

    #[qslot]
    pub fn top(&self, query: String, count: i32) -> Value {
        top_json(self.top_matches(query.trim(), count_arg(count)))
    }
}
