use std::time::Instant;

use drakeflake_exec::SHORT_TIMEOUT;
use drakeflake_render::split_option_name;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::{HOME_MANAGER_USERS, OPTIONS_DOC, SELECTED, SNAPSHOT};
use crate::flake_target::Target;
use crate::nix::{Query, json_arg};
use crate::shard::{EvalJob, OnSkip, Shard, eval_merged, eval_with_retries, merge_shards};
use crate::tree::{OptionTree, prefix_keys};

/// A broken option restarts its shard's evaluation, so smaller shards make restarts cheaper.
const SNAPSHOT_SPLIT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WantedOption<'a> {
    #[serde(rename = "n")]
    name: &'a str,
    #[serde(rename = "l")]
    loc: Vec<String>,
}

impl<'a> WantedOption<'a> {
    pub const fn new(name: &'a str, loc: Vec<String>) -> Self {
        Self { name, loc }
    }
}

pub fn home_manager_users(
    nix: &Nix,
    target: &Target,
) -> Result<Option<Vec<drakeflake_render::UserName>>, FlakeError> {
    let users: Option<Vec<drakeflake_render::UserName>> = target.eval_json(
        nix,
        &Query::new("home-manager users", SHORT_TIMEOUT).apply(HOME_MANAGER_USERS),
    )?;

    match &users {
        Some(users) => log::debug!("{}: home manager users: {}", target.host, users.join(", ")),
        None => log::debug!(
            "{}: home manager is not used as a NixOS module",
            target.host
        ),
    }

    Ok(users)
}

pub fn eval_options_doc(
    nix: &Nix,
    target: &Target,
    tree: &OptionTree,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<String, FlakeError> {
    let installable = target.config_ref()?;
    let template = tree.specialise(OPTIONS_DOC);
    let docs = eval_merged(
        nix,
        EvalJob {
            what: "option docs",
            installable: &installable,
            cwd: target.local_dir().as_deref(),
            template: &template,
            degrade: true,
            walker: true,
            split: 1,
        },
        skip,
        on_skip,
    )?;

    Ok(prefix_keys(docs, &tree.prefix()))
}

pub fn snapshot_values(
    nix: &Nix,
    target: &Target,
    tree: &OptionTree,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<String, FlakeError> {
    let installable = target.config_ref()?;
    let template = tree.specialise(SNAPSHOT);
    let values = eval_merged(
        nix,
        EvalJob {
            what: "option values",
            installable: &installable,
            cwd: target.local_dir().as_deref(),
            template: &template,
            degrade: false,
            walker: true,
            // Home Manager trees are small: more shards only add overhead.
            split: if *tree == OptionTree::System {
                SNAPSHOT_SPLIT
            } else {
                1
            },
        },
        skip,
        on_skip,
    )?;

    Ok(prefix_keys(values, &tree.prefix()))
}

fn relative_wanted<'a>(
    tree: &OptionTree,
    names: &'a [drakeflake_render::OptionName],
) -> Vec<WantedOption<'a>> {
    let prefix = split_option_name(tree.prefix().trim_end_matches('.'));

    names
        .iter()
        .map(|name| {
            let loc = split_option_name(name);
            let relative = if prefix.iter().all(String::is_empty) {
                loc
            } else {
                loc.strip_prefix(prefix.as_slice())
                    .map_or_else(Vec::new, <[String]>::to_vec)
            };

            WantedOption::new(name, relative)
        })
        .collect()
}

pub fn selected_values(
    nix: &Nix,
    target: &Target,
    tree: &OptionTree,
    names: &[drakeflake_render::OptionName],
) -> Result<String, FlakeError> {
    let wanted = relative_wanted(tree, names);
    let template = tree
        .specialise(SELECTED)
        .replace("__NAMES__", &json_arg(&wanted)?);

    let started = Instant::now();
    let (values, skipped) = run_selected(nix, target, &template)?;

    log::info!(
        "evaluated {} of {} selected option values in {:.1}s ({skipped} skipped)",
        values.len(),
        names.len(),
        started.elapsed().as_secs_f32(),
    );
    Ok(prefix_keys(values, &tree.prefix()))
}

fn run_selected(
    nix: &Nix,
    target: &Target,
    template: &str,
) -> Result<(Map<String, Value>, usize), FlakeError> {
    let installable = target.config_ref()?;
    let mut skip = Vec::new();
    let output = eval_with_retries(
        nix,
        EvalJob {
            what: "selected option values",
            installable: &installable,
            cwd: target.local_dir().as_deref(),
            template,
            degrade: false,
            walker: false,
            split: 1,
        },
        &Shard::single(),
        &mut skip,
        &|entry| log::debug!("selected values: skipping {entry}"),
    )?;

    Ok((merge_shards(&[output], false)?, skip.len()))
}

#[cfg(test)]
mod tests;
