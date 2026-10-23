//! qtbridge aborts on a nested model borrow, so everything crossing models or threads is posted to a [`Mailbox`].

mod apply;
mod argv;
mod capabilities;
pub mod cleanup;
mod commands;
mod commit;
mod console;
pub mod editor;
mod error;
mod evaluation;
mod facade;
mod flake_search;
mod functions;
pub mod generations;
pub mod history;
mod home;
mod host_list;
mod ids;
pub mod images;
#[cfg(test)]
mod integration;
mod load;
mod lock;
mod mailbox;
mod messages;
mod operations;
mod package_sets;
pub mod persistence;
pub mod portal;
mod programs;
pub mod pty;
mod rebuild;
mod refresh;
mod reload;
mod review;
mod scans;
mod services;
mod session;
pub mod setup;
mod steps;
mod system;
mod target;
pub mod term;
pub mod terminal;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
pub mod try_package;
mod values;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

pub use capabilities::Capabilities;
use console::Console;
pub use drakeflake_evaluate::cache::Cache;
use drakeflake_exec::{CancelToken, Context};
use drakeflake_flake::Nix;
pub use error::HubError;
use evaluation::Evaluation;
pub use flake_search::FlakeHit;
pub use ids::{ConsoleSession, Generation, HomeEpoch};
pub use load::CacheUse;
pub use mailbox::Mailbox;
pub use messages::{
    ChangesMsg, CheckRequest, CommandLookupError, ConsoleMsg, FunctionsMsg, GenerationsMsg,
    ImagesMsg, InputsMsg, Mode, OptionsMsg, PackagesMsg, Prompt, SearchMsg, SetupMsg, SystemMsg,
    TargetInfo, Warning, WarningId,
};
use operations::Operations;
pub use operations::{Exclusive, Outcome, Reporting};
use programs::{find_program, find_root_program, helper_path, in_store};
pub use pty::strip_ansi;
use services::Services;
use session::Session;

#[derive(Default)]
pub struct Mailboxes {
    pub options: Mailbox<OptionsMsg>,
    pub configured: Mailbox<OptionsMsg>,
    pub home: Mailbox<OptionsMsg>,
    pub tree: Mailbox<OptionsMsg>,
    pub home_tree: Mailbox<OptionsMsg>,
    pub packages: Mailbox<PackagesMsg>,
    pub changes: Mailbox<ChangesMsg>,
    pub system: Mailbox<SystemMsg>,
    pub console: Mailbox<ConsoleMsg>,
    pub generations: Mailbox<GenerationsMsg>,
    pub images: Mailbox<ImagesMsg>,
    pub inputs: Mailbox<InputsMsg>,
    pub setup: Mailbox<SetupMsg>,
    pub functions: Mailbox<FunctionsMsg>,
    pub search: Mailbox<SearchMsg>,
    pub persistence: Mailbox<persistence::PersistenceMsg>,
}

impl Mailboxes {
    pub fn post_options(&self, message: OptionsMsg) {
        self.configured.post(message.clone());
        self.tree.post(message.clone());
        self.options.post(message);
    }

    pub fn post_home(&self, message: OptionsMsg) {
        self.home_tree.post(message.clone());
        self.home.post(message);
    }
}

struct Processes(Context);

impl Default for Processes {
    fn default() -> Self {
        Self(Context::system())
    }
}

pub struct Hub {
    mail: Arc<Mailboxes>,
    processes: Processes,
    session: Session,
    operations: Operations,
    evaluation: Evaluation,
    console: Console,
    background: CancelToken,
}

impl Default for Hub {
    fn default() -> Self {
        Self::with_processes(Processes::default())
    }
}

impl Hub {
    fn with_processes(processes: Processes) -> Self {
        let mail = Arc::new(Mailboxes::default());

        Self {
            console: Console::new(Arc::clone(&mail)),
            mail,
            processes,
            session: Session::default(),
            operations: Operations::default(),
            evaluation: Evaluation::default(),
            background: CancelToken::new(),
        }
    }

