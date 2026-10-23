use drakeflake_flake::Nix;
use drakeflake_flake::lock::{LockedInput, UpdateCheck};

use crate::error::CheckError;

const DAY_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputUpdate {
    pub name: String,
    pub locked_at: Option<i64>,
    pub link: Option<String>,
}

impl InputUpdate {
    fn new(input: &LockedInput, check: UpdateCheck) -> Option<Self> {
        let UpdateCheck::Available {
            compare_url,
            rev: _,
            lock_url: _,
        } = check
        else {
            return None;
        };

        Some(Self {
            name: input.name.clone(),
            locked_at: input.last_modified,
            link: compare_url.or_else(|| input.web_url.clone()),
        })
    }

    #[must_use = "this only formats the label"]
    pub fn label(&self, now: i64) -> String {
        let Some(locked_at) = self.locked_at else {
            return self.name.clone();
        };

        let age = match (now - locked_at).max(0) / DAY_SECONDS {
            0 => "today".to_owned(),
            1 => "1 day ago".to_owned(),
            days => format!("{days} days ago"),
        };

        format!("{} — locked {age}", self.name)
    }
}

pub fn check(nix: &Nix) -> Result<Vec<InputUpdate>, CheckError> {
    let (config, _) = drakeflake_config::load();
    let dir = drakeflake_flake::local_flake_dir(&config.nix.flake_path).ok_or_else(|| {
        drakeflake_log::debug!("{} is not a local flake", config.nix.flake_path);
        CheckError::NotLocal
    })?;

    drakeflake_log::debug!("checking inputs of {}", dir.display());
    let inputs = drakeflake_flake::lock::read_lock(&dir)?;
    let checked = drakeflake_flake::lock::check_updates(nix.context(), &inputs);

    Ok(checked
        .into_iter()
        .filter_map(|(input, check)| InputUpdate::new(input, check))
        .collect())
}

#[must_use = "this only joins the names"]
pub fn names(updates: &[InputUpdate]) -> String {
    updates
        .iter()
        .map(|update| update.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn report(
    result: &Result<Vec<InputUpdate>, CheckError>,
    notified: &mut Vec<String>,
    notify: impl Fn(&str, &str),
) {
    match result {
        Ok(updates) if updates.is_empty() => drakeflake_log::info!("inputs up to date"),
        Ok(updates) => drakeflake_log::info!("updates available: {}", names(updates)),
        Err(error) => drakeflake_log::warn!("check failed: {error}"),
    }

    let Ok(updates) = result else {
        return;
    };

    let current: Vec<String> = updates.iter().map(|update| update.name.clone()).collect();

    // Once everything is up to date, the next update of the same input is news again.
    if current.is_empty() {
        notified.clear();
        return;
    }

    if current != *notified {
        notify(
            "Flake updates available",
            &format!(
                "{} can be updated. Open DrakeFlake → Inputs to update.",
                names(updates)
            ),
        );
        *notified = current;
    }
}

#[cfg(test)]
mod tests;
