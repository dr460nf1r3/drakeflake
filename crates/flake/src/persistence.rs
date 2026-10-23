mod place;

use std::collections::BTreeMap;

use drakeflake_exec::SHORT_TIMEOUT;
pub use place::{PersistPlace, flake_relative};
use serde::{Deserialize, Serialize};

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::PERSISTENCE;
use crate::flake_target::Target;
use crate::nix::Query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersistKind {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersistScope {
    System,
    User,
    Home,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistEntry {
    pub kind: PersistKind,
    pub path: String,
    pub entry: String,
    pub storage: String,
    pub scope: PersistScope,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistSource {
    pub scope: PersistScope,
    pub user: String,
    pub storage: String,
    pub entry: String,
    pub file: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Persistence {
    pub entries: Vec<PersistEntry>,
    pub storages: Vec<String>,
    pub homes: BTreeMap<String, String>,
    pub home_module: BTreeMap<String, bool>,
    pub sources: Vec<PersistSource>,
}

pub fn persistence(nix: &Nix, target: &Target) -> Result<Option<Persistence>, FlakeError> {
    let found: Option<Persistence> = target.eval_json(
        nix,
        &Query::new("persistence", SHORT_TIMEOUT).apply(PERSISTENCE),
    )?;

    match &found {
        Some(found) => log::debug!(
            "{}: {} persisted entries in {:?}",
            target.host,
            found.entries.len(),
            found.storages
        ),
        None => log::debug!("{}: no impermanence module", target.host),
    }

    Ok(found)
}
