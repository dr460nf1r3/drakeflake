use std::fmt;
use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use log::debug;

pub const SYNTAX_CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// A finished parser closes stderr at once. This only guards against a hang.
const PIPE_GRACE: Duration = Duration::from_secs(5);
const PARSER: &str = "nix-instantiate";

/// Boxed, so the app's runner keeps its own error type and a cancelled run stays recognisable.
pub type RunError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRun {
    pub success: bool,
    pub stderr: String,
}

#[derive(Debug)]
pub enum SyntaxError {
    Invalid(String),
    Run(RunError),
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Run(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SyntaxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(_) => None,
            Self::Run(error) => Some(error.as_ref()),
        }
    }
}

#[derive(Debug)]
pub enum SyntaxRunError {
    Spawn {
        program: String,
        source: std::io::Error,
    },
    Wait {
        program: String,
        source: std::io::Error,
    },
    Timeout {
        program: String,
        after: Duration,
    },
}

impl fmt::Display for SyntaxRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn { program, source } => write!(f, "cannot run {program}: {source}"),
            Self::Wait { program, source } => write!(f, "waiting for {program} failed: {source}"),
            Self::Timeout { program, after } => {
                write!(f, "{program} timed out after {}s", after.as_secs())
            }
        }
    }
}

impl std::error::Error for SyntaxRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, program: _ } | Self::Wait { source, program: _ } => Some(source),
            Self::Timeout {
                program: _,
                after: _,
            } => None,
        }
    }
}

/// The text goes over stdin, so no file with a guessable name is ever written.
pub fn check_syntax_with(
    text: &str,
    run: impl FnOnce(&str, &[&str], &str) -> Result<SyntaxRun, RunError>,
) -> Result<(), SyntaxError> {
    let finished = run(PARSER, &["--parse", "-"], text).map_err(SyntaxError::Run)?;

    if finished.success {
        Ok(())
    } else {
        let message = finished.stderr.trim();
        debug!("syntax check failed: {message}");
        Err(SyntaxError::Invalid(message.to_owned()))
    }
}

pub fn check_syntax(text: &str) -> Result<(), SyntaxError> {
    check_syntax_with(text, |program, args, input| {
        run_with_timeout(program, args, input, SYNTAX_CHECK_TIMEOUT).map_err(RunError::from)
    })
}

fn run_with_timeout(
    program: &str,
    args: &[&str],
    input: &str,
    timeout: Duration,
) -> Result<SyntaxRun, SyntaxRunError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| SyntaxRunError::Spawn {
            program: program.to_owned(),
            source,
        })?;

    let (sender, receiver) = mpsc::channel();
    let pipe = child.stderr.take();

    // Read concurrently: a parser blocked on a full pipe would never exit.
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = pipe {
            pipe.read_to_end(&mut buffer).ok();
        }

        sender
            .send(String::from_utf8_lossy(&buffer).into_owned())
            .ok();
    });

    // Written from a thread as well: a parser that stops reading must not
    // block the timeout. Dropping the pipe at the end of the thread sends EOF.
    let feed = child.stdin.take();
    let text = input.to_owned();
    std::thread::spawn(move || {
        if let Some(mut writer) = feed {
            std::io::Write::write_all(&mut writer, text.as_bytes()).ok();
        }
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(source) => {
                child.kill().ok();
                child.wait().ok();
                return Err(SyntaxRunError::Wait {
                    program: program.to_owned(),
                    source,
                });
            }
        }

        if started.elapsed() > timeout {
            child.kill().ok();
            child.wait().ok();
            return Err(SyntaxRunError::Timeout {
                program: program.to_owned(),
                after: timeout,
            });
        }

        std::thread::sleep(POLL_INTERVAL);
    };

    Ok(SyntaxRun {
        success: status.success(),
        stderr: receiver.recv_timeout(PIPE_GRACE).unwrap_or_default(),
    })
}
