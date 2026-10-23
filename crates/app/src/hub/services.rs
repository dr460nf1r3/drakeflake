use super::Mailboxes;
use super::console::Console;
use super::session::Session;

#[derive(Clone, Copy)]
pub struct Services<'a> {
    pub session: &'a Session,
    pub console: &'a Console,
    pub mail: &'a Mailboxes,
}
