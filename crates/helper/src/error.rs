use std::fmt;
use std::path::PathBuf;
use std::process::ExitStatus;

use drakeflake_plan::{Refusal, exit};

use crate::USAGE;

#[derive(Debug)]
pub enum HelperError {
    Usage,
    ReadPlan(std::io::Error),
    InvalidPlan(serde_json::Error),
    PlanTooLarge,
    UnknownCaller,
    Refused(Refusal),
    #[cfg(target_os = "linux")]
    Denied(String),
    ScanFailed(String),
    #[cfg(target_os = "linux")]
    ScanIo {
        context: String,
        source: std::io::Error,
    },
    ApplyPlan(std::io::Error),
    ReadLock {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteLock {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidLock(String),
    Resolve {
        path: PathBuf,
        source: std::io::Error,
    },
    NotInStore(PathBuf),
    NotASystem(PathBuf),
    UnknownMode(String),
    NotAbsolute(PathBuf),
    NotAFlake(PathBuf),
    Spawn {
        program: String,
        source: std::io::Error,
    },
    Failed {
        program: String,
        status: ExitStatus,
    },
}

impl fmt::Display for HelperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => f.write_str(USAGE),
            Self::ReadPlan(err) => write!(f, "cannot read the plan from stdin: {err}"),
            Self::InvalidPlan(err) => write!(f, "invalid plan: {err}"),
            Self::PlanTooLarge => f.write_str("the plan is too large"),
            Self::UnknownCaller => f.write_str(
                "cannot tell who asked (neither PKEXEC_UID nor SUDO_UID is set), so no file of theirs is read",
            ),
            Self::Refused(reason) => write!(f, "refusing the plan: {reason}"),
            #[cfg(target_os = "linux")]
            Self::Denied(reason) => write!(f, "refusing the plan: {reason}"),
            Self::ScanFailed(reason) => write!(f, "cannot list the root file system: {reason}"),
            #[cfg(target_os = "linux")]
            Self::ScanIo { context, source } => write!(f, "{context}: {source}"),
            Self::ApplyPlan(err) => write!(f, "cannot apply the plan: {err}"),
            Self::ReadLock { path, source } => {
                write!(
                    f,
                    "cannot read the new lock file {}: {source}",
                    path.display()
                )
            }
            Self::WriteLock { path, source } => {
                write!(f, "cannot write {}: {source}", path.display())
            }
            Self::InvalidLock(reason) => write!(f, "not a flake lock file: {reason}"),
            Self::Resolve { path, source } => {
                write!(f, "cannot resolve {}: {source}", path.display())
            }
            Self::NotInStore(path) => write!(f, "{} is not in the Nix store", path.display()),
            Self::NotASystem(path) => write!(f, "{} is not a NixOS system", path.display()),
            Self::UnknownMode(mode) => write!(f, "unknown activation mode \"{mode}\""),
            Self::NotAbsolute(path) => write!(f, "{} is not an absolute path", path.display()),
            Self::NotAFlake(path) => write!(f, "{} is not a flake directory", path.display()),
            Self::Spawn { program, source } => write!(f, "cannot run {program}: {source}"),
            Self::Failed { program, status } => write!(f, "{program} failed ({status})"),
        }
    }
}

impl std::error::Error for HelperError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadPlan(err) | Self::ApplyPlan(err) => Some(err),
            Self::InvalidPlan(err) => Some(err),
            Self::Refused(refusal) => Some(refusal),
            #[cfg(target_os = "linux")]
            Self::Denied(_) => None,
            Self::ReadLock { source, path: _ }
            | Self::WriteLock { source, path: _ }
            | Self::Resolve { source, path: _ }
            | Self::Spawn { source, program: _ } => Some(source),
            #[cfg(target_os = "linux")]
            Self::ScanIo { context: _, source } => Some(source),
            Self::Usage
            | Self::PlanTooLarge
            | Self::UnknownCaller
            | Self::ScanFailed(_)
            | Self::InvalidLock(_)
            | Self::NotInStore(_)
            | Self::NotASystem(_)
            | Self::UnknownMode(_)
            | Self::NotAbsolute(_)
            | Self::NotAFlake(_)
            | Self::Failed {
                program: _,
                status: _,
            } => None,
        }
    }
}

impl HelperError {
    #[must_use = "only maps the error to a code"]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage => exit::USAGE,
            #[cfg(target_os = "linux")]
            Self::Denied(_) => exit::REFUSED,
            Self::PlanTooLarge
            | Self::InvalidPlan(_)
            | Self::UnknownCaller
            | Self::Refused(_)
            | Self::InvalidLock(_)
            | Self::NotInStore(_)
            | Self::NotASystem(_)
            | Self::UnknownMode(_)
            | Self::NotAbsolute(_)
            | Self::NotAFlake(_) => exit::REFUSED,
            Self::ReadPlan(_)
            | Self::ScanFailed(_)
            | Self::ApplyPlan(_)
            | Self::ReadLock { path: _, source: _ }
            | Self::WriteLock { path: _, source: _ }
            | Self::Resolve { path: _, source: _ } => exit::IO,
            #[cfg(target_os = "linux")]
            Self::ScanIo {
                context: _,
                source: _,
            } => exit::IO,
            Self::Spawn {
                program: _,
                source: _,
            }
            | Self::Failed {
                program: _,
                status: _,
            } => exit::TOOL,
        }
    }
}
