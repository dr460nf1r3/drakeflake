mod group;
mod pipe;
mod redact;

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use self::group::{kill_with_parent, own_group, stop};
use self::pipe::read_pipe;
pub use self::redact::redact_urls;
use crate::cancel::CancelToken;
use crate::error::ExecError;

pub const SHORT_TIMEOUT: Duration = Duration::from_secs(180);
pub const LONG_TIMEOUT: Duration = Duration::from_mins(15);
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);
const MIN_POLL: Duration = Duration::from_millis(1);
/// Grandchildren may keep a finished command's pipes open.
const PIPE_GRACE: Duration = Duration::from_secs(5);
const TAIL_LINES: usize = 4;
const LOGGED_ARG_CHARS: usize = 160;

#[derive(Debug, Clone)]
pub struct Captured {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct Invocation<'a> {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub input: Option<&'a str>,
    pub cwd: Option<&'a Path>,
    pub timeout: Duration,
    pub cancel: Option<&'a CancelToken>,
    pub failure: log::Level,
    pub summarize: fn(&str) -> String,
}

impl<'a> Invocation<'a> {
    #[must_use = "the invocation is only built"]
    pub fn new(
        program: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
        timeout: Duration,
    ) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
            input: None,
            cwd: None,
            timeout,
            cancel: None,
            failure: log::Level::Warn,
            summarize: stderr_tail,
        }
    }

    #[must_use = "the invocation is only built"]
    pub const fn input(mut self, input: &'a str) -> Self {
        self.input = Some(input);
        self
    }

    #[must_use = "the invocation is only built"]
    pub const fn cwd(mut self, cwd: Option<&'a Path>) -> Self {
        self.cwd = cwd;
        self
    }

    #[must_use = "the invocation is only built"]
    pub const fn failure(mut self, failure: log::Level) -> Self {
        self.failure = failure;
        self
    }

    #[must_use = "the invocation is only built"]
    pub const fn summarize(mut self, summarize: fn(&str) -> String) -> Self {
        self.summarize = summarize;
        self
    }
}

pub fn run_logged(invocation: Invocation<'_>) -> Result<Captured, ExecError> {
    let failure = invocation.failure;
    let summarize = invocation.summarize;
    let started = Instant::now();
    let line = command_line(&invocation.program, &invocation.args);
    let name = display_name(&invocation.program).into_owned();

    if log::log_enabled!(target: "command", log::Level::Debug) {
        log::debug!(target: "command", "run: {line}");
    }

    let result = spawn_and_wait(invocation);
    let seconds = started.elapsed().as_secs_f32();

    match &result {
        Ok(captured) if captured.success => {
            log::debug!(target: "command", "{name} finished in {seconds:.1}s");
            log::trace!(target: "command", "{name} stderr:\n{}", captured.stderr);
        }
        Ok(captured) => log::log!(
            target: "command",
            failure,
            "{line}\nfailed after {seconds:.1}s:\n{}",
            summarize(&captured.stderr)
        ),
        Err(error) => log::warn!(target: "command", "{line}\n{error}"),
    }

    result
}

#[must_use = "the tail is only built, nothing is logged"]
pub fn stderr_tail(stderr: &str) -> String {
    let tail: Vec<&str> = stderr
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().is_empty())
        .rev()
        .take(TAIL_LINES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    tail.join("\n")
}

fn command_line(program: &OsStr, args: &[OsString]) -> String {
    args.iter()
        .fold(display_name(program).into_owned(), |mut line, arg| {
            let flat = redact_urls(&arg.to_string_lossy());
            line.push(' ');

            let mut chars = flat.chars();
            let shortened: String = chars.by_ref().take(LOGGED_ARG_CHARS).collect();
            line.push_str(&shortened);

            if chars.next().is_some() {
                line.push('…');
            }

            line
        })
}

fn display_name(program: &OsStr) -> Cow<'_, str> {
    program.to_string_lossy()
}

fn spawn_and_wait(invocation: Invocation<'_>) -> Result<Captured, ExecError> {
    let Invocation {
        program,
        args,
        input,
        cwd,
        timeout,
        cancel,
        failure: _,
        summarize: _,
    } = invocation;

    let name = display_name(&program);
    let mut command = Command::new(&program);

    if let Some(dir) = cwd {
        if !dir.is_dir() {
            return Err(ExecError::Failed(format!(
                "cannot run {name}: {} is not a directory",
                dir.display()
            )));
        }

        command.current_dir(dir);
    }

    own_group(&mut command);
    kill_with_parent(&mut command);

    let stdin_mode = if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    };

    let mut child = command
        .args(&args)
        .stdin(stdin_mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| ExecError::spawn(&name, &err))?;

    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let text = text.to_owned();
        let owner = name.as_ref().to_owned();

        // A child that does not read its stdin must not block us.
        std::thread::spawn(move || {
            if let Err(err) = std::io::Write::write_all(&mut stdin, text.as_bytes()) {
                log::debug!("writing stdin of {owner} failed: {err}");
            }
        });
    }

    let stdout = read_pipe(child.stdout.take());
    let stderr = read_pipe(child.stderr.take());
    let status = wait_for_exit(&mut child, &name, timeout, cancel)?;

    Ok(Captured {
        success: status.success(),
        code: status.code(),
        stdout: stdout.finish(PIPE_GRACE, &name, "stdout"),
        stderr: stderr.finish(PIPE_GRACE, &name, "stderr"),
    })
}

fn wait_for_exit(
    child: &mut Child,
    program: &str,
    timeout: Duration,
    cancel: Option<&CancelToken>,
) -> Result<ExitStatus, ExecError> {
    let started = Instant::now();
    let mut pause = MIN_POLL;

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| ExecError::wait(program, &err))?
        {
            return Ok(status);
        }

        let cancelled = cancel.is_some_and(CancelToken::is_cancelled);

        if cancelled || started.elapsed() > timeout {
            stop(child, program);

            return Err(if cancelled {
                ExecError::Cancelled(Some(program.to_owned()))
            } else {
                ExecError::TimedOut {
                    program: program.to_owned(),
                    after: timeout,
                }
            });
        }

        std::thread::sleep(pause);
        pause = (pause * 2).min(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests;
