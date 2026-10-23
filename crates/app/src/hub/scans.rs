use std::path::Path;
use std::sync::Arc;

use drakeflake_plan::ManagedState;

use super::generations::{PROFILES, list_generations};
use super::session::Session;
use super::{GenerationsMsg, Hub, cleanup, history};

pub fn load_generations(owner: &Arc<Hub>) {
    owner.spawn_background("generations", |hub, _| {
        let found = if hub.capabilities().generations {
            list_generations(Path::new(PROFILES))
        } else {
            Vec::new()
        };

        hub.mail
            .generations
            .post(GenerationsMsg::Generations(found));
    });
}

pub fn load_usage(owner: &Arc<Hub>) {
    owner.spawn_background("store-usage", |hub, _| {
        hub.mail
            .generations
            .post(GenerationsMsg::Usage(cleanup::usage()));
    });
}

pub fn load_history(owner: &Arc<Hub>) {
    owner.spawn_background("history", |hub, _| {
        hub.mail
            .generations
            .post(GenerationsMsg::History(history_entries(hub.session())));
    });
}

fn history_entries(session: &Session) -> Vec<history::HistoryEntry> {
    let entry = match session.managed_file() {
        Ok(entry) => entry,
        Err(error) => {
            log::debug!("no history: {error}");
            return Vec::new();
        }
    };

    let current = ManagedState::load(&entry).unwrap_or_else(|error| {
        log::warn!("cannot read {}: {error}", entry.display());
        ManagedState::default()
    });

    history::list_history(&entry, &current)
}
