use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    parent: Option<Arc<Self>>,
}

impl CancelToken {
    #[must_use = "the token is only built"]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    #[must_use = "only reads the cancel flag, nothing is cancelled"]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
            || self
                .parent
                .as_ref()
                .is_some_and(|parent| parent.is_cancelled())
    }

    #[must_use = "the token is only built"]
    pub fn child(&self) -> Self {
        Self {
            flag: Arc::default(),
            parent: Some(Arc::new(self.clone())),
        }
    }
}

#[cfg(test)]
mod tests;
