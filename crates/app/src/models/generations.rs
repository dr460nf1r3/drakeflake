use std::sync::Arc;

use qtbridge::qobject;
use serde::Serialize;
use serde_json::{Value, json};

use crate::hub::cleanup::Usage;
use crate::hub::generations::Generation;
use crate::hub::history::HistoryEntry;
use crate::hub::{GenerationsMsg, Hub};

struct Scanned<T> {
    value: Option<T>,
    loading: bool,
}

impl<T> Default for Scanned<T> {
    fn default() -> Self {
        Self {
            value: None,
            loading: false,
        }
    }
}

impl<T: Serialize> Scanned<T> {
    fn arrived(&mut self, value: T) {
        self.value = Some(value);
        self.loading = false;
    }

    fn value(&self, empty: Value) -> Value {
        self.value.as_ref().map_or(empty, |value| json!(value))
    }
}

#[derive(Default)]
pub struct GenerationsModel {
    hub: Arc<Hub>,
    generations: Scanned<Vec<Generation>>,
    usage: Scanned<Usage>,
    history: Scanned<Vec<HistoryEntry>>,
}

impl GenerationsModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    fn handle(&mut self, message: GenerationsMsg) {
        match message {
            GenerationsMsg::Generations(found) => self.generations.arrived(found),
            GenerationsMsg::Usage(usage) => self.usage.arrived(usage),
            GenerationsMsg::History(entries) => self.history.arrived(entries),
        }
    }
}

#[qobject]
impl GenerationsModel {
    qproperty!("generations", Read = generations, Notify = state_changed);
    qproperty!(
        "generations_loading",
        Read = generations_loading,
        Notify = state_changed
    );
    qproperty!("cleanup_info", Read = cleanup_info, Notify = state_changed);
    qproperty!(
        "cleanup_loading",
        Read = cleanup_loading,
        Notify = state_changed
    );
    qproperty!("history", Read = history, Notify = state_changed);
    qproperty!(
        "history_loading",
        Read = history_loading,
        Notify = state_changed
    );

    pub fn generations(&self) -> Value {
        self.generations.value(json!([]))
    }

    pub const fn generations_loading(&self) -> bool {
        self.generations.loading
    }

    pub fn cleanup_info(&self) -> Value {
        self.usage.value(json!({}))
    }

    pub const fn cleanup_loading(&self) -> bool {
        self.usage.loading
    }

    pub fn history(&self) -> Value {
        self.history.value(json!([]))
    }

    pub const fn history_loading(&self) -> bool {
        self.history.loading
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().generations.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn load_generations(&mut self) {
        self.generations.loading = true;
        self.hub.load_generations();
        self.state_changed();
    }

    #[qslot]
    pub fn load_cleanup_info(&mut self) {
        self.usage.loading = true;
        self.hub.load_usage();
        self.state_changed();
    }

    #[qslot]
    pub fn load_history(&mut self) {
        self.history.loading = true;
        self.hub.load_history();
        self.state_changed();
    }

    #[qslot]
    pub fn remove_tried(&self) {
        let count = crate::hub::cleanup::remove_tried();
        self.hub.log(format!(
            "Removed {count} tried package(s); the next clean-up frees their space."
        ));
    }
}
