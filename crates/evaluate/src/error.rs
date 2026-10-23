use std::path::PathBuf;
use std::{fmt, io};

use drakeflake_flake::FlakeError;

#[derive(Debug)]
pub enum EvaluateError {
    Flake(FlakeError),
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse(String),
    Fallback {
        build: Box<FlakeError>,
        eval: Box<FlakeError>,
    },
    Encode(serde_json::Error),
}

impl EvaluateError {
    #[must_use = "only inspects the error"]
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Flake(error) => error.is_cancelled(),
            Self::Fallback { build: _, eval } => eval.is_cancelled(),
            Self::Read { path: _, source: _ } | Self::Parse(_) | Self::Encode(_) => false,
        }
    }
}

impl fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flake(error) => error.fmt(f),
            Self::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            Self::Parse(message) => f.write_str(message),
            Self::Fallback { build, eval } => write!(f, "{build}\n{eval}"),
            Self::Encode(error) => write!(f, "cannot encode the persisted entries: {error}"),
        }
    }
}

impl std::error::Error for EvaluateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Flake(error) => Some(error),
            Self::Read { path: _, source } => Some(source),
            Self::Fallback { build: _, eval } => Some(eval.as_ref()),
            Self::Encode(error) => Some(error),
            Self::Parse(_) => None,
        }
    }
}

impl From<FlakeError> for EvaluateError {
    fn from(error: FlakeError) -> Self {
        Self::Flake(error)
    }
}
