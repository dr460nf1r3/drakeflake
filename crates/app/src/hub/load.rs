use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use drakeflake_evaluate::cache::{
    INSTALLED_FILE, OPTIONS_FILE, OPTIONS_KEY_FILE, PACKAGES_FILE, VALUES_FILE,
};
pub use drakeflake_evaluate::{EvaluateError, parse_json};
use drakeflake_exec::CancelToken;
use drakeflake_flake::{Nix, Target, same_flake};
use drakeflake_render::PackageAttr;
use drakeflake_store::{OptionIndex, PackageIndex, ValueSnapshot};

use super::console::Console;
use super::home::HomeData;
use super::ids::Generation;
use super::services::Services;
use super::session::{LoadStart, Session};
use super::target::{self, Requested};
use super::{Cache, ChangesMsg, Hub, Mailboxes, OptionsMsg, PackagesMsg, steps};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheUse {
    Reuse,
    Rebuild,
}

impl CacheUse {
    #[must_use = "only picks the variant"]
    pub const fn reuse_or_rebuild(rebuild: bool) -> Self {
        if rebuild { Self::Rebuild } else { Self::Reuse }
    }
}

#[derive(Clone, Copy)]
pub struct Loader<'a> {
    pub session: &'a Session,
    pub console: &'a Console,
    pub mail: &'a Mailboxes,
    pub nix: &'a Nix,
    pub generation: Generation,
    pub target: &'a Target,
}

#[derive(Clone, Copy)]
pub struct IndexKey<'a> {
    pub key: Option<&'a str>,
    pub reusable: bool,
}

impl<'a> Loader<'a> {
    pub const fn new(
        services: Services<'a>,
        nix: &'a Nix,
        generation: Generation,
        target: &'a Target,
    ) -> Self {
        Self {
            session: services.session,
            console: services.console,
            mail: services.mail,
            nix,
            generation,
            target,
        }
    }

    pub const fn with_nix(self, nix: &'a Nix) -> Self {
        Self { nix, ..self }
    }

    pub fn is_current(&self) -> bool {
        self.session.is_current(self.generation)
    }

    fn load_data(self, cache: &Cache, cache_use: CacheUse) {
        let started = Instant::now();

        let index_shown = match cache_use {
            CacheUse::Rebuild => {
                cache.clear();
                false
            }
            CacheUse::Reuse => self.load_cached(cache),
        };

        let (fingerprint, index_key) = self.identify();
        let fingerprint = fingerprint.as_deref();

        if cache_use == CacheUse::Reuse && fingerprint.is_some() && cache.is_current(fingerprint) {
            self.load_unchanged(cache, fingerprint);
            log::info!("load finished from the cache in {:.1?}", started.elapsed());
            return;
        }

        let index = IndexKey {
            key: index_key.as_deref(),
            reusable: index_shown
                && index_key.is_some()
                && cache.read(OPTIONS_KEY_FILE).as_deref() == index_key.as_deref(),
        };

        let complete = steps::run(
            self.nix,
            vec![
                steps::Step::new("options", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_options_index(cache, index)
                }),
                steps::Step::new("installed", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_installed(cache)
                }),
                steps::Step::new("values", steps::Urgency::Urgent, |step| {
                    self.with_nix(step).refresh_snapshot(cache)
                }),
                steps::Step::new("packages", steps::Urgency::Normal, |step| {
                    self.with_nix(step).refresh_packages(cache)
                }),
                steps::Step::new("home-manager", steps::Urgency::Normal, |step| {
                    self.with_nix(step).load_home(HomeData::Evaluate)
                }),
                steps::Step::new("package-sets", steps::Urgency::Normal, |step| {
                    self.with_nix(step).load_package_sets(cache, fingerprint);
                    true
                }),
            ],
        );

        if complete {
            cache.save_fingerprint(fingerprint);
            self.session.mark_fresh(fingerprint);
        } else {
            log::debug!("evaluation incomplete, the configuration fingerprint is not saved");
        }

        log::info!(
            "load finished in {:.1?} (complete {complete})",
            started.elapsed()
        );
    }

    fn identify(&self) -> (Option<String>, Option<String>) {
        let activity = self
            .console
            .activity("target", "Checking for configuration changes…");

        let fingerprint = drakeflake_flake::fingerprint(self.nix, self.target)
            .inspect_err(|error| log::warn!("cannot fingerprint the configuration: {error}"))
            .ok();

        let index_key = drakeflake_flake::option_index_key(
            self.nix.context(),
            self.nix,
            self.target,
            &self.generated_paths(),
        );

        drop(activity);

        log::debug!("configuration fingerprint {fingerprint:?}, option index key {index_key:?}");
        self.session.set_index_key(index_key.as_deref());
        (fingerprint, index_key)
    }

    fn load_unchanged(self, cache: &Cache, fingerprint: Option<&str>) {
        self.console
            .log("The configuration is unchanged since the last evaluation; using cached data.");
        self.session.mark_fresh(fingerprint);
        steps::run(
            self.nix,
            vec![
                steps::Step::new("home-manager", steps::Urgency::Normal, |step| {
                    self.with_nix(step).load_home(HomeData::Cached)
                }),
                steps::Step::new("package-sets", steps::Urgency::Normal, |step| {
                    self.with_nix(step).load_package_sets(cache, fingerprint);
                    true
                }),
            ],
        );
    }

    fn load_cached(&self, cache: &Cache) -> bool {
        let installed = cache.read_list(INSTALLED_FILE);

        if !installed.is_empty() {
            self.mail.packages.post(PackagesMsg::Installed(
                installed.into_iter().map(PackageAttr::from).collect(),
            ));
        }

        let options = cache.read_parsed(OPTIONS_FILE, OptionIndex::from_json_str);
        let index_shown = options.is_some();

        if let Some(options) = options {
            self.mail.post_options(OptionsMsg::Index(Arc::new(options)));
        }

        if let Some(snapshot) = cache.read_parsed(VALUES_FILE, ValueSnapshot::from_json_str) {
            self.mail
                .post_options(OptionsMsg::Snapshot(Arc::new(snapshot)));
        }

        if let Some(index) = cache.read_parsed(PACKAGES_FILE, PackageIndex::from_json_str) {
            self.mail.packages.post(PackagesMsg::Index(Arc::new(index)));
        }

        index_shown
    }

    fn generated_paths(&self) -> Vec<PathBuf> {
        self.session
            .target()
            .map(|target| self.session.generated_paths_of(&target.flake))
            .unwrap_or_default()
    }
}

