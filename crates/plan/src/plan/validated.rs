use std::path::{Path, PathBuf};

use super::WritePlan;
use crate::header::state_line;

/// The only thing the writers accept, so a plan cannot reach the disk unchecked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPlan {
    plan: WritePlan,
}

impl ValidatedPlan {
    pub(super) const fn new(plan: WritePlan) -> Self {
        Self { plan }
    }

    /// The entry goes last, so a failure in between never leaves an entry that imports parts not yet written.
    #[must_use = "the order is only computed"]
    pub fn ordered_writes(&self) -> Vec<(&Path, &str)> {
        let (entries, parts): (Vec<_>, Vec<_>) = self
            .plan
            .write
            .iter()
            .map(|(path, text)| (path.as_path(), text.as_str()))
            .partition(|(_, text)| state_line(text).is_some());

        parts.into_iter().chain(entries).collect()
    }

    #[must_use = "only returns the paths"]
    pub fn removals(&self) -> &[PathBuf] {
        &self.plan.remove
    }

    #[must_use = "only returns the plan"]
    pub const fn plan(&self) -> &WritePlan {
        &self.plan
    }
}
