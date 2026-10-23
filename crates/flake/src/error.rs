use std::path::PathBuf;
use std::{fmt, io};

use drakeflake_exec::ExecError;
use drakeflake_render::HostName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlakeProblem {
    Empty,
    HasFragment,
    /// It would be read as an option of `nix`.
    StartsWithDash,
    ControlCharacters,
    NotUtf8,
    /// Relative inputs would resolve against the evaluation directory.
    RelativePath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostProblem {
    Empty,
    Unsupported(String),
}

#[derive(Debug)]
pub enum LockError {
    Read { path: PathBuf, source: io::Error },
    NotJson(serde_json::Error),
    Malformed(serde_json::Error),
    Version(Option<u32>),
    MissingNode { input: String, node: String },
    NoNodes,
    NoRootNode,
    Git(String),
    NotFoundUpstream(String),
    Curl(String),
    CurlUnavailable(ExecError),
    Exec(ExecError),
}

#[derive(Debug)]
pub enum FlakeError {
    Exec(ExecError),
    InvalidFlake(FlakeProblem),
    InvalidHost(HostProblem),
    Nix(String),
    /// Missing on NixOS older than 25.05, e.g. `images`.
    MissingAttribute {
        attr: String,
        summary: String,
    },
    UnexpectedOutput {
        what: String,
        source: serde_json::Error,
    },
    Lock(LockError),
    NoContentHash,
    NoOutputPath,
    NoOptionsJson(String),
    NotPackageSet(String),
    GaveUp {
        skipped: usize,
    },
    HostNotDeclared {
        host: HostName,
        found: String,
    },
    NoHostsDeclared,
    SeveralHosts(String),
    ThreadCrashed(&'static str),
    ManualBuilds {
        forced: String,
        plain: String,
    },
}

impl FlakeError {
    #[must_use = "only checks the variant"]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Exec(error) if error.is_cancelled())
    }

    #[must_use = "only checks the variant"]
    pub const fn is_timeout(&self) -> bool {
        matches!(self, Self::Exec(error) if error.is_timeout())
    }

    #[must_use = "only reads the variant"]
    pub fn missing_attribute(&self) -> Option<&str> {
        match self {
            Self::MissingAttribute { attr, summary: _ } => Some(attr),
            Self::Exec(_)
            | Self::InvalidFlake(_)
            | Self::InvalidHost(_)
            | Self::Nix(_)
            | Self::UnexpectedOutput { what: _, source: _ }
            | Self::Lock(_)
            | Self::NoContentHash
            | Self::NoOutputPath
            | Self::NoOptionsJson(_)
            | Self::NotPackageSet(_)
            | Self::GaveUp { skipped: _ }
            | Self::HostNotDeclared { host: _, found: _ }
            | Self::NoHostsDeclared
            | Self::SeveralHosts(_)
            | Self::ThreadCrashed(_)
            | Self::ManualBuilds {
                forced: _,
                plain: _,
            } => None,
        }
    }
}

impl fmt::Display for FlakeProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "No flake configured",
            Self::HasFragment => "The flake path must not contain '#'; set the host separately",
            Self::StartsWithDash => "The flake path must not start with '-'",
            Self::ControlCharacters => "The flake path contains control characters",
            Self::NotUtf8 => "The flake path is not valid UTF-8",
            Self::RelativePath => "Local flake paths must be absolute, starting with '/' or '~/'",
        })
    }
}

