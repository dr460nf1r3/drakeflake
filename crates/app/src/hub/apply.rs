mod save;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_file::ManagedStateExt as _;
use drakeflake_flake::Target;
use drakeflake_plan::ManagedState;

use super::operations::{Exclusive, Outcome, Reporting};
use super::session::Session;
use super::{ChangesMsg, Hub, HubError, Mode, reload, target};

pub struct Logged;

impl Exclusive {
    pub(super) fn save_and_rebuild(
        &self,
        plan: &ManagedState,
        mode: Mode,
        rebuild: impl FnOnce(&Self, &Target) -> Outcome,
    ) -> Outcome {
        let Some(target) = self.session().target() else {
            self.console().error("No system is selected yet.");
            return Outcome::Failed;
        };

        let Some((path, rendered)) = self.prepare_plan(&target, plan, mode) else {
            return Outcome::Failed;
        };

        let previous = reload::state_before_write(self.session(), self.nix(), &path);
        let Ok(written) = self.save_plan(&path, &rendered) else {
            return Outcome::Failed;
        };

        let prepared = super::commit::prepare(self, &target);
        self.track_written(&written);

        self.mail().changes.post(ChangesMsg::Saved(plan.clone()));

        if prepared.created_repository {
            self.session().note_written_layout();
            self.hub().restart_load();
        } else {
            reload::reload_after_write(self.hub(), self.nix(), previous.clone(), plan.clone());
        }

        let outcome = rebuild(self, &target);

        if let Some(dir) = prepared.dir.filter(|_| outcome.is_success()) {
            super::commit::commit_applied(self, &dir, previous.as_ref(), plan);
        }

        outcome
    }

    pub(super) fn prepare_plan(
        &self,
        target: &Target,
        plan: &ManagedState,
        mode: Mode,
    ) -> Option<(PathBuf, drakeflake_plan::Rendered)> {
        let (path, rendered) = match render_plan(self.session(), plan) {
            Ok(result) => result,
            Err(errors) => {
                for error in errors {
                    self.console().error(format!("Invalid value: {error}"));
                }

                return None;
            }
        };

        let texts = std::iter::once((path.as_path(), &rendered.entry)).chain(
            rendered
                .parts
                .iter()
                .map(|part| (part.path.as_path(), &part.text)),
        );

        for (file, text) in texts {
            if let Err(error) = drakeflake_file::check_syntax(self.nix().context(), text) {
                self.console().error(format!(
                    "Generated file {} failed the syntax check:\n{error}",
                    file.display()
                ));
                return None;
            }
        }

        let imported = target.local_dir().is_some_and(|dir| {
            drakeflake_file::is_inside(&dir, &path) && drakeflake_file::is_imported(&dir, &path)
        });

        if !imported && mode != Mode::Save {
            self.console().error(format!(
                "Not rebuilding: the flake does not import {}. Add it to the imports of host \"{}\" first (Save only still works).",
                path.display(),
                target.host
            ));
            return None;
        }

        Some((path, rendered))
    }

    fn run_restore(&self, backup: &Path) -> Outcome {
        let state = match fs::read_to_string(backup)
            .map_err(|err| format!("Cannot read {}: {err}", backup.display()))
            .and_then(|text| {
                ManagedState::parse(&text)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("{} holds no settings", backup.display()))
            }) {
            Ok(state) => state,
            Err(error) => {
                self.console().error(error);
                return Outcome::Failed;
            }
        };

        let Ok((path, rendered)) = render_plan(self.session(), &state).map_err(|errors| {
            for error in errors {
                self.console()
                    .error(format!("Invalid value in the backup: {error}"));
            }
        }) else {
            return Outcome::Failed;
        };

        let previous = reload::state_before_write(self.session(), self.nix(), &path);
        if self.save_plan(&path, &rendered).is_err() {
            return Outcome::Failed;
        }

        self.console()
            .log(format!("Restored the settings from {}", backup.display()));
        self.mail().changes.post(ChangesMsg::Managed(
            ManagedState::load(&path).map_err(|error| error.to_string()),
        ));
        reload::reload_after_write(self.hub(), self.nix(), previous, state);
        Outcome::Succeeded
    }
}

pub fn start(hub: &Arc<Hub>, plan: ManagedState, mode: Mode) -> Result<(), HubError> {
    log::info!("apply started ({})", mode.verb());
    hub.spawn_exclusive("apply", Reporting::Shown, move |run| {
        run.save_and_rebuild(&plan, mode, |run, target| {
            if mode == Mode::Save {
                Outcome::Succeeded
            } else {
                run.rebuild(target, mode)
            }
        })
    })
}

pub fn restore(hub: &Arc<Hub>, backup: PathBuf) -> Result<(), HubError> {
    log::info!("restoring {}", backup.display());
    hub.spawn_exclusive("restore", Reporting::Shown, move |run| {
        run.run_restore(&backup)
    })
}

pub fn track_in_git(hub: &Arc<Hub>) -> Result<(), HubError> {
    hub.spawn_exclusive("git", Reporting::Hidden, |run| {
        let Some(target) = run.session().target() else {
            log::warn!("git add requested without a selected system");
            return Outcome::Failed;
        };

        let path = match run.session().managed_file() {
            Ok(path) => path,
            Err(error) => {
                log::warn!("git add skipped, no managed file: {error}");
                return Outcome::Failed;
            }
        };

        let outcome = match drakeflake_file::git_track(run.nix().context(), &path) {
            Ok(()) => {
                run.console().log(format!(
                    "{} is now tracked by git.",
                    drakeflake_plan::FILE_NAME
                ));
                Outcome::Succeeded
            }
            Err(error) => {
                run.console().error(format!("git add failed: {error}"));
                Outcome::Failed
            }
        };

        target::check(run.hub().services(), run.nix(), &target);
        outcome
    })
}

pub fn render_plan(
    session: &Session,
    plan: &ManagedState,
) -> Result<(PathBuf, drakeflake_plan::Rendered), Vec<String>> {
    let path = session
        .managed_file()
        .map_err(|error| vec![error.to_string()])?;

    let layout = session.layout();
    let rendered = plan
        .render_layout(&layout, &drakeflake_file::settings_folder(&path))
        .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>())?;

    Ok((path, rendered))
}
