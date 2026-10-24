use std::any::Any;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use drakeflake_functions::FunctionIndex;
use drakeflake_plan::ManagedState;
use drakeflake_render::{OptionName, PackageAttr, UserName};
use drakeflake_store::{OptionIndex, PackageIndex, ValueSnapshot};
use serde::Serialize;
use serde_json::Value;

use super::ids::{ConsoleSession, Generation, HomeEpoch};
use super::{cleanup, generations, history, portal, setup, term};

#[derive(Clone)]
pub enum OptionsMsg {
    Reset,
    Index(Arc<OptionIndex>),
    IndexError(String),
    Snapshot(Arc<ValueSnapshot>),
    SnapshotError(String),
    Value(OptionName, Result<Value, String>, Generation),
    /// The Home Manager user changed: `epoch` tells the answers still in
    /// flight for the previous user apart ([`Self::HomeValue`]).
    HomeReset {
        epoch: HomeEpoch,
    },
    HomeValue {
        name: OptionName,
        result: Result<Value, String>,
        generation: Generation,
        epoch: HomeEpoch,
    },
    TreeBuilt {
        number: u64,
        tree: Arc<dyn Any + Send + Sync>,
    },
}

pub enum PackagesMsg {
    Details {
        attr: PackageAttr,
        result: Result<Value, String>,
    },
    Reset,
    Index(Arc<PackageIndex>),
    IndexError(String),
    Installed(HashSet<PackageAttr>),
    Sets(Vec<String>),
    Commands {
        command: String,
        result: Result<Vec<PackageAttr>, CommandLookupError>,
    },
    CommandIndexReady,
    SetIndex {
        set: String,
        result: Result<Arc<PackageIndex>, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLookupError {
    MissingIndex,
    Failed(String),
}

impl From<String> for CommandLookupError {
    fn from(error: String) -> Self {
        Self::Failed(error)
    }
}

impl From<&str> for CommandLookupError {
    fn from(error: &str) -> Self {
        Self::Failed(error.to_owned())
    }
}

impl From<drakeflake_exec::ExecError> for CommandLookupError {
    fn from(error: drakeflake_exec::ExecError) -> Self {
        Self::Failed(error.to_string())
    }
}

pub enum ChangesMsg {
    Reset {
        keep_staged: bool,
    },
    Managed(Result<ManagedState, String>),
    Saved(ManagedState),
    Checked {
        name: OptionName,
        input: String,
        result: Result<drakeflake_flake::TypeCheck, String>,
    },
}

pub struct CheckRequest {
    pub name: OptionName,
    pub input: String,
    pub nix_value: String,
}

#[derive(Debug, Clone, Default)]
pub struct TargetInfo {
    pub flake: String,
    pub host: String,
    pub kind: drakeflake_flake::SystemKind,
    pub hosts: Vec<drakeflake_flake::Host>,
    pub managed_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningId {
    Output,
    Remote,
    Missing,
    Outside,
    NotImported,
    Untracked,
}

impl WarningId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Remote => "remote",
            Self::Missing => "missing",
            Self::Outside => "outside",
            Self::NotImported => "not-imported",
            Self::Untracked => "untracked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub id: WarningId,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prompt {
    pub text: String,
    pub password: bool,
    pub core: String,
}

pub enum SystemMsg {
    Target(TargetInfo),
    TargetError {
        flake: String,
        hosts: Vec<drakeflake_flake::Host>,
        error: String,
    },
    Running(bool),
    Finished {
        ok: bool,
        backup: Option<PathBuf>,
        commit: Option<String>,
    },
    Activities(Vec<String>),
    HomeUsers {
        users: Vec<UserName>,
        selected: UserName,
        error: Option<String>,
    },
    Warnings(Vec<Warning>),
    CheckUi,
}

pub enum ConsoleMsg {
    Lines(Vec<String>),
    Screen {
        session: ConsoleSession,
        changes: term::Changes,
    },
    Prompt(Option<Prompt>),
}

pub enum ImagesMsg {
    Formats(Result<Vec<String>, String>),
    Built { format: String, file: String },
    Saved,
}

pub enum GenerationsMsg {
    Generations(Vec<generations::Generation>),
    Usage(cleanup::Usage),
    History(Vec<history::HistoryEntry>),
}

pub enum InputsMsg {
    Flake(String),
    Lock(Result<Vec<drakeflake_flake::lock::LockedInput>, String>),
    Checks {
        flake: String,
        checks: Vec<(String, drakeflake_flake::lock::UpdateCheck)>,
    },
    Updated(bool),
}

pub enum FunctionsMsg {
    Loaded(Arc<FunctionIndex>),
    Missing(String),
    Failed(String),
}

pub enum SearchMsg {
    Flake {
        query: String,
        total: usize,
        hits: Vec<super::FlakeHit>,
    },
}

pub enum SetupMsg {
    ImportCheck(setup::ImportCheck),
    Picked(portal::Picked),
    FlakeCandidates(Vec<PathBuf>),
    TargetChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Save,
    Build,
    Test,
    Switch,
    Boot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownMode;

impl FromStr for Mode {
    type Err = UnknownMode;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "save" => Ok(Self::Save),
            "build" => Ok(Self::Build),
            "test" => Ok(Self::Test),
            "switch" => Ok(Self::Switch),
            "boot" => Ok(Self::Boot),
            _ => Err(UnknownMode),
        }
    }
}

impl Mode {
    pub(super) const fn verb(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Build => "build",
            Self::Test => "test",
            Self::Switch => "switch",
            Self::Boot => "boot",
        }
    }

    pub(super) const fn needs_root(self) -> bool {
        matches!(self, Self::Test | Self::Switch | Self::Boot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_serialise_as_before() {
        for id in [
            WarningId::Output,
            WarningId::Remote,
            WarningId::Missing,
            WarningId::Outside,
            WarningId::NotImported,
            WarningId::Untracked,
        ] {
            let warning = Warning {
                id,
                text: "text".to_owned(),
            };

            assert_eq!(
                serde_json::to_value(&warning).unwrap(),
                serde_json::json!({ "id": id.as_str(), "text": "text" })
            );
        }
    }

    #[test]
    fn modes() {
        assert_eq!("switch".parse(), Ok(Mode::Switch));
        assert_eq!("nope".parse::<Mode>(), Err(UnknownMode));
        assert!(!Mode::Build.needs_root());
    }
}
