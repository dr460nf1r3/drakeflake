use std::fmt;
use std::path::PathBuf;

use drakeflake_plan::ManagedState;
use serde::Deserialize as _;
use serde_json::Value;

use super::SystemModel;
use crate::hub::generations::GenerationAction;
use crate::hub::{Capabilities, HubError, Mode};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    #[default]
    Unknown,
    Ok,
    Failed,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unknown => "",
            Self::Ok => "ok",
            Self::Failed => "failed",
        })
    }
}

#[derive(Default)]
pub struct RunState {
    pub running: bool,
    pub last_result: Outcome,
    pub last_backup: Option<PathBuf>,
    pub last_commit: Option<String>,
}

impl RunState {
    pub fn can_restore(&self) -> bool {
        !self.running && self.last_result == Outcome::Failed && self.last_backup.is_some()
    }

    pub fn finished(&mut self, ok: bool, backup: Option<PathBuf>, commit: Option<String>) {
        self.running = false;
        self.last_commit = commit;
        self.last_result = if ok { Outcome::Ok } else { Outcome::Failed };

        match &backup {
            Some(file) => log::info!(
                "run finished: {}, backup {}",
                self.last_result,
                file.display()
            ),
            None => log::info!("run finished: {}", self.last_result),
        }

        if backup.is_some() {
            self.last_backup = backup;
        }

        crate::macos::window::dock_bounce();
    }
}

impl SystemModel {
    pub(super) fn run_started(&mut self, result: Result<(), HubError>) {
        match result {
            Ok(()) => {
                self.run.last_result = Outcome::Unknown;
                self.run.last_commit = None;
                self.run.running = true;
            }
            Err(error) => self.hub.log_error(error.to_string()),
        }
    }

    pub(super) fn started(&mut self, result: Result<(), HubError>) -> String {
        let error = match result {
            Ok(()) => {
                self.run.last_result = Outcome::Unknown;
                self.run.last_commit = None;
                self.run.running = true;
                String::new()
            }
            Err(error) => {
                log::error!("cannot start: {error}");
                error.to_string()
            }
        };

        self.state_changed();
        error
    }

    pub(super) fn parse_apply(
        &self,
        plan: &Value,
        mode: &str,
        allowed: fn(Capabilities) -> &'static [Mode],
    ) -> Option<(ManagedState, Mode)> {
        let Some(parsed) = parse_mode(mode, allowed(self.target.capabilities())) else {
            self.hub.log_error(format!(
                "Apply mode \"{mode}\" is not available for this host here"
            ));
            return None;
        };

        match ManagedState::deserialize(plan) {
            Ok(plan) => {
                log::info!(
                    "{mode} requested: {} options, {} packages in the plan",
                    plan.options.len(),
                    plan.packages.len()
                );
                Some((plan, parsed))
            }
            Err(err) => {
                self.hub.log_error(format!("Invalid plan: {err}"));
                None
            }
        }
    }
}

impl SystemModel {
    pub(super) fn start_rebuild(&mut self, mode: &str) -> String {
        let Some(parsed) = parse_mode(mode, self.target.capabilities().rebuild) else {
            log::warn!("ignoring rebuild mode {mode:?}, not available for this host here");
            return format!("Rebuild mode \"{mode}\" is not available for this host here");
        };

        log::info!("rebuild ({mode}) requested");
        let result = self.hub.rebuild_only(parsed);
        self.started(result)
    }

    pub(super) fn start_lock_update(&mut self, names: &Value) -> String {
        let names = match Vec::<String>::deserialize(names) {
            Ok(names) => names,
            Err(err) => {
                log::warn!("invalid input list from QML: {err}");
                return format!("Invalid input list: {err}");
            }
        };

        log::info!("updating flake inputs {names:?} (empty: all)");
        let result = self.hub.update_lock(names);
        self.started(result)
    }

    pub(super) fn start_restore_backup(&mut self) {
        let Some(backup) = self.run.last_backup.take() else {
            log::warn!("restore requested but there is no backup");
            return;
        };

        log::info!("restoring backup {}", backup.display());
        let result = self.hub.restore(backup);
        self.run_started(result);
        self.state_changed();
    }

    pub(super) fn start_generation_action(&mut self, id: i32, action: &str) {
        let parsed = action
            .parse::<GenerationAction>()
            .map_err(|error| HubError::invalid(error.to_string()));

        let result = parsed
            .and_then(|parsed| {
                let id = u32::try_from(id)
                    .ok()
                    .ok_or_else(|| HubError::invalid(format!("Invalid generation {id}")))?;

                self.hub.generation_action(id, parsed)
            })
            .inspect(|()| log::info!("generation {id}: {action} started"));

        self.run_started(result);
        self.state_changed();
    }
}

pub fn parse_mode(text: &str, allowed: &[Mode]) -> Option<Mode> {
    text.parse().ok().filter(|mode| allowed.contains(mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_per_run() {
        use drakeflake_flake::SystemKind;

        let nixos = Capabilities::of(SystemKind::NixOs, SystemKind::NixOs);
        assert_eq!(parse_mode("save", nixos.apply), Some(Mode::Save));
        assert_eq!(parse_mode("switch", nixos.review), Some(Mode::Switch));
        assert_eq!(parse_mode("build", nixos.review), None);
        assert_eq!(parse_mode("save", nixos.review), None);
        assert_eq!(parse_mode("save", nixos.rebuild), None);
        assert_eq!(parse_mode("boot", nixos.rebuild), Some(Mode::Boot));
        assert_eq!(parse_mode("nope", nixos.apply), None);

        let darwin = Capabilities::of(SystemKind::Darwin, SystemKind::Darwin);
        assert_eq!(parse_mode("test", darwin.apply), None);
        assert_eq!(parse_mode("boot", darwin.review), None);
        let elsewhere = Capabilities::of(SystemKind::Darwin, SystemKind::NixOs);
        assert_eq!(parse_mode("switch", elsewhere.apply), None);
        assert_eq!(parse_mode("save", elsewhere.apply), Some(Mode::Save));
    }
}
