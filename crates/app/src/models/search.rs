use std::sync::Arc;

use qtbridge::qobject;
use serde_json::{Value, json};

use crate::hub::{FlakeHit, Hub, SearchMsg};

#[derive(Default)]
pub struct SearchModel {
    hub: Arc<Hub>,
    query: String,
    answered: String,
    total: usize,
    hits: Vec<FlakeHit>,
}

impl SearchModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    fn handle(&mut self, message: SearchMsg) {
        match message {
            SearchMsg::Flake { query, total, hits } => {
                if query != self.query {
                    log::debug!("dropping source hits for the older query {query:?}");
                    return;
                }

                self.answered = query;
                self.total = total;
                self.hits = hits;
            }
        }
    }
}

#[qobject]
impl SearchModel {
    qproperty!("flake_hits", Read = flake_hits, Notify = state_changed);
    qproperty!("searching", Read = searching, Notify = state_changed);

    pub fn flake_hits(&self) -> Value {
        json!({ "query": self.answered, "total": self.total, "hits": self.hits })
    }

    pub fn searching(&self) -> bool {
        !self.query.is_empty() && self.query != self.answered
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().search.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn search_flake(&mut self, query: String) {
        let query = query.trim().to_owned();

        if query == self.query {
            return;
        }

        self.query.clone_from(&query);
        self.answered.clear();
        self.total = 0;
        self.hits.clear();

        if !query.is_empty() {
            self.hub.search_flake(query);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn open_hit(&self, file: String, line: i32) {
        crate::hub::editor::open_and_report(&self.hub, &file, line);
    }
}
