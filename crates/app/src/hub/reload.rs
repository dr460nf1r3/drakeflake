//! The generated files only assign values, so declarations and the package set stay.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use drakeflake_evaluate::cache::{FINGERPRINT_FILE, VALUES_FILE};
use drakeflake_file::PACKAGES_PATH;
use drakeflake_flake::{Nix, OptionTree, Target};
use drakeflake_plan::ManagedState;
use drakeflake_render::{OptionName, UserName};
use drakeflake_store::ValueSnapshot;
use serde_json::{Map, Value};

use super::home::{HomeData, HomeFiles};
use super::load::Loader;
use super::session::Session;
use super::{Cache, Hub, OptionsMsg, steps, target};

const HOME_PREFIX: &str = "home-manager.users.";
const HOME_USERS: &str = "home-manager.users";

impl Loader<'_> {
    fn reload_everything(&self, cache: &Cache) -> bool {
        steps::run(
            self.nix,
            vec![
                steps::Step::new("installed", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_installed(cache)
                }),
                steps::Step::new("values", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_snapshot(cache)
                }),
                steps::Step::new("home-manager", steps::Urgency::Normal, |step| {
                    self.with_nix(step).load_home(HomeData::Evaluate)
                }),
            ],
        )
    }

    fn reload_changed(&self, cache: &Cache, names: BTreeSet<OptionName>) -> bool {
        let (system, homes) = split_names(names);
        log::info!(
            "re-evaluating {} changed options ({} of Home Manager users)",
            system.len(),
            homes.values().map(Vec::len).sum::<usize>()
        );
        log::debug!("changed options: {system:?} {homes:?}");

        steps::run(
            self.nix,
            vec![
                steps::Step::new("installed", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_installed(cache)
                }),
                steps::Step::new("values", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_selected(cache, &system)
                }),
                steps::Step::new("home-manager", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_home_selected(cache, &homes)
                }),
            ],
        )
    }

    fn refresh_selected(&self, cache: &Cache, names: &[OptionName]) -> bool {
        if names.is_empty() {
            return true;
        }

        let result = {
            let _activity = self.console.activity(
                "values",
                format!("Updating {} changed option values…", names.len()),
            );

            drakeflake_flake::selected_values(self.nix, self.target, &OptionTree::System, names)
        };

        if !self.is_current() {
            return false;
        }

        let fresh = match result {
            Ok(fresh) => fresh,
            Err(error) if error.is_cancelled() => return false,
            Err(error) => {
                self.console.warn(format!(
                    "Could not update the changed option values ({error}); evaluating all of them"
                ));
                return self.refresh_snapshot(cache);
            }
        };

        match merge_into(cache, VALUES_FILE, &fresh, names) {
            Ok(snapshot) => {
                self.mail
                    .post_options(OptionsMsg::Snapshot(Arc::new(snapshot)));
                true
            }
            Err(error) => {
                self.console.warn(format!(
                    "Could not update the changed option values ({error}); evaluating all of them"
                ));
                self.refresh_snapshot(cache)
            }
        }
    }

    fn refresh_home_selected(
        &self,
        cache: &Cache,
        homes: &BTreeMap<UserName, Vec<OptionName>>,
    ) -> bool {
        let selected = self.session.home_user();
        let mut complete = true;

        for (user, names) in homes {
            let files = HomeFiles::of(user);

            if selected.as_ref() != Some(user) {
                log::debug!("dropping the cached Home Manager values of {user}");
                cache.remove(&files.values);
                continue;
            }

            let tree = OptionTree::HomeManager(user.clone());
            let result = drakeflake_flake::selected_values(self.nix, self.target, &tree, names);

            if !self.is_current() {
                return false;
            }

            let fresh = match result {
                Ok(fresh) => fresh,
                Err(error) if error.is_cancelled() => return false,
                Err(error) => {
                    self.console.warn(format!(
                        "Could not update the changed Home Manager values of {user} ({error}); evaluating all of them"
                    ));
                    complete &= self.refresh_home(user, HomeData::Evaluate);
                    continue;
                }
            };

            match merge_into(cache, &files.values, &fresh, names) {
                Ok(snapshot) => self
                    .mail
                    .post_home(OptionsMsg::Snapshot(Arc::new(snapshot))),
                Err(error) => {
                    self.console.warn(format!(
                        "Could not update the changed Home Manager values of {user} ({error}); evaluating all of them"
                    ));
                    complete &= self.refresh_home(user, HomeData::Evaluate);
                }
            }
        }

        complete
    }
}

pub(super) fn state_before_write(
    session: &Session,
    nix: &Nix,
    path: &Path,
) -> Option<ManagedState> {
    let target = session.target()?;
    let fresh = flake_matches_cache(session, nix, &target);
    log::debug!("cache current before writing {}: {fresh}", path.display());

    fresh
        .then(|| {
            ManagedState::load(path)
                .inspect_err(|error| log::debug!("previous settings unknown: {error}"))
                .ok()
        })
        .flatten()
}

