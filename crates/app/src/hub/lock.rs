use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::lock::{LockedInput, UpdateCheck, UpdatePlan};

use super::argv::{arg, echo};
use super::operations::{Exclusive, Outcome, Reporting};
use super::rebuild::console_elevation;
use super::{Hub, HubError, InputsMsg, helper_path};

impl Exclusive {
    fn update_lock(&self, dir: &Path, names: &[String]) -> Outcome {
        self.console().log(format!(
            "Updating {} in {}",
            describe_inputs(names),
            dir.join("flake.lock").display()
        ));

        let Some(plan) = self.plan_update(dir, names) else {
            return Outcome::Failed;
        };

        if plan.is_empty() {
            self.console().log("Everything is up to date.");
            return Outcome::Succeeded;
        }

        // The update always runs as the user: as root, `nix` would fetch any
        // input the flake names (`path:/etc/shadow`) into the readable store.
        let Ok(output) = self.lock_output(dir) else {
            return Outcome::Failed;
        };

        let mut args: Vec<OsString> = drakeflake_flake::NIX_FEATURES
            .iter()
            .map(arg)
            .chain(plan.args().iter().map(arg))
            .collect();

        if let Some(output) = &output {
            log::debug!(
                "flake.lock is not writable: updating into {} first",
                output.display()
            );
            if let Some(parent) = output.parent()
                && let Err(error) = std::fs::create_dir_all(parent)
            {
                self.console()
                    .error(format!("Cannot create {}: {error}", parent.display()));
                return Outcome::Failed;
            }

            args.extend([arg("--output-lock-file"), arg(output)]);
        }

        self.console().log(echo(OsStr::new("nix"), &args));
        let updated = self.stream_pty(OsStr::new("nix"), &args, dir);

        let done = output.map_or_else(
            || updated,
            |output| {
                let installed = updated.then(|| self.install_lock(dir, &output));
                if let Err(error) = std::fs::remove_file(&output)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    log::warn!("cannot remove {}: {error}", output.display());
                }

                installed
            },
        );

        done.then(|| self.verify_update(dir, &plan))
    }

    fn lock_output(&self, dir: &Path) -> Result<Option<PathBuf>, Outcome> {
        if lock_writable(dir) {
            return Ok(None);
        }

        match drakeflake_fs::state_dir() {
            Ok(state) => Ok(Some(
                state.join(format!("flake.lock.{}", std::process::id())),
            )),
            Err(error) => {
                self.console()
                    .error(format!("Not updating the lock: {error}"));
                Err(Outcome::Failed)
            }
        }
    }

    fn plan_update(&self, dir: &Path, names: &[String]) -> Option<UpdatePlan> {
        let inputs = drakeflake_flake::lock::read_lock(dir)
            .inspect_err(|error| {
                self.console()
                    .error(format!("Cannot read flake.lock: {error}"));
            })
            .ok()?;

        let checked = {
            let _activity = self
                .console()
                .activity("lock", "Checking flake inputs for updates…");

            drakeflake_flake::lock::check_updates(self.nix().context(), &inputs)
        };

        let plan = UpdatePlan::new(&checked, names);
        log::debug!("lock update plan: {plan:?}");
        Some(plan)
    }

    fn verify_update(&self, dir: &Path, plan: &UpdatePlan) -> Outcome {
        let after = match drakeflake_flake::lock::read_lock(dir) {
            Ok(after) => after,
            Err(error) => {
                self.console()
                    .warn(format!("Cannot read the updated flake.lock: {error}"));
                return Outcome::Succeeded;
            }
        };

        let stale = plan.not_updated(&after);

        if stale.is_empty() {
            Outcome::Succeeded
        } else {
            self.console().error(format!(
                "Not updated: {}. Nix kept its cached version (for GitHub inputs usually because GitHub's API rate limit was reached; an access token in nix.conf raises it).",
                stale.join(", ")
            ));
            Outcome::Failed
        }
    }

    fn install_lock(&self, dir: &Path, new_lock: &Path) -> Outcome {
        let (Some(helper), Some(elevation)) = (helper_path(), console_elevation(self.console()))
        else {
            self.console().error(
                "flake.lock is not writable and the privileged helper is not available; update it with `sudo nix flake update` in the flake directory.",
            );
            return Outcome::Failed;
        };

        log::debug!("installing the new flake.lock through {}", helper.display());

        let mut command = elevation.prefix();
        command.extend([arg(&helper), arg("install-lock"), arg(dir), arg(new_lock)]);

        self.work_dir().map_or(Outcome::Failed, |workdir| {
            self.stream_command(&command, &workdir)
        })
    }
}

pub fn load(hub: &Arc<Hub>, flake: String, check: bool) {
    hub.spawn_persistent("lock", move |hub, nix| {
        let result = read_flake_lock(&flake);
        match &result {
            Ok(inputs) => log::debug!("flake.lock of {flake}: {} inputs", inputs.len()),
            Err(error) => log::warn!("cannot read flake.lock of {flake}: {error}"),
        }

        let to_check = result.as_ref().ok().filter(|_| check).cloned();
        hub.mail.inputs.post(InputsMsg::Lock(result));

        if let Some(inputs) = to_check {
            let checked = {
                let _activity = hub
                    .console()
                    .activity("lock", "Checking flake inputs for updates…");

                drakeflake_flake::lock::check_updates(nix.context(), &inputs)
            };

            let checks: Vec<(String, UpdateCheck)> = checked
                .into_iter()
                .map(|(input, outcome)| (input.name.clone(), outcome))
                .collect();

            hub.mail.inputs.post(InputsMsg::Checks { flake, checks });
        }
    });
}

pub fn update(hub: &Arc<Hub>, names: Vec<String>) -> Result<(), HubError> {
    if let Some(bad) = names
        .iter()
        .find(|name| !drakeflake_flake::lock::is_input_name(name))
    {
        return Err(HubError::invalid(format!("\"{bad}\" is not an input name")));
    }

    let dir = hub
        .target()
        .and_then(|target| target.local_dir())
        .ok_or_else(|| {
            HubError::invalid("Flake inputs can only be updated for a local flake directory.")
        })?;

    hub.spawn_exclusive("update-lock", Reporting::Shown, move |run| {
        let outcome = run.update_lock(&dir, &names);
        if outcome.is_success() {
            log::info!("lock updated: {}", describe_inputs(&names));
        }

        run.mail()
            .inputs
            .post(InputsMsg::Updated(outcome.is_success()));
        outcome
    })
}

fn describe_inputs(names: &[String]) -> String {
    if names.is_empty() {
        "all inputs".to_owned()
    } else {
        names.join(", ")
    }
}

fn read_flake_lock(flake: &str) -> Result<Vec<LockedInput>, String> {
    let dir = drakeflake_flake::local_flake_dir(flake)
        .ok_or("Flake inputs can only be managed for a local flake directory.")?;

    drakeflake_flake::lock::read_lock(&dir).map_err(|error| error.to_string())
}

fn lock_writable(dir: &Path) -> bool {
    let lock = dir.join("flake.lock");

    if lock.exists() {
        std::fs::OpenOptions::new().append(true).open(&lock).is_ok()
    } else {
        can_create_in(dir)
    }
}

fn can_create_in(dir: &Path) -> bool {
    let probe = dir.join(".drakeflake-write-test");
    let created = std::fs::File::create(&probe).is_ok();

    if created && let Err(err) = std::fs::remove_file(&probe) {
        log::warn!("cannot remove the write probe {}: {err}", probe.display());
    }

    created
}

#[cfg(test)]
mod tests;
