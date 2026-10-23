//! Listing hosts evaluates the flake outputs, which copies the flake into the store
//! and burns CPU on every start.

use drakeflake_evaluate::cache::{Cache, HOSTS_FILE};
use drakeflake_flake::{FlakeError, Host, Nix};
use serde::{Deserialize, Serialize};

use super::load::CacheUse;
use super::services::Services;

#[derive(Serialize, Deserialize)]
struct CachedHosts {
    key: String,
    hosts: Vec<Host>,
}

pub(super) fn hosts_of(
    services: Services<'_>,
    nix: &Nix,
    flake: &str,
    cache_use: CacheUse,
) -> Result<Vec<Host>, FlakeError> {
    let generated = services.session.generated_paths_of(flake);
    let key = drakeflake_flake::hosts_key(nix.context(), flake, &generated);
    let cache = Cache::for_flake(flake);

    if let Some(key) = key.as_deref()
        && cache_use == CacheUse::Reuse
        && let Some(cached) =
            cache.read_parsed(HOSTS_FILE, |text| serde_json::from_str::<CachedHosts>(text))
        && cached.key == key
    {
        log::debug!("{} hosts of {flake} from the cache", cached.hosts.len());
        return Ok(cached.hosts);
    }

    let hosts = {
        let _activity = services
            .console
            .activity("target", "Reading hosts from the flake…");

        drakeflake_flake::list_hosts(nix, flake)?
    };

    if let Some(key) = key {
        let cached = CachedHosts {
            key,
            hosts: hosts.clone(),
        };

        match serde_json::to_string(&cached) {
            Ok(text) => cache.write(HOSTS_FILE, &text),
            Err(error) => log::warn!("cannot cache the hosts of {flake}: {error}"),
        }
    }

    Ok(hosts)
}

#[cfg(test)]
mod tests;
