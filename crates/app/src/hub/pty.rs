mod ansi;
mod cancel;
mod question;
mod read;
mod slot;
mod spawn;
#[cfg(test)]
mod tests;

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub use ansi::{ansi_to_html, strip_ansi};
use cancel::Cancellation;
use question::{looks_like_password_prompt, looks_like_question, question_core};
use read::{Output, read_output};
pub use slot::Pty;
use spawn::spawn_in_pty;

use super::operations::{Exclusive, Outcome};
use super::term::Emulator;
use super::{ConsoleMsg, Prompt};

const PROMPT_QUIET: Duration = Duration::from_millis(250);

const POLL_INTERVAL: Duration = Duration::from_millis(100);

const FRAME_INTERVAL: Duration = Duration::from_millis(50);

const EXIT_GRACE: Duration = Duration::from_millis(500);

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

struct Display<'a> {
    shown: bool,
    screen: &'a mut Emulator,
}

impl Exclusive {
    pub(super) fn stream_pty(&self, program: &OsStr, args: &[OsString], workdir: &Path) -> Outcome {
        let (status, _) = self.run_pty(program, args, workdir, true);
        self.report_exit(status)
    }

    pub(super) fn stream_pty_quiet(
        &self,
        program: &OsStr,
        args: &[OsString],
        workdir: &Path,
    ) -> Outcome {
        let (status, screen) = self.run_pty(program, args, workdir, false);

        if !self.is_cancelled() && status.as_ref().is_none_or(|status| !status.success()) {
            self.show_final(&screen);
        }

        self.report_exit(status)
    }

    fn run_pty(
        &self,
        program: &OsStr,
        args: &[OsString],
        workdir: &Path,
        shown: bool,
    ) -> (Option<ExitStatus>, Emulator) {
        let name = program.to_string_lossy();
        let program_name = &*name;
        let size = self.pty().size();
        let mut screen = Emulator::new(usize::from(size.ws_col), usize::from(size.ws_row));

        let (mut child, master) = match spawn_in_pty(program, args, workdir, self.pty().size()) {
            Ok(spawned) => spawned,
            Err(err) => {
                self.console()
                    .error(format!("Cannot start {program_name}: {err}"));
                return (None, screen);
            }
        };

        log::debug!(
            "started {program_name} {args:?} in a pseudo-terminal (pid {}, cwd {})",
            child.id(),
            workdir.display()
        );

        self.pty().attach(
            master
                .try_clone()
                .inspect_err(|err| log::warn!("cannot answer {program_name}'s questions: {err}"))
                .ok(),
        );
        let (sender, receiver) = mpsc::channel();
        let stop_reader = read_output(master, sender);
        let mut display = Display {
            shown,
            screen: &mut screen,
        };

        let status = self.pump_pty(program_name, &mut child, &receiver, &mut display);
        log::debug!("{program_name} exited: {status:?}");

        stop_reader.store(true, Ordering::SeqCst);
        self.pty().detach();
        self.mail().console.post(ConsoleMsg::Prompt(None));
        (status, screen)
    }

