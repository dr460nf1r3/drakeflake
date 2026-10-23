use std::path::{Path, PathBuf};

use drakeflake_file::StagedChanges;
use drakeflake_plan::ManagedState;

use super::ChangeModel;
use crate::hub::ChangesMsg;

impl ChangeModel {
    fn staged_file(&self) -> Option<PathBuf> {
        let target = self.hub.target()?;
        staged_path(&staged_key(&target.flake, &target.host))
    }

    fn legacy_staged_file(&self) -> Option<PathBuf> {
        let target = self.hub.target()?;
        let key = format!("{}#{}", target.flake.trim(), target.host);
        (key != staged_key(&target.flake, &target.host))
            .then(|| staged_path(&key))
            .flatten()
    }

    pub(super) fn persist(&mut self) {
        if !self.loaded || !self.restored || self.changes == self.saved {
            return;
        }

        let Some(file) = self.staged_file() else {
            return;
        };

        match self.changes.save(&file) {
            Ok(()) => {
                log::debug!(
                    "saved {} staged options, {} packages to {}",
                    self.changes.options().len(),
                    self.changes.packages().len(),
                    file.display()
                );
                self.saved = self.changes.clone();
            }
            Err(error) => log::error!(
                "cannot save the staged edits to {}: {error}",
                file.display()
            ),
        }
    }

    fn restore_staged(&mut self) {
        let Some(file) = self.staged_file() else {
            return;
        };

        self.restored = true;
        self.saved = StagedChanges::default();

        let (restored, migrated) = match read_staged(&file) {
            Read::Found(restored) => {
                self.saved = restored.clone();
                (restored, None)
            }
            Read::Missing => {
                let Some(legacy) = self.legacy_staged_file() else {
                    return;
                };

                match read_staged(&legacy) {
                    Read::Found(restored) => (restored, Some(legacy)),
                    Read::Missing | Read::Unreadable => return,
                }
            }
            Read::Unreadable => return,
        };

        log::info!(
            "restored {} staged options, {} packages from {}",
            restored.options().len(),
            restored.packages().len(),
            migrated.as_deref().unwrap_or(&file).display()
        );

        self.merge_staged(restored);

        if let Some(legacy) = migrated
            && let Err(error) = std::fs::remove_file(&legacy)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!(
                "cannot remove the migrated staged edits {}: {error}",
                legacy.display()
            );
        }
    }

    pub(super) fn merge_staged(&mut self, restored: StagedChanges) {
        let managed = self.managed.clone();

        for (name, needed) in self.changes.merge(restored, &managed) {
            self.request_check(&name, needed.key, needed.nix_value);
        }
    }

    pub(super) fn handle(&mut self, message: ChangesMsg) {
        match message {
            ChangesMsg::Reset { keep_staged } => {
                self.managed = ManagedState::default();
                self.managed_error.clear();
                self.loaded = false;

                if !keep_staged {
                    log::debug!(
                        "system changed: dropping {} staged options, {} packages",
                        self.changes.options().len(),
                        self.changes.packages().len()
                    );
                    self.changes.clear();
                    self.checks.clear();
                }
            }
            ChangesMsg::Managed(Ok(state)) => {
                log::debug!(
                    "managed file read: {} options, {} packages",
                    state.options.len(),
                    state.packages.len()
                );
                self.set_managed(state);
                self.restore_staged();
            }
            ChangesMsg::Managed(Err(error)) => {
                log::warn!("cannot read the managed file: {error}");
                self.set_managed(ManagedState::default());
                self.managed_error = error;
            }
            ChangesMsg::Saved(plan) => {
                log::info!(
                    "saved: {} options, {} packages now managed",
                    plan.options.len(),
                    plan.packages.len()
                );
                self.set_managed(plan);
            }
            ChangesMsg::Checked {
                name,
                input,
                result,
            } => self.on_checked(name, input, result),
        }
    }

    fn set_managed(&mut self, state: ManagedState) {
        self.managed = state;
        self.managed_error.clear();
        self.loaded = true;
    }
}

enum Read {
    Missing,
    Unreadable,
    Found(StagedChanges),
}

fn read_staged(file: &Path) -> Read {
    match StagedChanges::load(file) {
        Ok(Some(found)) => Read::Found(found),
        Ok(None) => Read::Missing,
        Err(error) => {
            log::warn!("cannot read the staged edits {}: {error}", file.display());
            Read::Unreadable
        }
    }
}

pub(super) fn staged_path(key: &str) -> Option<PathBuf> {
    let dir = drakeflake_file::staged_dir()
        .inspect_err(|err| log::warn!("staged edits are not kept: {err}"))
        .ok()?;

    Some(dir.join(format!(
        "{:016x}.json",
        drakeflake_fs::fnv1a(key.as_bytes())
    )))
}

pub(super) fn staged_key(flake: &str, host: &str) -> String {
    let dir = drakeflake_file::resolve_existing(Path::new(flake.trim()));
    format!("{}#{}", dir.display(), host)
}
