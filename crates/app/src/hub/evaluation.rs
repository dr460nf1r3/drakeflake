use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use drakeflake_exec::CancelToken;
use drakeflake_render::PackageAttr;

use super::{CheckRequest, lock};

pub const WORKER_IDLE: Duration = Duration::from_secs(60);

pub struct Queue<T> {
    sender: Mutex<Option<mpsc::Sender<T>>>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            sender: Mutex::new(None),
        }
    }
}

impl<T> Queue<T> {
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the slot stays locked until the new worker is registered"
    )]
    #[must_use = "without the receiver nobody evaluates the request"]
    pub fn send_or_start(&self, item: T) -> Option<mpsc::Receiver<T>> {
        let mut slot = lock(&self.sender);

        let item = match slot.as_ref() {
            Some(sender) => match sender.send(item) {
                Ok(()) => return None,
                Err(mpsc::SendError(item)) => {
                    log::debug!("the worker is gone, starting a new one");
                    item
                }
            },
            None => item,
        };

        let (sender, receiver) = mpsc::channel();

        sender.send(item).ok();
        *slot = Some(sender);
        Some(receiver)
    }
}

#[derive(Default)]
pub struct Evaluation {
    values: Queue<(
        super::ids::Generation,
        super::ids::HomeEpoch,
        drakeflake_render::OptionName,
    )>,
    checks: Queue<(super::ids::Generation, CheckRequest)>,
    packages: Queue<(super::ids::Generation, PackageAttr)>,
    search: AtomicU64,
    load: Mutex<CancelToken>,
}

impl Evaluation {
    pub const fn values(
        &self,
    ) -> &Queue<(
        super::ids::Generation,
        super::ids::HomeEpoch,
        drakeflake_render::OptionName,
    )> {
        &self.values
    }

    pub const fn checks(&self) -> &Queue<(super::ids::Generation, CheckRequest)> {
        &self.checks
    }

    pub const fn packages(&self) -> &Queue<(super::ids::Generation, PackageAttr)> {
        &self.packages
    }

    pub fn next_search(&self) -> u64 {
        self.search.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn search_is_current(&self, number: u64) -> bool {
        self.search.load(Ordering::SeqCst) == number
    }

    pub fn replace_load(&self, token: CancelToken) {
        std::mem::replace(&mut *lock(&self.load), token).cancel();
    }

    pub fn cancel_load(&self) {
        lock(&self.load).cancel();
    }

    #[must_use = "the token is only built"]
    pub fn scoped_token(&self) -> CancelToken {
        lock(&self.load).child()
    }

    #[cfg(test)]
    pub fn load_token(&self) -> CancelToken {
        lock(&self.load).clone()
    }
}

#[cfg(test)]
mod tests;
