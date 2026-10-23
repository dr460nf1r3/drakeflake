mod diff;
mod screen;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase as _, QModelItem, qobject};
use serde::Deserialize as _;
use serde_json::Value;

use crate::hub::{ConsoleSession, Hub};
use crate::models::count_i32;

const MAX_LOG_LINES: usize = 5000;

#[derive(Clone, Default, QModelItem)]
pub struct LogLine {
    pub text: String,
    pub html: String,
    pub raw: String,
}

#[derive(Clone, Copy)]
struct ScreenBlock {
    session: ConsoleSession,
    base: usize,
    rows: usize,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    #[default]
    YesNo,
    Password,
}

#[derive(Default)]
pub struct ConsoleModel {
    hub: Arc<Hub>,
    log: Vec<LogLine>,
    palette: Vec<String>,
    screen_block: Option<ScreenBlock>,
    prompt: String,
    prompt_core: String,
    prompt_kind: PromptKind,
}

impl QListModel for ConsoleModel {
    type Item = LogLine;

    // Newest first, so a line printed above nom's live tree leaves the tree in place.

    fn len(&self) -> usize {
        self.log.len()
    }

    fn get(&self, index: usize) -> Option<&Self::Item> {
        self.log.get(self.row(index)?)
    }

    fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
        self.row(index)
            .and_then(|position| self.log.get_mut(position))
            .map(|line| *line = value)
            .is_some()
    }

    fn push_unnotified(&mut self, value: Self::Item) {
        self.log.insert(0, value);
    }

    fn insert_unnotified(&mut self, index: usize, value: Self::Item) {
        let position = self.log.len().saturating_sub(index);
        self.log.insert(position, value);
    }

    fn pop_unnotified(&mut self) -> Option<Self::Item> {
        (!self.log.is_empty()).then(|| self.log.remove(0))
    }

    fn remove_unnotified(&mut self, index: usize) -> Self::Item {
        match self.row(index) {
            Some(position) => self.log.remove(position),
            None => LogLine::default(),
        }
    }

    fn reset_unnotified(&mut self) {
        self.log.clear();
    }
}

impl ConsoleModel {
    #[must_use = "the model is only built"]
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }
}

#[qobject(Base = QListModel)]
impl ConsoleModel {
    qproperty!("line_count", Read = line_count, Notify = state_changed);
    qproperty!("prompt", Read = prompt, Notify = state_changed);
    qproperty!("prompt_core", Read = prompt_core, Notify = state_changed);
    qproperty!(
        "prompt_password",
        Read = prompt_password,
        Notify = state_changed
    );

    pub fn line_count(&self) -> i32 {
        count_i32(self.log.len())
    }

    pub fn prompt(&self) -> String {
        self.prompt.clone()
    }

    pub fn prompt_core(&self) -> String {
        self.prompt_core.clone()
    }

    pub fn prompt_password(&self) -> bool {
        self.prompt_kind == PromptKind::Password
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().console.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    /// Answer the running program's password prompt (the text is never logged).
    #[qslot]
    pub fn answer_password(&mut self, password: String) {
        log::info!("password sent for {:?}", self.prompt);
        self.prompt.clear();
        self.prompt_core.clear();
        self.hub.answer_secret(&password);
        self.state_changed();
    }

    #[qslot]
    pub fn answer_yes(&mut self) {
        self.answer(true);
        self.state_changed();
    }

    #[qslot]
    pub fn answer_no(&mut self) {
        self.answer(false);
        self.state_changed();
    }

    #[qslot]
    pub fn clear_log(&mut self) {
        log::debug!("console cleared ({} lines)", self.log.len());
        self.reset();
        self.screen_block = None;
        self.state_changed();
    }

    #[qslot]
    pub fn log_text(&self) -> String {
        self.log_lines()
    }

    #[qslot]
    pub fn set_terminal_size(&self, columns: i32, rows: i32) {
        let size = |value: i32| u16::try_from(value.max(0_i32)).unwrap_or(u16::MAX);
        self.hub.set_terminal_size(size(columns), size(rows));
    }

    #[qslot]
    pub fn set_console_palette(&mut self, palette: Value) {
        let palette = Vec::<String>::deserialize(&palette).unwrap_or_else(|error| {
            log::warn!("invalid console palette from QML ({error}); using default colours");
            Vec::new()
        });

        self.recolour(palette);
    }
}
