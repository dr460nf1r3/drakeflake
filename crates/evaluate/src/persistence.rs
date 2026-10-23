use drakeflake_flake::{Nix, Persistence, Target};
use serde::{Deserialize, Serialize};

use crate::cache::{Cache, PERSISTENCE_FILE};
use crate::error::EvaluateError;

#[derive(Deserialize)]
struct Cached {
    fingerprint: String,
    persistence: Option<Persistence>,
}

#[derive(Serialize)]
struct CachedRef<'a> {
    fingerprint: &'a str,
    persistence: Option<&'a Persistence>,
}

pub fn persistence_current(
    nix: &Nix,
    target: &Target,
) -> Result<Option<Persistence>, EvaluateError> {
    let fingerprint = drakeflake_flake::fingerprint(nix, target)?;

    persistence_for(nix, target, &Cache::for_target(target), &fingerprint)
}

pub fn persistence_for(
    nix: &Nix,
    target: &Target,
    cache: &Cache,
    fingerprint: &str,
) -> Result<Option<Persistence>, EvaluateError> {
    if let Some(stored) = cached(cache, fingerprint) {
        log::debug!("persisted entries from the cache");
        return Ok(stored.persistence);
    }

    let found = drakeflake_flake::persistence(nix, target)?;
    let text = serde_json::to_string(&CachedRef {
        fingerprint,
        persistence: found.as_ref(),
    })
    .map_err(EvaluateError::Encode)?;

    cache.write(PERSISTENCE_FILE, &text);

    Ok(found)
}

fn cached(cache: &Cache, fingerprint: &str) -> Option<Cached> {
    let stored: Cached = cache.read_parsed(PERSISTENCE_FILE, |text| serde_json::from_str(text))?;

    (stored.fingerprint == fingerprint).then_some(stored)
}

#[cfg(test)]
mod tests;
