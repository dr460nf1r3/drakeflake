use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use qtbridge::QmlMethodInvoker;

use super::lock;

pub struct Mailbox<T> {
    queue: Mutex<Vec<T>>,
    invoker: Mutex<Option<QmlMethodInvoker>>,
    detached: AtomicBool,
}

impl<T> Default for Mailbox<T> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            invoker: Mutex::new(None),
            detached: AtomicBool::new(false),
        }
    }
}

impl<T> Mailbox<T> {
    pub fn attach(&self, invoker: QmlMethodInvoker) {
        *lock(&self.invoker) = Some(invoker);
        self.wake();
    }

    pub fn post(&self, message: T) {
        if self.detached.load(Ordering::SeqCst) {
            return;
        }

        let was_empty = {
            let mut queue = lock(&self.queue);
            let was_empty = queue.is_empty();
            queue.push(message);
            was_empty
        };

        if was_empty {
            self.wake();
        }
    }

    pub fn detach(&self) {
        self.detached.store(true, Ordering::SeqCst);

        lock(&self.invoker).take();
    }

    fn wake(&self) {
        if let Some(invoker) = lock(&self.invoker).as_ref() {
            invoker.invoke_method("drain");
        } else {
            log::debug!(
                "no model attached to the {} mailbox; message stays queued",
                std::any::type_name::<T>()
            );
        }
    }

    pub fn take(&self) -> Vec<T> {
        std::mem::take(&mut *lock(&self.queue))
    }
}

#[cfg(test)]
mod tests;
