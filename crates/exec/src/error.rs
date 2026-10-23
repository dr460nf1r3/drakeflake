use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    Cancelled(Option<String>),
    TimedOut {
        program: String,
        after: Duration,
    },
    Spawn {
        program: String,
        kind: std::io::ErrorKind,
    },
    Wait {
        program: String,
        kind: std::io::ErrorKind,
    },
    Failed(String),
}

impl ExecError {
    #[must_use = "only builds the error"]
    pub fn spawn(program: &str, error: &std::io::Error) -> Self {
        Self::Spawn {
            program: program.to_owned(),
            kind: error.kind(),
        }
    }

    #[must_use = "only builds the error"]
    pub fn wait(program: &str, error: &std::io::Error) -> Self {
        Self::Wait {
            program: program.to_owned(),
            kind: error.kind(),
        }
    }

    #[must_use = "only reads the kind"]
    pub const fn io_kind(&self) -> Option<std::io::ErrorKind> {
        match self {
            Self::Spawn { kind, program: _ } | Self::Wait { kind, program: _ } => Some(*kind),
            Self::Cancelled(_)
            | Self::Failed(_)
            | Self::TimedOut {
                program: _,
                after: _,
            } => None,
        }
    }
    #[must_use = "only checks the variant"]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled(_))
    }

    #[must_use = "only checks the variant"]
    pub const fn is_timeout(&self) -> bool {
        matches!(
            self,
            Self::TimedOut {
                program: _,
                after: _,
            }
        )
    }
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled(None) => f.write_str("cancelled"),
            Self::Cancelled(Some(program)) => write!(f, "{program} was cancelled"),
            Self::TimedOut { program, after } => {
                write!(f, "{program} timed out after {}s", after.as_secs())
            }
            Self::Spawn { program, kind } => write!(f, "cannot run {program}: {kind}"),
            Self::Wait { program, kind } => write!(f, "waiting for {program} failed: {kind}"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ExecError {}

#[cfg(test)]
mod tests;
