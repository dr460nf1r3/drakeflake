use std::fs::File;
use std::io::Write as _;
use std::process::Child;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal};

const TERM_AFTER: Duration = Duration::from_secs(1);

const KILL_AFTER: Duration = Duration::from_secs(2);

/// Time a `SIGKILL`ed program gets before the pump gives up waiting for it.
/// The operation slot must free even when the program cannot die.
const KILL_GRACE: Duration = Duration::from_secs(10);

const INTERRUPT: u8 = 0x03;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Stage {
    Interrupt,
    Terminate,
    Kill,
}

#[derive(Debug, Default)]
pub(super) struct Cancellation {
    started: Option<Instant>,
    sent: Option<Stage>,
    killed: Option<Instant>,
}

impl Cancellation {
    fn due(&mut self, now: Instant) -> Option<Stage> {
        let started = *self.started.get_or_insert(now);
        let elapsed = now.saturating_duration_since(started);

        let next = match self.sent {
            None => Stage::Interrupt,
            Some(Stage::Interrupt) if elapsed >= TERM_AFTER => Stage::Terminate,
            Some(Stage::Terminate) if elapsed >= TERM_AFTER + KILL_AFTER => Stage::Kill,
            Some(_) => return None,
        };

        self.sent = Some(next);
        Some(next)
    }

    pub(super) fn step(&mut self, program: &str, child: &mut Child, terminal: Option<&mut File>) {
        let Some(stage) = self.due(Instant::now()) else {
            return;
        };

        match stage {
            Stage::Interrupt => {
                log::debug!("cancelled: sending Ctrl-C to {program}");
                let written = terminal
                    .map_or_else(|| Err(std::io::Error::other("no terminal")), send_interrupt);

                if let Err(err) = written {
                    log::warn!("cannot send Ctrl-C to {program}: {err}");
                }
            }
            Stage::Terminate => {
                log::debug!("cancelled: {program} ignored Ctrl-C, sending SIGTERM");
                signal_group(program, child, Signal::TERM);
            }
            Stage::Kill => {
                log::warn!("cancelled: {program} ignored SIGTERM, sending SIGKILL");
                signal_group(program, child, Signal::KILL);

                if let Err(err) = child.kill() {
                    log::debug!("cannot kill {program} directly: {err}");
                }

                self.killed.get_or_insert_with(Instant::now);
            }
        }
    }

    pub(super) fn overdue(&self, now: Instant) -> bool {
        self.killed
            .is_some_and(|killed| now.saturating_duration_since(killed) >= KILL_GRACE)
    }
}

fn send_interrupt(master: &mut File) -> std::io::Result<()> {
    master.write_all(&[INTERRUPT])?;
    master.flush()
}

fn signal_group(program: &str, child: &Child, signal: Signal) {
    let Some(group) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) else {
        log::warn!("{program} has no usable process id");
        return;
    };

    if let Err(err) = rustix::process::kill_process_group(group, signal) {
        log::warn!("cannot signal the process group of {program}: {err}");
    }
}

#[cfg(test)]
mod tests;
