use std::fmt;

use drakeflake_file::check_key;
use drakeflake_flake::TypeCheck;
use drakeflake_render::OptionName;

use super::ChangeModel;
use crate::hub::CheckRequest;

pub const CHECK_PENDING: &str = "pending";
pub const CHECK_OK: &str = "ok";
pub const CHECK_INVALID: &str = "invalid";
pub const CHECK_FAILED: &str = "failed";
pub const CHECK_UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckState {
    Pending,
    Done(TypeCheck),
    Error(String),
}

impl fmt::Display for CheckState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pending => CHECK_PENDING,
            Self::Done(TypeCheck::Valid) => CHECK_OK,
            Self::Done(TypeCheck::Invalid(_)) => CHECK_INVALID,
            Self::Done(TypeCheck::Failed(_)) | Self::Error(_) => CHECK_FAILED,
            Self::Done(TypeCheck::Unknown) => CHECK_UNKNOWN,
        })
    }
}

impl CheckState {
    pub fn message(&self) -> String {
        match self {
            Self::Pending => "Checking the value against the option type…".to_owned(),
            Self::Done(TypeCheck::Valid) => "Matches the option type".to_owned(),
            Self::Done(TypeCheck::Invalid(type_name)) => format!("Not a valid {type_name}"),
            Self::Done(TypeCheck::Failed(message)) | Self::Error(message) => message.clone(),
            Self::Done(TypeCheck::Unknown) => {
                "The option type could not be checked; the build will validate it".to_owned()
            }
        }
    }

    pub const fn blocks(&self) -> bool {
        matches!(
            self,
            Self::Pending
                | Self::Error(_)
                | Self::Done(TypeCheck::Invalid(_) | TypeCheck::Failed(_))
        )
    }
}

impl ChangeModel {
    pub(super) fn check_of(&self, name: &OptionName) -> Option<&CheckState> {
        self.checks.get(name).map(|(_, state)| state)
    }

    pub(super) fn count_pending(&self) -> usize {
        self.checks
            .values()
            .filter(|(_, state)| *state == CheckState::Pending)
            .count()
    }

    pub(super) fn count_failed(&self) -> usize {
        self.checks
            .values()
            .filter(|(_, state)| state.blocks() && *state != CheckState::Pending)
            .count()
    }

    pub(super) fn request_check(&mut self, name: &OptionName, input: String, nix_value: String) {
        self.checks
            .insert(name.clone(), (input.clone(), CheckState::Pending));
        self.hub.request_check(CheckRequest {
            name: name.clone(),
            input,
            nix_value,
        });
    }

    pub(super) fn on_checked(
        &mut self,
        name: OptionName,
        input: String,
        result: Result<TypeCheck, String>,
    ) {
        let current = self
            .changes
            .staged_option(&name)
            .is_some_and(|option| check_key(option) == input);

        if !current {
            log::debug!("discarding stale type check of {name}");
            return;
        }

        let state = result.map_or_else(CheckState::Error, CheckState::Done);
        log::debug!("type check of {name}: {state}");
        self.checks.insert(name, (input, state));
    }
}