    fn pump_pty(
        &self,
        program: &str,
        child: &mut Child,
        receiver: &mpsc::Receiver<Output>,
        display: &mut Display<'_>,
    ) -> Option<ExitStatus> {
        let session = super::ids::ConsoleSession(NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
        let mut last_output = Instant::now();
        let mut last_shown = Instant::now();
        let mut prompted = String::new();
        let mut cancellation = Cancellation::default();
        let mut reader_open = true;

        loop {
            let wait = if display.screen.is_dirty() {
                FRAME_INTERVAL.saturating_sub(last_shown.elapsed())
            } else {
                POLL_INTERVAL
            };

            if reader_open {
                match receiver.recv_timeout(wait) {
                    Ok(Output::Chunk(text)) => {
                        display.screen.feed(&text);
                        last_output = Instant::now();

                        feed_queued(display.screen, receiver);
                    }
                    Ok(Output::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        reader_open = false;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            } else {
                std::thread::sleep(wait);
            }

            if display.shown && last_shown.elapsed() >= FRAME_INTERVAL {
                self.show(session, display.screen);
                last_shown = Instant::now();
            }

            self.offer_question(display.screen, last_output, &mut prompted);

            let size = self.pty().size();
            display
                .screen
                .resize(usize::from(size.ws_col), usize::from(size.ws_row));

            if self.poll_cancellation(program, child, &mut cancellation) {
                return None;
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if reader_open {
                        drain_after_exit(display.screen, receiver);
                    }

                    if display.shown {
                        self.show(session, display.screen);
                    }

                    log_screen(program, display.screen, status.success());
                    return Some(status);
                }
                Ok(None) => {}
                Err(err) => {
                    self.console()
                        .error(format!("Waiting for {program} failed: {err}"));
                    return None;
                }
            }
        }
    }

    fn show(&self, session: super::ids::ConsoleSession, screen: &mut Emulator) {
        if let Some(changes) = screen.take_changes() {
            self.mail()
                .console
                .post(ConsoleMsg::Screen { session, changes });
        }
    }

    fn poll_cancellation(
        &self,
        program: &str,
        child: &mut Child,
        cancellation: &mut Cancellation,
    ) -> bool {
        if !self.is_cancelled() {
            return false;
        }

        self.pty()
            .with_input(|input| cancellation.step(program, child, input));

        if cancellation.overdue(Instant::now()) {
            self.console().error(format!(
                "{program} did not stop after SIGKILL; giving up waiting for it"
            ));
            return true;
        }

        false
    }

    fn show_final(&self, screen: &Emulator) {
        let rows: Vec<String> = screen
            .rows()
            .iter()
            .map(|row| strip_ansi(row))
            .filter(|row| !row.trim().is_empty())
            .collect();

        if !rows.is_empty() {
            self.mail().console.post(ConsoleMsg::Lines(rows));
        }
    }

    fn offer_question(&self, screen: &Emulator, last_output: Instant, prompted: &mut String) {
        let question = screen
            .unfinished_line()
            .map(|line| strip_ansi(&line).trim().to_owned())
            .unwrap_or_default();

        let password = looks_like_password_prompt(&question) && !self.pty().echoes();

        if last_output.elapsed() >= PROMPT_QUIET
            && (password || looks_like_question(&question))
            && question != *prompted
        {
            log::debug!("question detected (password: {password}): {question}");
            prompted.clone_from(&question);
            self.mail().console.post(ConsoleMsg::Prompt(Some(Prompt {
                core: question_core(&question),
                text: question,
                password,
            })));
        }
    }

    fn report_exit(&self, status: Option<ExitStatus>) -> Outcome {
        match status {
            Some(status) if status.success() => {
                self.console().log("Finished successfully.");
                Outcome::Succeeded
            }
            Some(status) if self.is_cancelled() => {
                self.console().warn(format!("Cancelled ({status})."));
                Outcome::Cancelled
            }
            Some(status) => {
                self.console().error(format!("Failed ({status})."));
                Outcome::Failed
            }
            None if self.is_cancelled() => {
                self.console().warn("Cancelled.");
                Outcome::Cancelled
            }
            None => Outcome::Failed,
        }
    }
}

fn feed_queued(screen: &mut Emulator, receiver: &mpsc::Receiver<Output>) {
    for message in receiver.try_iter() {
        if let Output::Chunk(more) = message {
            screen.feed(&more);
        }
    }
}

fn drain_after_exit(screen: &mut Emulator, receiver: &mpsc::Receiver<Output>) {
    let deadline = Instant::now() + EXIT_GRACE;

    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(left) {
            Ok(Output::Chunk(text)) => screen.feed(&text),
            Ok(Output::Closed) | Err(_) => return,
        }
    }
}

fn log_screen(program: &str, screen: &Emulator, success: bool) {
    let level = if success {
        log::Level::Debug
    } else {
        log::Level::Warn
    };

    if log::log_enabled!(target: "console", level) {
        let text = screen
            .rows()
            .iter()
            .map(|row| strip_ansi(row))
            .collect::<Vec<_>>()
            .join("\n");

        log::log!(target: "console", level, "{program} output:\n{text}");
    }
}
