use std::fs::File;
use std::io::Write as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::hub::{ConsoleMsg, Mailbox, lock};

/// Terminal size. nh and nom fill lines to the terminal's width and fit their live tree
/// to its height. The fallback is wide enough for nh's diff tables.
pub(super) const TERMINAL_ROWS: u16 = 40;
pub(super) const TERMINAL_COLUMNS: u16 = 140;
const MIN_COLUMNS: u16 = 40;
const MAX_COLUMNS: u16 = 400;
const MIN_ROWS: u16 = 10;
const MAX_ROWS: u16 = 200;

#[derive(Default)]
pub struct Pty {
    input: Mutex<Option<File>>,
    size: AtomicU32,
}

impl Pty {
    pub(super) fn attach(&self, master: Option<File>) {
        *lock(&self.input) = master;
    }

    pub(super) fn detach(&self) {
        lock(&self.input).take();
    }

    pub fn set_size(&self, columns: u16, rows: u16) {
        let size = window_size(
            columns.clamp(MIN_COLUMNS, MAX_COLUMNS),
            rows.clamp(MIN_ROWS, MAX_ROWS),
        );

        let packed = (u32::from(size.ws_col) << 16_u32) | u32::from(size.ws_row);
        if self.size.swap(packed, Ordering::Relaxed) == packed {
            return;
        }

        log::debug!("terminal size: {} × {}", size.ws_col, size.ws_row);

        if let Some(master) = lock(&self.input).as_ref()
            && let Err(err) = rustix::termios::tcsetwinsize(master, size)
        {
            log::debug!("cannot resize the running program's terminal: {err}");
        }
    }

    pub(super) fn size(&self) -> rustix::termios::Winsize {
        let packed = self.size.load(Ordering::Relaxed);
        let half = |bits: u32| u16::try_from(bits & 0xffff).unwrap_or(u16::MAX);

        if packed == 0 {
            window_size(TERMINAL_COLUMNS, TERMINAL_ROWS)
        } else {
            window_size(half(packed >> 16), half(packed))
        }
    }

    pub fn answer(&self, console: &Mailbox<ConsoleMsg>, yes: bool) {
        log::debug!("answering {}", if yes { "yes" } else { "no" });
        self.send("answer", &[if yes { b"y\n" } else { b"n\n" }], console);
    }

    pub fn answer_secret(&self, console: &Mailbox<ConsoleMsg>, secret: &str) {
        if self.echoes() {
            log::warn!(
                "password not sent: the terminal echoes input, so no password is being read"
            );
            console.post(ConsoleMsg::Prompt(None));
            return;
        }

        log::debug!("sending the password to the running program");
        self.send("password", &[secret.as_bytes(), b"\n"], console);
    }

    pub(super) fn echoes(&self) -> bool {
        lock(&self.input).as_ref().is_none_or(|master| {
            rustix::termios::tcgetattr(master).map_or(true, |attrs| {
                attrs
                    .local_modes
                    .contains(rustix::termios::LocalModes::ECHO)
            })
        })
    }

    pub(super) fn with_input<T>(&self, use_input: impl FnOnce(Option<&mut File>) -> T) -> T {
        use_input(lock(&self.input).as_mut())
    }

    fn send(&self, what: &str, parts: &[&[u8]], console: &Mailbox<ConsoleMsg>) {
        if let Some(input) = lock(&self.input).as_mut() {
            let written = parts
                .iter()
                .try_for_each(|part| input.write_all(part))
                .and_then(|()| input.flush());

            if let Err(err) = written {
                log::warn!("cannot send the {what} to the program: {err}");
            }
        } else {
            log::warn!("{what} ignored: no program is running");
        }

        console.post(ConsoleMsg::Prompt(None));
    }
}

pub(super) const fn window_size(columns: u16, rows: u16) -> rustix::termios::Winsize {
    rustix::termios::Winsize {
        ws_row: rows,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}
