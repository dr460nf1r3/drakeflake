use std::path::Path;
use std::sync::Arc;

use drakeflake_exec::Context;
use drakeflake_validate::HitKind;
use serde::Serialize;

use super::{Hub, SearchMsg};

const MAX_HITS: usize = 200;

const MAX_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FlakeHitKind {
    Attribute,
    Value,
}

impl From<HitKind> for FlakeHitKind {
    fn from(kind: HitKind) -> Self {
        match kind {
            HitKind::Attribute => Self::Attribute,
            HitKind::Value => Self::Value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlakeHit {
    pub file: String,
    pub display: String,
    pub line: usize,
    pub kind: FlakeHitKind,
    pub path: String,
    pub snippet: String,
}

pub fn search_flake(owner: &Arc<Hub>, query: String) {
    let generation = owner.evaluation().next_search();
    let Some(dir) = owner.target().and_then(|target| target.local_dir()) else {
        log::debug!("source search skipped: no local flake");
        owner.mail.search.post(SearchMsg::Flake {
            query,
            total: 0,
            hits: Vec::new(),
        });
        return;
    };

    owner.spawn_background("flake-search", move |hub, nix| {
        let current = || hub.evaluation().search_is_current(generation);
        let Some((total, hits)) = search_dir(nix.context(), &dir, &query, current) else {
            log::debug!("source search for {query:?} replaced by a newer one");
            return;
        };

        log::debug!("source search for {query:?}: {total} hits");
        hub.mail
            .search
            .post(SearchMsg::Flake { query, total, hits });
    });
}

pub fn search_dir(
    exec: &Context,
    dir: &Path,
    query: &str,
    current: impl Fn() -> bool,
) -> Option<(usize, Vec<FlakeHit>)> {
    let mut hits = Vec::new();

    for file in drakeflake_file::nix_files(exec, dir) {
        if !current() {
            return None;
        }

        let Some(text) = read_source(&file) else {
            continue;
        };

        let display = file
            .strip_prefix(dir)
            .unwrap_or(&file)
            .to_string_lossy()
            .into_owned();

        hits.extend(
            drakeflake_validate::search_source(&text, query)
                .into_iter()
                .map(|hit| FlakeHit {
                    file: file.to_string_lossy().into_owned(),
                    display: display.clone(),
                    line: hit.line,
                    kind: hit.kind.into(),
                    path: hit.path,
                    snippet: hit.snippet,
                }),
        );
    }

    hits.sort_by_key(|hit| hit.kind);

    let total = hits.len();
    hits.truncate(MAX_HITS);
    Some((total, hits))
}

fn read_source(file: &Path) -> Option<String> {
    let size = std::fs::metadata(file).map_or(0, |meta| meta.len());

    if size > MAX_FILE_BYTES {
        log::debug!("source search skips {} ({size} bytes)", file.display());
        return None;
    }

    std::fs::read_to_string(file)
        .map_err(|error| log::debug!("source search cannot read {}: {error}", file.display()))
        .ok()
}

#[cfg(test)]
mod tests;
