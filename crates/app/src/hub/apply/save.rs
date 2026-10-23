use std::fs;
use std::path::{Path, PathBuf};

use drakeflake_file::{GitState, WriteMethod};

use super::super::operations::Exclusive;
use super::super::{helper_path, history};
use super::Logged;

impl Exclusive {
    pub(in crate::hub) fn save_plan(
        &self,
        path: &Path,
        rendered: &drakeflake_plan::Rendered,
    ) -> Result<drakeflake_plan::WritePlan, Logged> {
        let previous = fs::read_to_string(path)
            .inspect_err(|err| {
                if err.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("cannot read {}: {err}", path.display());
                }
            })
            .ok();

        let parts = PathBuf::from(drakeflake_file::settings_folder(path));
        let mut plan =
            drakeflake_plan::WritePlan::for_rendered(path, rendered, previous.as_deref(), &parts);

        self.drop_superseded_entry(path, &mut plan);

        let unchanged = plan.remove.is_empty()
            && plan.write.iter().all(|(file, text)| {
                fs::read_to_string(file).ok().as_deref() == Some(text.as_str())
            });

        if unchanged {
            self.console()
                .log(format!("{} is already up to date.", path.display()));

            return Ok(plan);
        }

        let backup = self.back_up(path)?;

        let method =
            drakeflake_file::write_files(self.nix().context(), &plan, helper_path().as_deref())
                .map_err(|error| {
                    self.console().error(error.to_string());
                    Logged
                })?;

        let how = if method == WriteMethod::Elevated {
            " with administrator rights"
        } else {
            ""
        };

        let removed = if plan.remove.is_empty() {
            String::new()
        } else {
            format!(", removed {} no longer used", plan.remove.len())
        };

        self.console()
            .log(format!("Saved {} file(s){how}{removed}", plan.write.len()));

        if let Some(saved) = backup {
            self.record_backup(saved);
        }

        Ok(plan)
    }

    pub(super) fn track_written(&self, plan: &drakeflake_plan::WritePlan) {
        for file in plan.write.keys() {
            if drakeflake_file::git_state(self.nix().context(), file) != GitState::Untracked {
                continue;
            }

            match drakeflake_file::git_track(self.nix().context(), file) {
                Ok(()) => self.console().log(format!(
                    "Marked {} for git (git add --intent-to-add) so the flake can see it",
                    file.display()
                )),
                Err(error) => self.console().warn(format!(
                    "Could not mark {} for git: {error}",
                    file.display()
                )),
            }
        }
    }

    fn back_up(&self, path: &Path) -> Result<Option<PathBuf>, Logged> {
        let backup = history::history_dir(path)
            .map_err(|error| error.to_string())
            .and_then(|dir| {
                drakeflake_file::backup_to(path, &dir).map_err(|error| error.to_string())
            })
            .map_err(|error| {
                self.console()
                    .error(format!("Not writing, backup failed: {error}"));
                Logged
            })?;

        if let Some(saved) = &backup {
            self.console().log(format!(
                "Backed up the previous file to {}",
                saved.display()
            ));
        }

        Ok(backup)
    }

    pub(in crate::hub) fn drop_superseded_entry(
        &self,
        entry: &Path,
        plan: &mut drakeflake_plan::WritePlan,
    ) {
        let superseded = drakeflake_file::superseded_files(entry);

        if superseded.is_empty() {
            return;
        }

        let import = entry
            .file_name()
            .is_some_and(|name| name == drakeflake_file::FOLDER_ENTRY)
            .then(|| entry.parent().map(Path::to_path_buf))
            .flatten()
            .unwrap_or_else(|| entry.to_path_buf());

        self.console().warn(format!(
            "The settings now start at {}; the old entry file is removed. Change your import to {}.",
            entry.display(),
            import.display()
        ));

        for file in superseded {
            if !plan.write.contains_key(&file) && !plan.remove.contains(&file) {
                plan.remove.push(file);
            }
        }
    }
}
