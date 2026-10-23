use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::{Nix, Target};
use drakeflake_plan::ManagedState;

use super::apply::render_plan;
use super::operations::{Exclusive, Outcome, Reporting};
use super::{Hub, SetupMsg, reload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportCheck {
    Imported,
    Missing,
    Failed(String),
}

struct Checked {
    target: Target,
    relative: PathBuf,
}

impl Exclusive {
    fn write_for_import_check(&self) -> Result<Checked, ImportCheck> {
        let Some(target) = self.session().target() else {
            return Err(ImportCheck::Failed("No system is selected yet.".to_owned()));
        };

        let Some(flake_dir) = target.local_dir().and_then(|dir| dir.canonicalize().ok()) else {
            return Err(ImportCheck::Failed(
                "The flake is not a local folder.".to_owned(),
            ));
        };

        let path = self
            .session()
            .managed_file()
            .map_err(|error| ImportCheck::Failed(error.to_string()))?;

        let Some(relative) = relative_to(&path, &flake_dir) else {
            return Err(ImportCheck::Failed(format!(
                "{} is not inside the flake folder {}.",
                path.display(),
                flake_dir.display()
            )));
        };

        let loaded = ManagedState::load(&path);

        let keeps_values = loaded.as_ref().is_ok_and(ManagedState::is_empty)
            && reload::flake_matches_cache(self.session(), self.nix(), &target);

        let state = loaded.unwrap_or_else(|error| {
            log::warn!(
                "cannot read {}: {error}; checking with empty settings",
                path.display()
            );
            ManagedState::default()
        });

        let (entry, rendered) = render_plan(self.session(), &state)
            .map_err(|errors| ImportCheck::Failed(errors.join("\n")))?;

        if self.save_plan(&entry, &rendered).is_err() {
            return Err(ImportCheck::Failed(format!(
                "Could not write {}; see the console.",
                entry.display()
            )));
        }

        if keeps_values {
            reload::adopt_fingerprint(self.session(), self.nix(), &target);
        }

        Ok(Checked { target, relative })
    }
}

pub fn scan_flakes(owner: &Arc<Hub>) {
    owner.spawn_background("flake-scan", |hub, _| {
        let started = std::time::Instant::now();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let found = drakeflake_flake::find_system_flakes(home.as_deref());

        log::debug!(
            "found {} system flakes in {:.1?}",
            found.len(),
            started.elapsed()
        );
        hub.mail.setup.post(SetupMsg::FlakeCandidates(found));
    });
}

pub fn check_import(owner: &Arc<Hub>) {
    let started = owner.spawn_exclusive("import-check", Reporting::Hidden, |run| {
        match run.write_for_import_check() {
            Ok(checked) => {
                run.hub().spawn_background("import-eval", move |hub, nix| {
                    let import = {
                        let _activity = hub.console().activity(
                            "import-check",
                            "Checking that your configuration imports the generated file…",
                        );

                        evaluate_import(nix, &checked.target, &checked.relative)
                    };

                    hub.mail.setup.post(SetupMsg::ImportCheck(import));
                });
                Outcome::Succeeded
            }
            Err(failed) => {
                run.mail().setup.post(SetupMsg::ImportCheck(failed));
                Outcome::Failed
            }
        }
    });

    if let Err(error) = started {
        owner
            .mail
            .setup
            .post(SetupMsg::ImportCheck(ImportCheck::Failed(
                error.to_string(),
            )));
    }
}

fn evaluate_import(nix: &Nix, target: &Target, relative: &Path) -> ImportCheck {
    match drakeflake_flake::module_arg_files(nix, target) {
        Ok(files) if drakeflake_flake::lists_file(&files, relative) => {
            log::info!(
                "{}#{} imports {}",
                target.flake,
                target.host,
                relative.display()
            );
            ImportCheck::Imported
        }
        Ok(files) => {
            log::info!(
                "{}#{} does not import {} ({} files define _module.args)",
                target.flake,
                target.host,
                relative.display(),
                files.len()
            );
            ImportCheck::Missing
        }
        Err(error) => {
            log::warn!("import check failed: {error}");
            ImportCheck::Failed(error.to_string())
        }
    }
}

fn relative_to(path: &Path, dir: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let name = path.file_name()?;

    let real = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());

    real.strip_prefix(dir)
        .or_else(|_| parent.strip_prefix(dir))
        .ok()
        .map(|inside| inside.join(name))
}

#[must_use = "the line is only built, not written to a file"]
pub fn import_line(entry: &Path, flake_dir: &Path) -> Option<String> {
    let dir = flake_dir.canonicalize().ok()?;
    let relative = relative_to(entry, &dir).or_else(|| relative_to(entry, flake_dir))?;

    let imported = if relative
        .file_name()
        .is_some_and(|name| name == drakeflake_file::FOLDER_ENTRY)
    {
        relative.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        relative
    };

    Some(format!("imports = [ ./{} ];", imported.display()))
}

#[cfg(test)]
mod tests;
