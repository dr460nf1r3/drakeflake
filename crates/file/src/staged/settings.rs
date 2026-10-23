use std::fmt;

use drakeflake_plan::{InputMode, ManagedOption, Priority};
use serde::Deserialize;
use serde_json::Value;

use crate::{FileError, ManagedOptionExt as _};

#[derive(Debug)]
pub enum StageError {
    NoOption,
    InvalidSettings(serde_json::Error),
    UnknownMode(String),
    UnknownPriority(String),
    Option(FileError),
    Package(drakeflake_validate::InputError),
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOption => f.write_str("No option selected"),
            Self::InvalidSettings(error) => write!(f, "Invalid settings: {error}"),
            Self::UnknownMode(mode) => write!(f, "Unknown input mode \"{mode}\""),
            Self::UnknownPriority(priority) => write!(f, "Unknown priority \"{priority}\""),
            Self::Option(error) => error.fmt(f),
            Self::Package(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for StageError {}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SettingsInput {
    mode: String,
    priority: String,
    host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityChoices {
    pub choices: Vec<Priority>,
    pub implicit: Priority,
}

impl SettingsInput {
    pub fn from_value(value: &Value) -> Result<Self, StageError> {
        if value.is_null() {
            return Ok(Self::default());
        }

        Self::deserialize(value).map_err(StageError::InvalidSettings)
    }

    pub fn apply(self, name: &str, mut option: ManagedOption) -> Result<ManagedOption, StageError> {
        option.mode = parse_mode(&self.mode)?;
        option.priority = parse_priority(&self.priority)?;
        let host = self.host.trim();
        option.host = (!host.is_empty()).then(|| host.into());

        option.check_settings(name).map_err(StageError::Option)?;
        Ok(option.normalized())
    }
}

pub fn parse_mode(text: &str) -> Result<InputMode, StageError> {
    match text {
        "" => Ok(InputMode::Typed),
        _ => text
            .parse()
            .map_err(|error: drakeflake_plan::UnknownMode| StageError::UnknownMode(error.0)),
    }
}

fn parse_priority(text: &str) -> Result<Option<Priority>, StageError> {
    match text {
        "" => Ok(None),
        _ => text
            .parse()
            .map(Some)
            .map_err(|error: drakeflake_plan::UnknownPriority| {
                StageError::UnknownPriority(error.0)
            }),
    }
}

#[must_use = "only derives the choices"]
pub fn priority_choices(type_name: &str, input: &str, mode: InputMode) -> PriorityChoices {
    let option = ManagedOption {
        mode,
        ..ManagedOption::typed(type_name, input)
    };

    PriorityChoices {
        choices: option.priority_choices(),
        implicit: option.implicit_priority(),
    }
}
