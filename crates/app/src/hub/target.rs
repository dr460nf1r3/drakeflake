use std::sync::Arc;

use drakeflake_file::GitState;
use drakeflake_flake::{Nix, Target};
use drakeflake_plan::ManagedState;

use super::console::Console;
use super::host_list::hosts_of;
use super::ids::Generation;
use super::load::CacheUse;
use super::services::Services;
use super::{ChangesMsg, Hub, SystemMsg, TargetInfo, Warning, WarningId};

#[derive(Clone, Copy)]
pub struct Requested<'a> {
    pub flake: &'a str,
    pub preferred_host: Option<&'a str>,
    pub cache_use: CacheUse,
}

pub(super) fn resolve(
    services: Services<'_>,
    nix: &Nix,
    generation: Generation,
    asked: Requested<'_>,
) -> Option<Target> {
    let Requested {
        flake: requested,
        preferred_host: preferred,
        cache_use,
    } = asked;

    let flake = drakeflake_flake::normalize_flake(requested);

    if flake != requested.trim() {
        services.console.warn(format!(
            "{requested} contains no flake.nix; using the enclosing flake {flake}"
        ));
    }

    if flake.starts_with('/') && !std::path::Path::new(&flake).exists() {
        services.mail.system.post(SystemMsg::TargetError {
            flake: flake.clone(),
            hosts: Vec::new(),
            error: format!("The flake folder {flake} does not exist."),
        });
        return None;
    }

    let hosts = hosts_of(services, nix, &flake, cache_use);

    if !services.session.is_current(generation) {
        log::debug!("load generation {generation} superseded while listing hosts");
        return None;
    }

    let host = choose_host(services.console, &hosts, preferred);
    let hosts = hosts.unwrap_or_default();

    let host = match host {
        Ok(host) => host,
        Err(error) => {
            log::error!("no usable host in {flake}: {error}");
            services.mail.system.post(SystemMsg::TargetError {
                flake,
                hosts,
                error,
            });
            return None;
        }
    };

    let target = Target {
        flake,
        host: host.name.into(),
        kind: host.kind,
    };

    if !services
        .session
        .store_target_if_current(generation, &target)
    {
        log::debug!("load generation {generation} superseded before the target was set");
        return None;
    }

    announce(services, nix, &target, hosts);
    Some(target)
}

pub(super) fn set_managed(
    hub: &Arc<Hub>,
    managed_setting: Option<String>,
    hosts: Vec<drakeflake_flake::Host>,
) {
    hub.session().set_managed_setting(managed_setting);

    hub.spawn_background("managed", move |hub, nix| {
        if let Some(target) = hub.session().target() {
            announce(hub.services(), nix, &target, hosts);
        } else {
            log::warn!("output path changed while no system is loaded");
        }
    });
}

fn announce(
    services: Services<'_>,
    nix: &Nix,
    target: &Target,
    hosts: Vec<drakeflake_flake::Host>,
) {
    let managed = services.session.managed_file();
    match &managed {
        Ok(path) => log::info!(
            "target {}#{}, managed file {}",
            target.flake,
            target.host,
            path.display()
        ),
        Err(error) => log::warn!(
            "target {}#{} has no managed file: {error}",
            target.flake,
            target.host
        ),
    }

    services.mail.system.post(SystemMsg::Target(TargetInfo {
        flake: target.flake.clone(),
        host: target.host.to_string(),
        kind: target.kind,
        hosts,
        managed_path: managed.as_ref().ok().cloned(),
    }));

    services.mail.changes.post(ChangesMsg::Managed(
        managed
            .map_err(|error| error.to_string())
            .and_then(|path| ManagedState::load(&path).map_err(|error| error.to_string())),
    ));

    check(services, nix, target);
}

fn choose_host(
    console: &Console,
    hosts: &Result<Vec<drakeflake_flake::Host>, drakeflake_flake::FlakeError>,
    preferred: Option<&str>,
) -> Result<drakeflake_flake::Host, String> {
    match hosts {
        Ok(list) => {
            let local = drakeflake_flake::local_hostname();
            log::debug!(
                "hosts {list:?}, preferred {preferred:?}, local hostname {:?}",
                local.as_deref()
            );
            drakeflake_flake::choose_host(list, preferred, local.as_deref())
                .map_err(|error| error.to_string())
        }
        Err(error) => match preferred {
            Some(name) if drakeflake_flake::validate_host(name).is_ok() => {
                console.warn(format!("Could not list hosts ({error}); trying \"{name}\""));
                Ok(drakeflake_flake::Host {
                    name: name.to_owned(),
                    kind: drakeflake_flake::SystemKind::native(),
                })
            }
            _ => Err(error.to_string()),
        },
    }
}

pub(super) fn check(services: Services<'_>, nix: &Nix, target: &Target) {
    let mut warnings = Vec::new();
    let managed = services.session.managed_file();

    match (target.local_dir(), &managed) {
        (_, Err(error)) => warnings.push(Warning {
            id: WarningId::Output,
            text: format!("{error}. Changes cannot be saved."),
        }),
        (None, Ok(_)) => warnings.push(Warning {
            id: WarningId::Remote,
            text: "This flake is not a local directory, so it cannot import the generated file. Point Settings at a local checkout.".to_owned(),
        }),
        (Some(dir), Ok(_)) if !dir.is_dir() => warnings.push(Warning {
            id: WarningId::Missing,
            text: format!("{} does not exist.", dir.display()),
        }),
        (Some(dir), Ok(file)) => {
            let name = file.file_name().map_or_else(
                || file.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );

            if drakeflake_file::is_inside(&dir, file) {
                if !drakeflake_file::is_imported(&dir, file) {
                    warnings.push(Warning {
                        id: WarningId::NotImported,
                        text: format!(
                            "Your configuration does not import {name} yet. Add it (e.g. ./{}) to the imports of host \"{}\", otherwise applied changes have no effect.",
                            file.strip_prefix(&dir).unwrap_or(file).display(),
                            target.host
                        ),
                    });
                }
            } else {
                warnings.push(Warning {
                    id: WarningId::Outside,
                    text: format!(
                        "{} is outside the flake ({}). Flakes can only import files inside their own directory, so the generated settings will not be used unless you evaluate impurely.",
                        file.display(),
                        dir.display()
                    ),
                });
            }

            if file.exists() && drakeflake_file::git_state(nix.context(), file) == GitState::Untracked {
                warnings.push(Warning {
                    id: WarningId::Untracked,
                    text: format!("{name} is not tracked by git, so the flake cannot see it."),
                });
            }
        }
    }

    services.mail.system.post(SystemMsg::Warnings(warnings));
}

#[cfg(test)]
mod tests;
