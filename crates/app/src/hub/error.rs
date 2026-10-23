use std::error::Error;
use std::fmt;

use drakeflake_file::FileError;

#[derive(Debug)]
pub enum HubError {
    Busy,
    NoTarget,
    NoLocalFlake,
    Invalid(String),
    Library(Box<dyn Error + Send + Sync>),
}

impl HubError {
    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }
}

impl fmt::Display for HubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => f.write_str("Another operation is still running"),
            Self::NoTarget => f.write_str("No system is selected yet"),
            Self::NoLocalFlake => f.write_str("The flake is not a local folder"),
            Self::Invalid(reason) => f.write_str(reason),
            Self::Library(error) => error.fmt(f),
        }
    }
}

impl Error for HubError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error.as_ref()),
            Self::Busy | Self::NoTarget | Self::NoLocalFlake | Self::Invalid(_) => None,
        }
    }
}

impl From<FileError> for HubError {
    fn from(error: FileError) -> Self {
        if matches!(error, FileError::NoLocalFlake) {
            Self::NoLocalFlake
        } else {
            Self::Library(Box::new(error))
        }
    }
}

#[cfg(test)]
mod tests;
