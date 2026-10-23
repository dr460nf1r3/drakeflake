use std::sync::Arc;

use drakeflake_evaluate::EvaluateError;
use drakeflake_evaluate::cache::PACKAGES_SKIP_FILE;
use drakeflake_fs::fnv1a;
use drakeflake_store::PackageIndex;

use super::load::{Loader, log_step_failure};
use super::{Cache, Hub, PackagesMsg};

impl Loader<'_> {
    pub(super) fn load_package_sets(&self, cache: &Cache, fingerprint: Option<&str>) {
        let (nix, target) = (self.nix, self.target);

        let version = fingerprint.map(|text| format!("{:016x}", fnv1a(text.as_bytes())));
        self.session.set_package_version(version.as_deref());

        if let Some(version) = &version {
            remove_stale_sets(cache, version);
        }

        let file = version
            .as_ref()
            .map(|version| format!("package-sets-{version}.json"));

        let cached = file
            .as_ref()
            .and_then(|file| cache.read(file).map(|text| (file, text)))
            .and_then(|(file, text)| {
                serde_json::from_str::<Vec<String>>(&text)
                    .inspect_err(|err| log::warn!("ignoring the cached {file}: {err}"))
                    .ok()
            });

        let sets = if let Some(sets) = cached {
            log::debug!("package sets: {} cached", sets.len());
            sets
        } else {
            log::debug!("package sets: not cached, evaluating");
            match drakeflake_flake::package_sets(
                nix,
                target,
                &mut cache.read_list(PACKAGES_SKIP_FILE),
            ) {
                Ok(sets) => {
                    log::debug!("package sets: {} evaluated", sets.len());
                    if let Some(file) = &file {
                        match serde_json::to_string(&sets) {
                            Ok(text) => cache.write(file, &text),
                            Err(err) => log::warn!("cannot cache the package sets: {err}"),
                        }
                    }

                    sets
                }
                Err(error) => {
                    self.console
                        .warn(format!("Could not list package sets: {error}"));
                    return;
                }
            }
        };

        if self.is_current() {
            self.mail.packages.post(PackagesMsg::Sets(sets));
        }
    }

    fn index_set(&self, set: String) {
        let (nix, target) = (self.nix, self.target);

        let cache = Cache::for_target(target);
        let file = self
            .session
            .package_version()
            .map(|version| format!("set-{:016x}-{version}.json", fnv1a(set.as_bytes())));

        let text = file.as_ref().and_then(|file| cache.read(file)).map_or_else(
            || {
                log::debug!("package set {set}: not cached, indexing");
                let result = {
                    let _activity = self
                        .console
                        .activity("package-set", format!("Indexing {set}…"));

                    drakeflake_flake::package_set_index(
                        nix,
                        target,
                        &set,
                        &mut Vec::new(),
                        &|_, _| {},
                    )
                };

                if let (Ok(text), Some(file)) = (&result, &file) {
                    cache.write(file, text);
                }

                result
            },
            |text| {
                log::debug!("package set {set}: cached");
                Ok(text)
            },
        );

        if !self.is_current() {
            log::debug!("package set {set}: system changed meanwhile, dropped");
            return;
        }

        let result = text.map_err(EvaluateError::from).and_then(|text| {
            PackageIndex::from_json_str(&text)
                .map(Arc::new)
                .map_err(|err| EvaluateError::Parse(format!("{err:#}")))
        });

        if let Err(error) = &result {
            log_step_failure(&format!("package set {set}: indexing"), error);
        }

        self.mail.packages.post(PackagesMsg::SetIndex {
            set,
            result: result.map_err(|error| error.to_string()),
        });
    }
}

pub fn request_package_set(owner: &Arc<Hub>, set: String) {
    let generation = owner.session().generation();

    owner.spawn_background("package-set", move |hub, nix| {
        let Some(target) = hub.session().target() else {
            return;
        };

        Loader::new(hub.services(), nix, generation, &target).index_set(set);
    });
}

fn remove_stale_sets(cache: &Cache, version: &str) {
    let Some(Ok(entries)) = cache.dir().map(std::fs::read_dir) else {
        return;
    };

    let mut removed = 0_usize;

    for entry in entries.flatten() {
        if !is_stale_set_file(&entry.file_name().to_string_lossy(), version) {
            continue;
        }

        let path = entry.path();
        match std::fs::remove_file(&path) {
            Ok(()) => removed = removed.saturating_add(1),
            Err(err) => log::warn!("cannot remove the stale cache {}: {err}", path.display()),
        }
    }

    if removed > 0 {
        log::debug!("removed {removed} stale package set caches");
    }
}

fn is_stale_set_file(name: &str, version: &str) -> bool {
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };

    let file_version = stem.strip_prefix("package-sets-").or_else(|| {
        let rest = stem.strip_prefix("set-")?;
        rest.split_once('-').map(|(_, set_version)| set_version)
    });

    file_version.is_some_and(|file_version| file_version != version)
}

#[cfg(test)]
mod tests;
