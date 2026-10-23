use crate::check::{InputUpdate, names};
use crate::error::CheckError;
use crate::health::Health;

pub enum Request {
    Check,
    Quit,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    pub updates: Vec<InputUpdate>,
    pub error: Option<String>,
    pub checking: bool,
    pub checked: bool,
    pub health: Health,
}

impl State {
    #[must_use = "this only builds the state, the tray shows it"]
    pub fn from_result(
        result: Result<Vec<InputUpdate>, CheckError>,
        known: &[InputUpdate],
    ) -> Self {
        match result {
            Ok(updates) => Self {
                updates,
                checked: true,
                ..Self::default()
            },
            Err(error) => Self {
                updates: known.to_vec(),
                error: Some(error.to_string()),
                checked: true,
                ..Self::default()
            },
        }
    }

    #[must_use = "this only tells whether the tray should ask for attention"]
    pub const fn needs_attention(&self) -> bool {
        !self.updates.is_empty() || self.health.needs_attention()
    }

    #[must_use = "this only formats the summary"]
    pub fn summary(&self) -> String {
        if self.checking {
            "Checking for updates…".to_owned()
        } else if let Some(error) = &self.error {
            format!("Could not check for updates: {error}")
        } else if !self.updates.is_empty() {
            format!("Updates available: {}", names(&self.updates))
        } else if self.checked {
            "Your flake inputs are up to date.".to_owned()
        } else {
            "Checks your flake inputs for updates.".to_owned()
        }
    }
}

#[cfg(test)]
mod tests;