impl fmt::Display for HostProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("No host selected"),
            Self::Unsupported(host) => write!(f, "Unsupported host name \"{host}\""),
        }
    }
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "Cannot read {}: {source}", path.display())
            }
            Self::NotJson(err) => write!(f, "flake.lock is not valid JSON: {err}"),
            Self::Malformed(err) => write!(f, "flake.lock is malformed: {err}"),
            Self::Version(Some(version)) => {
                write!(
                    f,
                    "flake.lock has version {version}, version 7 or newer is supported"
                )
            }
            Self::Version(None) => f.write_str("flake.lock has no version"),
            Self::MissingNode { input, node } => {
                write!(
                    f,
                    "flake.lock: input {input} points to the missing node {node}"
                )
            }
            Self::NoNodes => f.write_str("flake.lock has no nodes"),
            Self::NoRootNode => f.write_str("flake.lock has no root node"),
            Self::Git(message) | Self::Curl(message) => f.write_str(message),
            Self::NotFoundUpstream(pattern) => write!(f, "{pattern} not found upstream"),
            Self::CurlUnavailable(err) => write!(f, "{err} (is curl installed?)"),
            Self::Exec(err) => err.fmt(f),
        }
    }
}

impl fmt::Display for FlakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exec(err) => err.fmt(f),
            Self::InvalidFlake(problem) => problem.fmt(f),
            Self::InvalidHost(problem) => problem.fmt(f),
            Self::Nix(summary) | Self::MissingAttribute { attr: _, summary } => {
                f.write_str(summary)
            }
            Self::UnexpectedOutput { what, source } => {
                write!(f, "unexpected {what} output: {source}")
            }
            Self::Lock(err) => err.fmt(f),
            Self::NoContentHash => f.write_str("the flake metadata has no content hash"),
            Self::NoOutputPath => f.write_str("nix build printed no output path"),
            Self::NoOptionsJson(out) => write!(f, "{out} has no options.json"),
            Self::NotPackageSet(set) => write!(f, "\"{set}\" is not a package set name"),
            Self::GaveUp { skipped } => write!(
                f,
                "gave up after skipping {skipped} entries that fail to evaluate"
            ),
            Self::HostNotDeclared { host, found } => write!(
                f,
                "Host \"{host}\" is not declared in this flake (found: {found})"
            ),
            Self::NoHostsDeclared => {
                f.write_str("This flake declares no nixosConfigurations or darwinConfigurations")
            }
            Self::SeveralHosts(names) => {
                write!(
                    f,
                    "Several hosts are declared ({names}); choose one in Settings"
                )
            }
            Self::ThreadCrashed(what) => write!(f, "{what} crashed"),
            Self::ManualBuilds { forced, plain } => write!(f, "{forced}\n{plain}"),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { path: _, source } => Some(source),
            Self::NotJson(err) | Self::Malformed(err) => Some(err),
            Self::CurlUnavailable(err) | Self::Exec(err) => Some(err),
            Self::NoNodes
            | Self::NoRootNode
            | Self::Version(_)
            | Self::MissingNode { input: _, node: _ }
            | Self::Git(_)
            | Self::NotFoundUpstream(_)
            | Self::Curl(_) => None,
        }
    }
}

impl std::error::Error for FlakeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Exec(err) => Some(err),
            Self::UnexpectedOutput { what: _, source } => Some(source),
            Self::Lock(err) => Some(err),
            Self::InvalidFlake(_)
            | Self::InvalidHost(_)
            | Self::Nix(_)
            | Self::MissingAttribute {
                attr: _,
                summary: _,
            }
            | Self::NoContentHash
            | Self::NoOutputPath
            | Self::NoOptionsJson(_)
            | Self::NotPackageSet(_)
            | Self::GaveUp { skipped: _ }
            | Self::HostNotDeclared { host: _, found: _ }
            | Self::NoHostsDeclared
            | Self::SeveralHosts(_)
            | Self::ThreadCrashed(_)
            | Self::ManualBuilds {
                forced: _,
                plain: _,
            } => None,
        }
    }
}

impl From<ExecError> for FlakeError {
    fn from(error: ExecError) -> Self {
        Self::Exec(error)
    }
}

impl From<LockError> for FlakeError {
    fn from(error: LockError) -> Self {
        Self::Lock(error)
    }
}

impl From<ExecError> for LockError {
    fn from(error: ExecError) -> Self {
        Self::Exec(error)
    }
}
