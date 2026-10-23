//! A plain update hits GitHub's API rate limit, where Nix keeps the old revision and still succeeds.

use super::{LockedInput, UpdateCheck, is_input_name};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct UpdatePlan {
    pub overrides: Vec<(String, String)>,
    pub update: Vec<String>,
    pub expected: Vec<(String, String)>,
    /// A tarball's "revision" is a release name, not a hex commit, so its URL is checked.
    pub expected_url: Vec<(String, String)>,
}

impl UpdatePlan {
    #[must_use = "the plan is only built, nothing is updated"]
    pub fn new(checked: &[(&LockedInput, UpdateCheck)], names: &[String]) -> Self {
        let mut plan = Self::default();

        // A name that is not a plain identifier would reach nix as an option.
        let wanted = |input: &LockedInput| {
            is_input_name(&input.name) && (names.is_empty() || names.contains(&input.name))
        };

        for (input, check) in checked.iter().filter(|(input, _)| wanted(input)) {
            match check {
                UpdateCheck::Available {
                    rev,
                    lock_url: Some(url),
                    compare_url: _,
                } => {
                    plan.overrides.push((input.name.clone(), url.clone()));

                    if rev.chars().all(|ch| ch.is_ascii_hexdigit()) {
                        plan.expected.push((input.name.clone(), rev.clone()));
                    } else {
                        plan.expected_url.push((input.name.clone(), url.clone()));
                    }
                }
                UpdateCheck::Available {
                    rev: _,
                    compare_url: _,
                    lock_url: _,
                }
                | UpdateCheck::Failed { error: _ } => {
                    plan.update.push(input.name.clone());
                }
                UpdateCheck::NotCheckable { reason: _ }
                    if input.follows.is_none() && !input.pinned =>
                {
                    plan.update.push(input.name.clone());
                }
                UpdateCheck::UpToDate | UpdateCheck::NotCheckable { reason: _ } => {}
            }
        }

        plan
    }

    #[must_use = "only checks the plan"]
    pub const fn is_empty(&self) -> bool {
        self.overrides.is_empty() && self.update.is_empty()
    }

    /// `flake lock` when only overrides remain, since an update without names updates every input.
    #[must_use = "the arguments are only built, nothing runs"]
    pub fn args(&self) -> Vec<String> {
        let command = if self.update.is_empty() {
            "lock"
        } else {
            "update"
        };

        ["flake".to_owned(), command.to_owned()]
            .into_iter()
            .chain(self.update.iter().cloned())
            .chain(
                self.overrides.iter().flat_map(|(name, url)| {
                    ["--override-input".to_owned(), name.clone(), url.clone()]
                }),
            )
            .collect()
    }

    #[must_use = "the inputs are only listed, nothing is reported"]
    pub fn not_updated(&self, after: &[LockedInput]) -> Vec<String> {
        let by_rev = self.expected.iter().filter(|(name, rev)| {
            !after
                .iter()
                .any(|input| input.name == *name && input.rev.as_ref() == Some(rev))
        });

        let by_url = self.expected_url.iter().filter(|(name, url)| {
            !after
                .iter()
                .any(|input| input.name == *name && input.locked_url.as_deref() == Some(url))
        });

        by_rev.chain(by_url).map(|(name, _)| name.clone()).collect()
    }
}

#[cfg(test)]
mod tests;
