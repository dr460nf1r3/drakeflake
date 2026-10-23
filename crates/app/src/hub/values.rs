use std::collections::HashMap;
use std::sync::Arc;

use drakeflake_exec::Context;
use drakeflake_flake::{Nix, Target, TypeCheck};
use drakeflake_plan::InputMode;
use drakeflake_render::{OptionName, PackageAttr};
use serde_json::Value;

use super::ids::{Generation, HomeEpoch};
use super::{ChangesMsg, CheckRequest, Hub, HubError, OptionsMsg, PackagesMsg};
use crate::hub::evaluation::WORKER_IDLE;

pub fn request_check(owner: &Arc<Hub>, request: CheckRequest) {
    let generation = owner.session().generation();
    let Some(receiver) = owner
        .evaluation()
        .checks()
        .send_or_start((generation, request))
    else {
        return;
    };

    owner.spawn_persistent("checks", move |hub, job_nix| {
        loop {
            let Ok(first) = receiver.recv_timeout(WORKER_IDLE) else {
                return;
            };

            let scoped = job_nix.with_cancel(hub.evaluation().scoped_token());
            let nix = &scoped;
            let pending = std::iter::once(first).chain(receiver.try_iter());

            for (queued_generation, queued) in newest_per_option(pending) {
                if !hub.is_current(queued_generation) {
                    continue;
                }

                if let Some(error) = expression_syntax_error(nix.context(), &queued.input) {
                    hub.mail.changes.post(ChangesMsg::Checked {
                        name: queued.name,
                        input: queued.input,
                        result: Ok(TypeCheck::Failed(error)),
                    });
                    continue;
                }

                let Some(target) = hub.target() else {
                    hub.mail.changes.post(ChangesMsg::Checked {
                        name: queued.name,
                        input: queued.input,
                        result: Err(HubError::NoTarget.to_string()),
                    });
                    continue;
                };

                log::debug!("type-checking {}", queued.name);
                let result =
                    drakeflake_flake::check_value(nix, &target, &queued.name, &queued.nix_value);

                if hub.is_current(queued_generation) {
                    hub.mail.changes.post(ChangesMsg::Checked {
                        name: queued.name,
                        input: queued.input,
                        result: result.map_err(|error| error.to_string()),
                    });
                }
            }
        }
    });
}

pub fn request_package(owner: &Arc<Hub>, attr: PackageAttr) {
    let generation = owner.session().generation();
    let Some(receiver) = owner
        .evaluation()
        .packages()
        .send_or_start((generation, attr))
    else {
        return;
    };

    owner.spawn_persistent("package", move |hub, job_nix| {
        loop {
            let Ok(first) = receiver.recv_timeout(WORKER_IDLE) else {
                return;
            };

            let scoped = job_nix.with_cancel(hub.evaluation().scoped_token());
            let nix = &scoped;
            let pending = std::iter::once(first).chain(receiver.try_iter());

            for (queued_generation, queued_attr) in newest_per_package(pending) {
                if !hub.is_current(queued_generation) {
                    continue;
                }

                let Some(target) = hub.target() else {
                    return;
                };

                log::debug!("reading the metadata of package {queued_attr}");
                let result = drakeflake_flake::package_meta(nix, &target, &queued_attr);

                if hub.is_current(queued_generation) {
                    hub.mail.packages.post(PackagesMsg::Details {
                        attr: queued_attr,
                        result: result.map_err(|error| error.to_string()),
                    });
                }
            }
        }
    });
}

pub fn request_value(owner: &Arc<Hub>, name: OptionName) {
    let generation = owner.session().generation();
    let epoch = owner.session().home_epoch();
    let Some(receiver) = owner
        .evaluation()
        .values()
        .send_or_start((generation, epoch, name))
    else {
        return;
    };

    owner.spawn_persistent("values", move |hub, job_nix| {
        loop {
            let Ok(first) = receiver.recv_timeout(WORKER_IDLE) else {
                return;
            };

            let scoped = job_nix.with_cancel(hub.evaluation().scoped_token());
            let nix = &scoped;
            let mut pending = vec![first];
            pending.extend(receiver.try_iter());
            let mut epochs: HashMap<OptionName, HomeEpoch> = HashMap::new();
            let mut names: Vec<OptionName> = Vec::new();
            let mut current = None;

            for (queued_generation, queued_epoch, queued_name) in pending {
                if !hub.is_current(queued_generation) || epochs.contains_key(&queued_name) {
                    continue;
                }

                current.get_or_insert(queued_generation);
                epochs.insert(queued_name.clone(), queued_epoch);
                names.push(queued_name);
            }

            let (Some(current), Some(target)) = (current, hub.target()) else {
                continue;
            };

            for (evaluated, result) in evaluate_values(nix, &target, names) {
                if hub.is_current(current) {
                    hub.mail.post_home(OptionsMsg::HomeValue {
                        epoch: epochs.get(&evaluated).copied().unwrap_or_default(),
                        name: evaluated.clone(),
                        result: result.clone(),
                        generation: current,
                    });
                    hub.mail
                        .post_options(OptionsMsg::Value(evaluated, result, current));
                }
            }
        }
    });
}

fn expression_syntax_error(exec: &Context, key: &str) -> Option<String> {
    let prefix = format!("{}:", InputMode::Expression);
    let expression = key.strip_prefix(&prefix)?;

    drakeflake_file::check_expression(exec, expression)
        .err()
        .map(|error| error.to_string())
}

fn newest_per_option(
    pending: impl IntoIterator<Item = (Generation, CheckRequest)>,
) -> Vec<(Generation, CheckRequest)> {
    let mut latest: Vec<(Generation, CheckRequest)> = Vec::new();
    for item in pending {
        latest.retain(|(_, queued)| queued.name != item.1.name);
        latest.push(item);
    }

    latest
}

fn newest_per_package(
    pending: impl IntoIterator<Item = (Generation, PackageAttr)>,
) -> Vec<(Generation, PackageAttr)> {
    let mut latest: Vec<(Generation, PackageAttr)> = Vec::new();
    for item in pending {
        latest.retain(|(_, queued)| queued != &item.1);
        latest.push(item);
    }

    latest
}

fn evaluate_values(
    nix: &Nix,
    target: &Target,
    names: Vec<OptionName>,
) -> Vec<(OptionName, Result<Value, String>)> {
    if names.len() > 1 {
        log::debug!("evaluating the values of {}", names.join(", "));

        match drakeflake_flake::eval_values(nix, target, &names) {
            Ok(mut values) => {
                return names
                    .into_iter()
                    .map(|name| {
                        let value = values
                            .remove(name.as_str())
                            .ok_or_else(|| "the value was not evaluated".to_owned());

                        (name, value)
                    })
                    .collect();
            }
            Err(error) => log::debug!("evaluating values together failed: {error}"),
        }
    }

    names
        .into_iter()
        .map(|name| {
            log::debug!("evaluating the value of {name}");
            let result =
                drakeflake_flake::eval_value(nix, target, &name).map_err(|error| error.to_string());

            (name, result)
        })
        .collect()
}

#[cfg(test)]
mod tests;