pub fn start(
    hub: &Arc<Hub>,
    flake: String,
    preferred_host: Option<String>,
    managed_setting: Option<String>,
    cache_use: CacheUse,
) {
    let LoadStart {
        generation,
        same_system,
    } = hub.session().begin_load(managed_setting, |current| {
        is_same_system(current, &flake, preferred_host.as_deref())
    });

    log::info!(
        "loading flake {flake} (host {}, cache {cache_use:?})",
        preferred_host.as_deref().unwrap_or("auto")
    );
    log::debug!("load generation {generation}, same system {same_system}");

    hub.console().clear_activities();
    hub.mail().post_options(OptionsMsg::Reset);
    hub.mail().post_home(OptionsMsg::Reset);
    hub.mail().packages.post(PackagesMsg::Reset);
    hub.mail().changes.post(ChangesMsg::Reset {
        keep_staged: same_system,
    });

    let token = CancelToken::new();
    hub.evaluation().replace_load(token.clone());

    hub.spawn_background("loader", move |hub, nix| {
        let nix = &nix.with_cancel(token);
        let services = hub.services();

        if let Some(target) = target::resolve(
            services,
            nix,
            generation,
            Requested {
                flake: &flake,
                preferred_host: preferred_host.as_deref(),
                cache_use,
            },
        ) {
            let cache = Cache::for_target(&target);
            Loader::new(services, nix, generation, &target).load_data(&cache, cache_use);
        }
    });
}

pub fn restart(hub: &Arc<Hub>) {
    let state = hub.session().snapshot();

    let Some(target) = state.target else {
        log::debug!("no load to restart: no system is loaded");
        return;
    };

    log::info!("restarting the load of {}#{}", target.flake, target.host);
    start(
        hub,
        target.flake,
        Some(target.host.to_string()),
        state.managed_setting,
        CacheUse::Reuse,
    );
}

fn is_same_system(current: Option<&Target>, flake: &str, preferred_host: Option<&str>) -> bool {
    current.is_some_and(|target| {
        same_flake(&target.flake, flake) && preferred_host.is_none_or(|host| host == target.host)
    })
}

pub fn log_step_failure(what: &str, error: &EvaluateError) {
    if error.is_cancelled() {
        log::debug!("{what} failed: {error} (replaced by a newer load)");
    } else {
        log::error!("{what} failed: {error}");
    }
}

#[cfg(test)]
mod tests;
