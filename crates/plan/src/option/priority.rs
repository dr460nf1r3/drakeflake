use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Default,
    Normal,
    Override,
    Force,
    Merge,
    Before,
    After,
    Replace,
}

impl Priority {
    const ALL: [Self; 8] = [
        Self::Default,
        Self::Normal,
        Self::Override,
        Self::Force,
        Self::Merge,
        Self::Before,
        Self::After,
        Self::Replace,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Normal => "normal",
            Self::Override => "override",
            Self::Force => "force",
            Self::Merge => "merge",
            Self::Before => "before",
            Self::After => "after",
            Self::Replace => "replace",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPriority(pub String);

impl fmt::Display for UnknownPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown priority \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownPriority {}

impl FromStr for Priority {
    type Err = UnknownPriority;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|priority| priority.name() == name)
            .ok_or_else(|| UnknownPriority(name.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputMode {
    #[default]
    Typed,
    Expression,
}

impl InputMode {
    #[must_use = "only checks the variant"]
    pub const fn is_typed(&self) -> bool {
        matches!(self, Self::Typed)
    }
}

impl fmt::Display for InputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Typed => "typed",
            Self::Expression => "expression",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMode(pub String);

impl fmt::Display for UnknownMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown input mode \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownMode {}

impl FromStr for InputMode {
    type Err = UnknownMode;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "typed" => Ok(Self::Typed),
            "expression" => Ok(Self::Expression),
            other => Err(UnknownMode(other.to_owned())),
        }
    }
}