    #[cfg(test)]
    pub fn with_context(exec: Context) -> Self {
        Self::with_processes(Processes(exec))
    }

    #[must_use = "the mailboxes are only looked up, nothing is posted"]
    pub fn mail(&self) -> &Mailboxes {
        &self.mail
    }

    pub const fn session(&self) -> &Session {
        &self.session
    }

    pub const fn operations(&self) -> &Operations {
        &self.operations
    }

    pub const fn evaluation(&self) -> &Evaluation {
        &self.evaluation
    }

    pub const fn console(&self) -> &Console {
        &self.console
    }

    pub fn services(&self) -> Services<'_> {
        Services {
            session: &self.session,
            console: &self.console,
            mail: &self.mail,
        }
    }

    #[must_use = "the context is only built"]
    pub fn nix(&self) -> Nix {
        self.nix_with(CancelToken::new())
    }

    fn nix_with(&self, cancel: CancelToken) -> Nix {
        Nix::new(self.processes.0.with_cancel(cancel))
    }

    pub fn spawn_background(
        self: &Arc<Self>,
        name: &str,
        job: impl FnOnce(&Arc<Self>, &Nix) + Send + 'static,
    ) {
        launch(
            self,
            name,
            self.nix_with(self.evaluation.scoped_token()),
            job,
        );
    }

    pub fn spawn_persistent(
        self: &Arc<Self>,
        name: &str,
        job: impl FnOnce(&Arc<Self>, &Nix) + Send + 'static,
    ) {
        launch(self, name, self.nix_with(self.background.child()), job);
    }

    pub fn spawn_exclusive(
        self: &Arc<Self>,
        name: &str,
        reporting: Reporting,
        job: impl FnOnce(&Exclusive) -> Outcome + Send + 'static,
    ) -> Result<(), HubError> {
        operations::start(self, name, reporting, job)
    }

    /// Called after the event loop ended: no thread may call into Qt anymore.
    pub fn shutdown(&self) {
        self.operations.cancel();
        self.evaluation.cancel_load();
        self.background.cancel();
        self.mail.options.detach();
        self.mail.configured.detach();
        self.mail.home.detach();
        self.mail.tree.detach();
        self.mail.home_tree.detach();
        self.mail.packages.detach();
        self.mail.changes.detach();
        self.mail.system.detach();
        self.mail.console.detach();
        self.mail.generations.detach();
        self.mail.images.detach();
        self.mail.inputs.detach();
        self.mail.setup.detach();
        self.mail.functions.detach();
        self.mail.search.detach();
        self.mail.persistence.detach();
    }

    pub fn watch_ui(self: &Arc<Self>, after: Duration) {
        self.spawn_persistent("ui-watchdog", move |hub, _| {
            std::thread::sleep(after);
            hub.mail.system.post(SystemMsg::CheckUi);
        });
    }
}

fn launch(
    hub: &Arc<Hub>,
    name: &str,
    nix: Nix,
    job: impl FnOnce(&Arc<Hub>, &Nix) + Send + 'static,
) {
    let owner = Arc::clone(hub);
    let label = name.to_owned();

    let spawned = std::thread::Builder::new()
        .name(format!("df-{name}"))
        .spawn(move || {
            let outcome = catch_unwind(AssertUnwindSafe(|| job(&owner, &nix)));

            if outcome.is_err() {
                let log = crate::crash_log_path()
                    .map_or_else(|| "the log".to_owned(), |path| path.display().to_string());

                owner.log_error(format!(
                    "Internal error in background task \"{label}\"; see {log}"
                ));
            }
        });

    match spawned {
        Ok(_) => log::trace!("started background task {name}"),
        Err(err) => hub.log_error(format!("Cannot start background task \"{name}\": {err}")),
    }
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
