use std::path::Path;

use drakeflake_exec::{LONG_TIMEOUT, SHORT_TIMEOUT};
use serde_json::{Map, Value};

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::{INSTALLED, PACKAGE_META, PACKAGE_SET, PACKAGE_SETS, PACKAGES};
use crate::flake_target::Target;
use crate::nix::{Query, json_arg};
use crate::pool::with_eval_slot;
use crate::shard::{EvalJob, OnSkip, eval_merged};

pub fn package_index(
    nix: &Nix,
    target: &Target,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<String, FlakeError> {
    let installable = target.config_ref()?;
    let packages = eval_merged(
        nix,
        flat_job(
            "packages",
            &installable,
            target.local_dir().as_deref(),
            PACKAGES,
        ),
        skip,
        on_skip,
    )?;

    Ok(Value::Object(packages).to_string())
}

pub fn package_sets(
    nix: &Nix,
    target: &Target,
    skip: &mut Vec<String>,
) -> Result<Vec<String>, FlakeError> {
    let installable = target.config_ref()?;
    let what = "top-level attributes for package sets";
    let merged = eval_merged(
        nix,
        flat_job(
            what,
            &installable,
            target.local_dir().as_deref(),
            PACKAGE_SETS,
        ),
        skip,
        &|_, _| {},
    )?;

    let sets = set_names(merged);
    log::debug!("{} package sets", sets.len());
    Ok(sets)
}

fn set_names(merged: Map<String, Value>) -> Vec<String> {
    let mut sets: Vec<String> = merged
        .into_iter()
        .filter_map(|(name, is_set)| (is_set.as_bool() == Some(true)).then_some(name))
        .collect();

    sets.sort_unstable();
    sets
}

#[must_use = "only checks the name"]
pub fn is_package_set_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

pub fn package_set_index(
    nix: &Nix,
    target: &Target,
    set: &str,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<String, FlakeError> {
    if !is_package_set_name(set) {
        return Err(FlakeError::NotPackageSet(set.to_owned()));
    }

    let installable = target.config_ref()?;
    let template = PACKAGE_SET.replace("__SET__", set);
    let what = format!("packages in {set}");
    let packages = eval_merged(
        nix,
        flat_job(
            &what,
            &installable,
            target.local_dir().as_deref(),
            &template,
        ),
        skip,
        on_skip,
    )?;

    Ok(Value::Object(packages).to_string())
}

const fn flat_job<'a>(
    what: &'a str,
    installable: &'a str,
    cwd: Option<&'a Path>,
    template: &'a str,
) -> EvalJob<'a> {
    EvalJob {
        what,
        installable,
        cwd,
        template,
        degrade: false,
        walker: false,
        split: 1,
    }
}

pub fn package_meta(
    nix: &Nix,
    target: &Target,
    attr: &drakeflake_render::PackageAttr,
) -> Result<Value, FlakeError> {
    let apply = format!("({PACKAGE_META}) {}", json_arg(&attr)?);

    target.eval_json(
        nix,
        &Query::new("package metadata", SHORT_TIMEOUT).apply(&apply),
    )
}

pub fn installed_packages(
    nix: &Nix,
    target: &Target,
) -> Result<Vec<drakeflake_render::PackageAttr>, FlakeError> {
    let installed: Vec<drakeflake_render::PackageAttr> = with_eval_slot(nix, || {
        target.eval_json(
            nix,
            &Query::new("package list", LONG_TIMEOUT).apply(INSTALLED),
        )
    })??;

    log::debug!("{}: {} installed packages", target.host, installed.len());
    Ok(installed)
}

#[cfg(test)]
mod tests;
