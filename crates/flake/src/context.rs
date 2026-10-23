use std::sync::Arc;

use drakeflake_exec::{CancelToken, Context};

use crate::pool::Priority;

#[derive(Clone)]
pub struct Nix {
    exec: Context,
    priority: Arc<Priority>,
}

impl Nix {
    #[must_use = "the context is only built"]
    pub fn new(exec: Context) -> Self {
        Self {
            exec,
            priority: Arc::new(Priority::normal()),
        }
    }

    #[must_use = "the context is only built"]
    pub fn system() -> Self {
        Self::new(Context::system())
    }

    #[must_use = "only reads the context"]
    pub const fn context(&self) -> &Context {
        &self.exec
    }

    #[must_use = "only reads the priority"]
    pub(crate) fn priority(&self) -> &Priority {
        &self.priority
    }

    #[must_use = "the context is only built"]
    pub fn with_priority(&self, priority: Priority) -> Self {
        Self {
            exec: self.exec.clone(),
            priority: Arc::new(priority),
        }
    }

    #[must_use = "the context is only built"]
    pub fn with_cancel(&self, cancel: CancelToken) -> Self {
        Self {
            exec: self.exec.with_cancel(cancel),
            priority: Arc::clone(&self.priority),
        }
    }

    #[must_use = "the context is only built"]
    pub fn child(&self) -> Self {
        Self {
            exec: self.exec.child(),
            priority: Arc::clone(&self.priority),
        }
    }

    #[must_use = "the context is only built"]
    pub fn sibling(&self) -> Self {
        self.with_priority(self.priority.handed_on())
    }

    #[must_use = "the contexts are only built"]
    pub fn workers(&self, count: usize) -> Vec<Self> {
        let workers = std::iter::repeat_with(|| self.sibling())
            .take(count)
            .collect();

        self.priority.release();
        workers
    }

    #[must_use = "only reads the priority"]
    pub fn is_urgent(&self) -> bool {
        self.priority.is_urgent()
    }

    pub fn cancel(&self) {
        self.exec.cancel();
    }

    #[must_use = "only reads the cancel flag, nothing is cancelled"]
    pub fn is_cancelled(&self) -> bool {
        self.exec.is_cancelled()
    }

    #[must_use = "only reads the token"]
    pub const fn cancel_token(&self) -> &CancelToken {
        self.exec.cancel_token()
    }
}

impl std::fmt::Debug for Nix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Nix")
            .field("exec", &self.exec)
            .field("urgent", &self.priority.is_urgent())
            .finish()
    }
}

#[cfg(test)]
mod tests;
