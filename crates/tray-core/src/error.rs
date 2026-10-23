use std::fmt;

use drakeflake_flake::{FlakeError, LockError};

#[derive(Debug)]
pub enum CheckError {
    NotLocal,
    Lock(LockError),
}

#[derive(Debug)]
pub enum EvaluationError {
    Flake(FlakeError),
    Incomplete,
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLocal => f.write_str("the configured flake is not a local directory"),
            Self::Lock(error) => error.fmt(f),
        }
    }
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flake(error) => error.fmt(f),
            Self::Incomplete => f.write_str("some parts could not be evaluated (see the log)"),
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lock(error) => Some(error),
            Self::NotLocal => None,
        }
    }
}

impl std::error::Error for EvaluationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Flake(error) => Some(error),
            Self::Incomplete => None,
        }
    }
}

impl From<LockError> for CheckError {
    fn from(error: LockError) -> Self {
        Self::Lock(error)
    }
}

impl From<FlakeError> for EvaluationError {
    fn from(error: FlakeError) -> Self {
        Self::Flake(error)
    }
}