pub(super) fn flake_matches_cache(session: &Session, nix: &Nix, target: &Target) -> bool {
    let Some(fresh) = session.fresh() else {
        return false;
    };

    let cache = Cache::for_target(target);
    let cached = cache.read(FINGERPRINT_FILE).as_deref() == Some(fresh.as_str())
        && cache.read(VALUES_FILE).is_some();

    cached
        && drakeflake_flake::fingerprint(nix, target)
            .inspect_err(|error| log::warn!("cannot fingerprint the configuration: {error}"))
            .is_ok_and(|now| now == fresh)
}

pub(super) fn adopt_fingerprint(session: &Session, nix: &Nix, target: &Target) {
    let fingerprint = drakeflake_flake::fingerprint(nix, target)
        .inspect_err(|error| log::warn!("cannot fingerprint the configuration: {error}"))
        .ok();

    Cache::for_target(target).save_fingerprint(fingerprint.as_deref());
    session.mark_fresh(fingerprint.as_deref());
}

pub(super) fn reload_after_write(
    hub: &Arc<Hub>,
    nix: &Nix,
    previous: Option<ManagedState>,
    applied: ManagedState,
) {
    let state = hub.session().snapshot();
    let generation = state.generation;

    let Some(target) = state.target else {
        return;
    };

    target::check(hub.services(), nix, &target);

    let moved = hub.session().note_written_layout();

    hub.spawn_background("reload", move |hub, job_nix| {
        let loader = Loader::new(hub.services(), job_nix, generation, &target);

        let cache = Cache::for_target(&target);
        let started = Instant::now();
        let changed = previous.map(|previous| changed_options(&previous, &applied, moved));

        let complete = changed.map_or_else(
            || {
                log::info!("the cache was not current before the write; evaluating all values");
                loader.reload_everything(&cache)
            },
            |names| loader.reload_changed(&cache, names),
        );

        if complete {
            adopt_fingerprint(hub.session(), job_nix, &target);
        }

        log::info!(
            "values updated after the write in {:.1?} (complete {complete})",
            started.elapsed()
        );
    });
}

fn merge_into(
    cache: &Cache,
    name: &str,
    fresh: &str,
    requested: &[OptionName],
) -> Result<ValueSnapshot, String> {
    let cached = cache
        .read(name)
        .ok_or_else(|| format!("{name} is not cached"))?;

    let merged = merge_snapshot_text(&cached, fresh, requested)?;
    let snapshot = ValueSnapshot::from_json_str(&merged).map_err(|err| format!("{err:#}"))?;

    cache.write(name, &merged);
    Ok(snapshot)
}

fn merge_snapshot_text(
    cached: &str,
    fresh: &str,
    requested: &[OptionName],
) -> Result<String, String> {
    let parse = |text: &str, what: &str| {
        serde_json::from_str::<Map<String, Value>>(text)
            .map_err(|err| format!("the {what} values are not a JSON object: {err}"))
    };

    let mut merged = parse(cached, "cached")?;
    let fresh = parse(fresh, "updated")?;
    for name in requested {
        if !fresh.contains_key(name.as_str()) {
            merged.remove(name.as_str());
        }
    }

    merged.extend(fresh);

    Ok(Value::Object(merged).to_string())
}

fn changed_options(
    previous: &ManagedState,
    applied: &ManagedState,
    moved: bool,
) -> BTreeSet<OptionName> {
    let differs =
        |name: &OptionName| moved || previous.options.get(name) != applied.options.get(name);

    let mut names: BTreeSet<OptionName> = previous
        .options
        .keys()
        .chain(applied.options.keys())
        .filter(|name| differs(name))
        .cloned()
        .collect();

    let packages = !previous.packages.is_empty() || !applied.packages.is_empty();
    if previous.packages != applied.packages || (moved && packages) {
        names.insert(PACKAGES_PATH.into());
    }

    names
}

fn split_names(
    names: BTreeSet<OptionName>,
) -> (Vec<OptionName>, BTreeMap<UserName, Vec<OptionName>>) {
    let mut system = Vec::new();
    let mut homes: BTreeMap<UserName, Vec<OptionName>> = BTreeMap::new();

    for name in names {
        let user = name
            .starts_with(HOME_PREFIX)
            .then(|| {
                drakeflake_render::split_option_name(name.as_str())
                    .into_iter()
                    .nth(2)
            })
            .flatten();

        if let Some(user) = user {
            homes.entry(UserName::from(user)).or_default().push(name);
        } else {
            system.push(name);
        }
    }

    if !homes.is_empty() && !system.iter().any(|name| name == HOME_USERS) {
        system.push(HOME_USERS.into());
    }

    (system, homes)
}

#[cfg(test)]
mod tests;
