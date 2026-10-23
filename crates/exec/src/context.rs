use std::ffi::OsStr;
use std::sync::Arc;
use std::time::Duration;

use crate::cancel::CancelToken;
use crate::error::ExecError;
use crate::process::{Captured, Invocation};
use crate::runner::{Runner, SystemRunner};

#[derive(Clone)]
pub struct Context {
    runner: Arc<dyn Runner>,
    cancel: CancelToken,
}

impl Context {
    #[must_use = "the context is only built"]
    pub fn new(runner: Arc<dyn Runner>) -> Self {
        Self {
            runner,
            cancel: CancelToken::new(),
        }
    }

    #[must_use = "the context is only built"]
    pub fn system() -> Self {
        Self::new(Arc::new(SystemRunner))
    }

    #[must_use = "the context is only built"]
    pub fn with_cancel(&self, cancel: CancelToken) -> Self {
        Self {
            runner: Arc::clone(&self.runner),
            cancel,
        }
    }

    #[must_use = "the context is only built"]
    pub fn child(&self) -> Self {
        self.with_cancel(self.cancel.child())
    }

    #[must_use = "only reads the token"]
    pub const fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    #[must_use = "only reads the cancel flag, nothing is cancelled"]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn run(&self, invocation: Invocation<'_>) -> Result<Captured, ExecError> {
        let cancel = invocation.cancel.unwrap_or(&self.cancel);

        self.runner.run(Invocation {
            cancel: Some(cancel),
            ..invocation
        })
    }

    pub fn command(
        &self,
        program: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
        timeout: Duration,
    ) -> Result<Captured, ExecError> {
        self.run(Invocation::new(program, args, timeout))
    }

    pub fn probe(
        &self,
        program: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
        timeout: Duration,
    ) -> Result<Captured, ExecError> {
        self.run(Invocation::new(program, args, timeout).failure(log::Level::Debug))
    }

    pub fn command_with_input(
        &self,
        program: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
        input: &str,
        timeout: Duration,
    ) -> Result<Captured, ExecError> {
        self.run(Invocation::new(program, args, timeout).input(input))
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("cancel", &self.cancel)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
