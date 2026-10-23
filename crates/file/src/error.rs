use std::fmt;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use drakeflake_exec::ExecError;
use drakeflake_plan::{Priority, Refusal, StateError, SyntaxError};
use drakeflake_render::{OptionName, PackageAttr};
use drakeflake_validate::{ExpressionError, InputError};

#[derive(Debug)]
pub enum FileError {
    Input(InputError),
    Expression(ExpressionError),
    Syntax(SyntaxError),
    PriorityNotApplicable(Priority),
    EmptyHost,
    HostScopeNotAllowed(OptionName),
    Package {
        attr: PackageAttr,
        source: InputError,
    },
    Option {
        name: OptionName,
        source: Box<Self>,
    },
    Encode(serde_json::Error),
    StateTooLarge(usize),
    State(StateError),
    Refused(Refusal),
    Write(io::Error),
    NoHelper,
    NoElevation,
    Exec(ExecError),
    HelperRefused(String),
    AuthenticationDismissed(String),
    HelperFailed(String),
    BackupDir {
        dir: PathBuf,
        source: io::Error,
    },
    Backup {
        path: PathBuf,
        source: io::Error,
    },
    NoLocalFlake,
    OutputNotAbsolute(String),
    OutputNotNix(String),
    NoOutputDirectory,
    Git(String),
}

impl FileError {
    #[must_use = "only inspects the error"]
    pub fn is_permission_denied(&self) -> bool {
        match self {
            Self::Write(source) => source.kind() == ErrorKind::PermissionDenied,
            Self::Refused(refusal) => refusal.io_kind() == Some(ErrorKind::PermissionDenied),
            Self::Input(_)
            | Self::Expression(_)
            | Self::Syntax(_)
            | Self::PriorityNotApplicable(_)
            | Self::EmptyHost
            | Self::HostScopeNotAllowed(_)
            | Self::Package { attr: _, source: _ }
            | Self::Option { name: _, source: _ }
            | Self::Encode(_)
            | Self::StateTooLarge(_)
            | Self::State(_)
            | Self::NoHelper
            | Self::NoElevation
            | Self::Exec(_)
            | Self::HelperRefused(_)
            | Self::AuthenticationDismissed(_)
            | Self::HelperFailed(_)
            | Self::BackupDir { dir: _, source: _ }
            | Self::Backup { path: _, source: _ }
            | Self::NoLocalFlake
            | Self::OutputNotAbsolute(_)
            | Self::OutputNotNix(_)
            | Self::NoOutputDirectory
            | Self::Git(_) => false,
        }
    }

    #[must_use = "only inspects the error"]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Exec(error) if error.is_cancelled())
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => error.fmt(f),
            Self::Expression(error) => error.fmt(f),
            Self::Syntax(error) => error.fmt(f),
            Self::PriorityNotApplicable(priority) => {
                write!(f, "the priority \"{priority}\" does not apply to this option")
            }
            Self::EmptyHost => f.write_str("the host name is empty"),
            Self::HostScopeNotAllowed(name) => {
                write!(f, "{name} cannot be limited to one host")
            }
            Self::Package { attr, source } => write!(f, "package {attr}: {source}"),
            Self::Option { name, source } => write!(f, "{name}: {source}"),
            Self::Encode(error) => error.fmt(f),
            Self::StateTooLarge(bytes) => {
                write!(f, "the settings are too large to write ({bytes} bytes)")
            }
            Self::State(error) => error.fmt(f),
            Self::Refused(refusal) => write!(f, "cannot write the settings: {refusal}"),
            Self::Write(error) => write!(f, "cannot write the settings: {error}"),
            Self::NoHelper => f.write_str(
                "no permission to write, and the privileged helper (drakeflake-helper) is not installed",
            ),
            Self::NoElevation => f.write_str(
                "no permission to write, and none of pkexec, run0 or the macOS administrator dialog is available",
            ),
            Self::Exec(error) => error.fmt(f),
            Self::HelperRefused(stderr)
            | Self::AuthenticationDismissed(stderr)
            | Self::HelperFailed(stderr) => write!(
                f,
                "could not write the settings with administrator rights: {stderr}"
            ),
            Self::BackupDir { dir, source } => {
                write!(f, "cannot create {}: {source}", dir.display())
            }
            Self::Backup { path, source } => {
                write!(f, "cannot back up {}: {source}", path.display())
            }
            Self::NoLocalFlake => f.write_str(
                "The flake is not a local directory; choose an output file in Settings",
            ),
            Self::OutputNotAbsolute(text) => {
                write!(f, "The output path \"{text}\" must be absolute")
            }
            Self::OutputNotNix(text) => write!(
                f,
                "The output path \"{text}\" must be a .nix file or an existing directory"
            ),
            Self::NoOutputDirectory => f.write_str("the output path has no directory"),
            Self::Git(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Input(error)
            | Self::Package {
                attr: _,
                source: error,
            } => Some(error),
            Self::Expression(error) => Some(error),
            Self::Syntax(error) => Some(error),
            Self::Option { name: _, source } => Some(source.as_ref()),
            Self::Encode(error) => Some(error),
            Self::State(error) => Some(error),
            Self::Refused(error) => Some(error),
            Self::Exec(error) => Some(error),
            Self::Write(source)
            | Self::BackupDir { dir: _, source }
            | Self::Backup { path: _, source } => Some(source),
            Self::StateTooLarge(_)
            | Self::PriorityNotApplicable(_)
            | Self::EmptyHost
            | Self::HostScopeNotAllowed(_)
            | Self::NoHelper
            | Self::NoElevation
            | Self::HelperRefused(_)
            | Self::AuthenticationDismissed(_)
            | Self::HelperFailed(_)
            | Self::NoLocalFlake
            | Self::OutputNotAbsolute(_)
            | Self::OutputNotNix(_)
            | Self::NoOutputDirectory
            | Self::Git(_) => None,
        }
    }
}

impl From<InputError> for FileError {
    fn from(error: InputError) -> Self {
        Self::Input(error)
    }
}

impl From<ExpressionError> for FileError {
    fn from(error: ExpressionError) -> Self {
        Self::Expression(error)
    }
}

impl From<SyntaxError> for FileError {
    fn from(error: SyntaxError) -> Self {
        Self::Syntax(error)
    }
}

impl From<StateError> for FileError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}

impl From<Refusal> for FileError {
    fn from(error: Refusal) -> Self {
        Self::Refused(error)
    }
}

impl From<ExecError> for FileError {
    fn from(error: ExecError) -> Self {
        Self::Exec(error)
    }
}

#[cfg(test)]
mod tests;
