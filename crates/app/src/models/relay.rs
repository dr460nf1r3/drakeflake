//! Requests between models go through a mailbox here, so no model ever borrows another.

use crate::hub::Mailbox;

#[derive(Default)]
pub struct Relay {
    pub system: Mailbox<SystemNote>,
}

impl Relay {
    pub fn detach(&self) {
        self.system.detach();
    }
}

#[derive(Clone, Copy)]
pub enum SystemNote {
    Reload,
    LayoutChanged,
}
