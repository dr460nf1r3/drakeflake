use std::fmt;

use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    Github,
    Gitlab,
    Sourcehut,
    Git,
    Path,
    Tarball,
    File,
    Indirect,
    Follows,
    Other(String),
}

impl Default for InputKind {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl InputKind {
    #[must_use = "only names the kind"]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Sourcehut => "sourcehut",
            Self::Git => "git",
            Self::Path => "path",
            Self::Tarball => "tarball",
            Self::File => "file",
            Self::Indirect => "indirect",
            Self::Follows => "follows",

            Self::Other(name) => name,
        }
    }

    #[must_use = "only classifies the kind"]
    pub const fn is_forge(&self) -> bool {
        matches!(self, Self::Github | Self::Gitlab | Self::Sourcehut)
    }
}

impl From<&str> for InputKind {
    fn from(name: &str) -> Self {
        match name {
            "github" => Self::Github,
            "gitlab" => Self::Gitlab,
            "sourcehut" => Self::Sourcehut,
            "git" => Self::Git,
            "path" => Self::Path,
            "tarball" => Self::Tarball,
            "file" => Self::File,
            "indirect" => Self::Indirect,
            "follows" => Self::Follows,

            other => Self::Other(other.to_owned()),
        }
    }
}

impl fmt::Display for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for InputKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests;
