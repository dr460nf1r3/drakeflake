use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use drakeflake_evaluate::cache::Cache;
use drakeflake_flake::OnSkip;

use super::{ConsoleMsg, Mailboxes, SystemMsg, lock};

/// Ends its entry on drop, so an early return or a panic never leaves the status bar busy.
/// The entry has an id of its own, so a guard never removes the entry of another job.
#[must_use = "the activity ends when the guard is dropped"]
pub struct ActivityGuard<'a> {
    console: &'a Console,
    id: u64,
    key: &'static str,
}

impl ActivityGuard<'_> {
    pub fn update(&self, text: String) {
        self.console.set_activity(self.id, self.key, Some(text));
    }
}

impl Drop for ActivityGuard<'_> {
    fn drop(&mut self) {
        self.console.set_activity(self.id, self.key, None);
    }
}

#[derive(Clone, Copy)]
pub struct Progress<'a> {
    pub key: &'static str,
    pub label: &'a str,
    pub skipped: &'a str,
}

#[derive(Default)]
struct Activities {
    next_id: u64,
    shown: BTreeMap<u64, String>,
}

pub struct Console {
    mail: Arc<Mailboxes>,
    activities: Mutex<Activities>,
}

impl Console {
    pub fn new(mail: Arc<Mailboxes>) -> Self {
        Self {
            mail,
            activities: Mutex::default(),
        }
    }

    pub fn log(&self, line: impl Into<String>) {
        self.line(log::Level::Info, line.into());
    }

    pub fn warn(&self, line: impl Into<String>) {
        self.line(log::Level::Warn, line.into());
    }

    pub fn error(&self, line: impl Into<String>) {
        self.line(log::Level::Error, line.into());
    }

    fn line(&self, level: log::Level, line: String) {
        log::log!(target: "console", level, "{line}");
        self.mail.console.post(ConsoleMsg::Lines(vec![line]));
    }

    pub fn activity(&self, key: &'static str, text: impl Into<String>) -> ActivityGuard<'_> {
        let id = {
            let mut activities = lock(&self.activities);
            activities.next_id += 1;
            activities.next_id
        };

        self.set_activity(id, key, Some(text.into()));
        ActivityGuard {
            console: self,
            id,
            key,
        }
    }

    fn set_activity(&self, id: u64, key: &'static str, text: Option<String>) {
        let mut activities = lock(&self.activities);

        if let Some(text) = text {
            log::debug!(target: "activity", "{key}: {text}");
            activities.shown.insert(id, text);
        } else {
            log::debug!(target: "activity", "{key}: done");
            activities.shown.remove(&id);
        }

        self.mail.system.post(SystemMsg::Activities(
            activities.shown.values().cloned().collect(),
        ));
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "posting under the lock keeps the snapshots in order"
    )]
    pub fn clear_activities(&self) {
        let mut activities = lock(&self.activities);
        activities.shown.clear();
        self.mail.system.post(SystemMsg::Activities(Vec::new()));
    }

    pub fn with_progress<T>(
        &self,
        progress: Option<Progress<'_>>,
        run: impl FnOnce(OnSkip<'_>) -> T,
    ) -> T {
        let activity = progress.map(|shown| (self.activity(shown.key, shown.label), shown));
        let on_skip = |_: &str, count: usize| {
            if let Some((guard, shown)) = &activity {
                guard.update(format!(
                    "{} (skipping {count} {})",
                    shown.label, shown.skipped
                ));
            }
        };

        run(&on_skip)
    }

    pub fn evaluate_skipping<T>(
        &self,
        cache: &Cache,
        file: &str,
        progress: Option<Progress<'_>>,
        evaluate: impl FnOnce(&mut Vec<String>, OnSkip<'_>) -> T,
    ) -> (T, Vec<String>) {
        self.with_progress(progress, |on_skip| {
            drakeflake_evaluate::evaluate_skipping(cache, file, evaluate, on_skip)
        })
    }
}

#[cfg(test)]
mod tests;
