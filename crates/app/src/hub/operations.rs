use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use drakeflake_exec::CancelToken;
use drakeflake_flake::Nix;

use super::console::Console;
use super::error::HubError;
use super::pty::Pty;
use super::session::Session;
use super::{Hub, Mailbox, Mailboxes, SystemMsg, lock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Outcome {
    Succeeded,
    #[default]
    Failed,
    Cancelled,
}

impl Outcome {
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }

    pub fn then(self, next: impl FnOnce() -> Self) -> Self {
        if self.is_success() { next() } else { self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reporting {
    Shown,
    Hidden,
}

struct Running {
    id: u64,
    cancel: CancelToken,
}

#[derive(Default)]
struct State {
    running: Option<Running>,
    last_id: u64,
}

#[derive(Default)]
pub struct Operations {
    state: Mutex<State>,
    pty: Pty,
}

impl Operations {
    pub const fn pty(&self) -> &Pty {
        &self.pty
    }

    #[cfg(test)]
    pub fn is_running(&self) -> bool {
        lock(&self.state).running.is_some()
    }

    pub fn cancel(&self) -> bool {
        lock(&self.state).running.as_ref().is_some_and(|running| {
            running.cancel.cancel();
            true
        })
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "posting under the lock keeps the order of the messages"
    )]
    fn begin(
        &self,
        system: &Mailbox<SystemMsg>,
        reporting: Reporting,
    ) -> Result<(u64, CancelToken), HubError> {
        let mut state = lock(&self.state);

        if state.running.is_some() {
            return Err(HubError::Busy);
        }

        state.last_id += 1;
        let id = state.last_id;
        let cancel = CancelToken::new();

        state.running = Some(Running {
            id,
            cancel: cancel.clone(),
        });

        if reporting == Reporting::Shown {
            system.post(SystemMsg::Running(true));
        }

        Ok((id, cancel))
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "posting under the lock keeps the order of the messages"
    )]
    fn finish(&self, end: Ending<'_>) {
        let mut state = lock(&self.state);

        if state
            .running
            .as_ref()
            .is_some_and(|running| running.id == end.id)
        {
            state.running = None;
        }

        if end.reporting == Reporting::Shown {
            end.system.post(SystemMsg::Finished {
                ok: end.outcome.is_success(),
                backup: end.backup,
                commit: end.commit,
            });
        }
    }
}

struct Ending<'a> {
    id: u64,
    outcome: Outcome,
    backup: Option<PathBuf>,
    commit: Option<String>,
    reporting: Reporting,
    system: &'a Mailbox<SystemMsg>,
}

type Backup = Arc<Mutex<Option<PathBuf>>>;
type Committed = Arc<Mutex<Option<String>>>;

pub struct Exclusive {
    hub: Arc<Hub>,
    cancel: CancelToken,
    nix: Nix,
    backup: Backup,
    commit: Committed,
}

impl Exclusive {
    pub const fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }

    pub const fn nix(&self) -> &Nix {
        &self.nix
    }

    pub fn console(&self) -> &Console {
        self.hub.console()
    }

    pub fn session(&self) -> &Session {
        self.hub.session()
    }

    pub fn mail(&self) -> &Mailboxes {
        self.hub.mail()
    }

    pub fn pty(&self) -> &Pty {
        self.hub.operations().pty()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn record_backup(&self, file: PathBuf) {
        *lock(&self.backup) = Some(file);
    }

    pub fn record_commit(&self, summary: String) {
        *lock(&self.commit) = Some(summary);
    }

    #[cfg(test)]
    pub fn detached(hub: &Arc<Hub>) -> Self {
        Self {
            hub: Arc::clone(hub),
            cancel: CancelToken::new(),
            nix: hub.nix(),
            backup: Backup::default(),
            commit: Committed::default(),
        }
    }

    #[cfg(test)]
    pub fn backup_for_test(&self) -> Option<PathBuf> {
        lock(&self.backup).clone()
    }

    #[cfg(test)]
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }
}

struct Finish {
    hub: Arc<Hub>,
    id: u64,
    outcome: Outcome,
    backup: Backup,
    commit: Committed,
    reporting: Reporting,
}

impl Finish {
    const fn complete(&mut self, outcome: Outcome) {
        self.outcome = outcome;
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        let outcome = std::mem::take(&mut self.outcome);
        let backup = lock(&self.backup).take();
        let commit = lock(&self.commit).take();

        self.hub.operations().finish(Ending {
            id: self.id,
            outcome,
            backup,
            commit,
            reporting: self.reporting,
            system: &self.hub.mail().system,
        });
    }
}

pub fn start(
    hub: &Arc<Hub>,
    name: &str,
    reporting: Reporting,
    job: impl FnOnce(&Exclusive) -> Outcome + Send + 'static,
) -> Result<(), HubError> {
    let (id, cancel) = hub.operations().begin(&hub.mail().system, reporting)?;

    log::debug!("{name}: started");

    let operation = name.to_owned();
    let backup = Backup::default();
    let commit = Committed::default();
    let mut finish = Finish {
        hub: Arc::clone(hub),
        id,
        outcome: Outcome::Failed,
        backup: Arc::clone(&backup),
        commit: Arc::clone(&commit),
        reporting,
    };

    let nix = hub.nix_with(cancel.clone());

    super::launch(hub, name, nix, move |hub, nix| {
        let run = Exclusive {
            hub: Arc::clone(hub),
            cancel,
            nix: nix.clone(),
            backup,
            commit,
        };

        let outcome = job(&run);
        finish.complete(outcome);

        match outcome {
            Outcome::Succeeded => log::info!("{operation}: finished"),
            Outcome::Cancelled => log::info!("{operation}: cancelled"),
            Outcome::Failed => log::error!("{operation}: failed"),
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests;
