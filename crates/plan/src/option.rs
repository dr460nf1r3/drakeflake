mod priority;

use drakeflake_render::HostName;
pub use priority::{InputMode, Priority, UnknownMode, UnknownPriority};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedOption {
    #[serde(rename = "type")]
    pub type_name: String,
    pub input: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(default, skip_serializing_if = "InputMode::is_typed")]
    pub mode: InputMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostName>,
}

impl ManagedOption {
    #[must_use = "only constructs the value"]
    pub fn typed(type_name: &str, input: &str) -> Self {
        Self {
            type_name: type_name.to_owned(),
            input: input.to_owned(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests;
