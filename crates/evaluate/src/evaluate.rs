use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use drakeflake_flake::{Nix, OnSkip, OptionTree, Target};
use drakeflake_store::{OptionIndex, PackageIndex, ValueSnapshot};

use crate::cache::{
    Cache, INSTALLED_FILE, OPTIONS_FILE, OPTIONS_KEY_FILE, OPTIONS_SKIP_FILE, PACKAGES_FILE,
    PACKAGES_SKIP_FILE, VALUES_FILE, VALUES_SKIP_FILE,
};
use crate::error::EvaluateError;

pub type Progress<'a> = &'a (dyn Fn(&str) + Sync);

#[must_use = "a dropped evaluation was run for nothing"]
pub struct Evaluated<T> {
    pub result: Result<(String, T), EvaluateError>,
    pub skipped: Vec<String>,
}

impl<T> Evaluated<T> {
    fn parsed<F, E>(
        (text, skipped): (Result<String, F>, Vec<String>),
        parse: impl FnOnce(&str) -> Result<T, E>,
    ) -> Self
    where
        F: Into<EvaluateError>,
        E: std::fmt::Display,
    {
        Self {
            result: parse_json(text, parse),
            skipped,
        }
    }
}

#[must_use = "the evaluation's result is the reason to run it"]
pub fn evaluate_skipping<T>(
    cache: &Cache,
    file: &str,
    evaluate: impl FnOnce(&mut Vec<String>, OnSkip<'_>) -> T,
    on_skip: OnSkip<'_>,
) -> (T, Vec<String>) {
    let mut skip = cache.read_list(file);
    let result = evaluate(&mut skip, on_skip);
    cache.write_list(file, &skip);
    (result, skip)
}

pub fn parse_json<T, F, E>(
    text: Result<String, F>,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<(String, T), EvaluateError>
where
    F: Into<EvaluateError>,
    E: std::fmt::Display,
{
    let text = text.map_err(Into::into)?;
    let parsed = parse(&text).map_err(|err| EvaluateError::Parse(format!("{err:#}")))?;
    Ok((text, parsed))
}

pub fn option_index(
    nix: &Nix,
    target: &Target,
    cache: &Cache,
    on_fallback: impl FnOnce(),
    on_skip: OnSkip<'_>,
) -> Evaluated<OptionIndex> {
    let evaluated = match drakeflake_flake::build_options_json(nix, target) {
        Ok(path) => (
            fs::read_to_string(&path).map_err(|source| EvaluateError::Read { path, source }),
            Vec::new(),
        ),
        Err(build_error) => {
            log::warn!("cannot build options.json, evaluating instead: {build_error}");
            on_fallback();

            let (text, skipped) = evaluate_skipping(
                cache,
                OPTIONS_SKIP_FILE,
                |skip, hook| {
                    drakeflake_flake::eval_options_doc(nix, target, &OptionTree::System, skip, hook)
                },
                on_skip,
            );

            (
                text.map_err(|eval| EvaluateError::Fallback {
                    build: Box::new(build_error),
                    eval: Box::new(eval),
                }),
                skipped,
            )
        }
    };

    Evaluated::parsed(evaluated, OptionIndex::from_json_str)
}

pub fn option_values(
    nix: &Nix,
    target: &Target,
    cache: &Cache,
    on_skip: OnSkip<'_>,
) -> Evaluated<ValueSnapshot> {
    let evaluated = evaluate_skipping(
        cache,
        VALUES_SKIP_FILE,
        |skip, hook| {
            drakeflake_flake::snapshot_values(nix, target, &OptionTree::System, skip, hook)
        },
        on_skip,
    );

    Evaluated::parsed(evaluated, ValueSnapshot::from_json_str)
}

pub fn package_index(
    nix: &Nix,
    target: &Target,
    cache: &Cache,
    on_skip: OnSkip<'_>,
) -> Evaluated<PackageIndex> {
    let evaluated = evaluate_skipping(
        cache,
        PACKAGES_SKIP_FILE,
        |skip, hook| drakeflake_flake::package_index(nix, target, skip, hook),
        on_skip,
    );

    Evaluated::parsed(evaluated, PackageIndex::from_json_str)
}

/// The fingerprint is saved only after a complete run, so a partial one is evaluated again.
#[must_use = "an incomplete run must not mark the cache fresh"]
pub fn refresh(
    nix: &Nix,
    target: &Target,
    fingerprint: &str,
    generated: &[PathBuf],
    progress: Progress<'_>,
) -> bool {
    let started = Instant::now();
    let cache = Cache::for_target(target);
    let index_key = drakeflake_flake::option_index_key(nix.context(), nix, target, generated);

    let mut complete = save_options(nix, target, &cache, index_key.as_deref(), progress);
    complete &= save_installed(nix, target, &cache);
    complete &= save_values(nix, target, &cache, progress);
    complete &= save_packages(nix, target, &cache, progress);
    complete &= crate::persistence::persistence_for(nix, target, &cache, fingerprint)
        .inspect_err(|error| log::warn!("persisted entries failed: {error}"))
        .is_ok();

    if complete {
        cache.save_fingerprint(Some(fingerprint));
    }

    log::info!(
        "evaluation of {} finished in {:.1?} (complete {complete})",
        target.host,
        started.elapsed()
    );
    complete
}

fn save<T>(cache: &Cache, file: &str, what: &str, evaluated: Evaluated<T>) -> bool {
    match evaluated.result {
        Ok((text, _)) => {
            log::debug!("{what} evaluated ({} skipped)", evaluated.skipped.len());
            cache.write(file, &text);
            true
        }
        Err(error) => {
            log::warn!("{what} failed: {error}");
            false
        }
    }
}

fn save_options(
    nix: &Nix,
    target: &Target,
    cache: &Cache,
    index_key: Option<&str>,
    progress: Progress<'_>,
) -> bool {
    if index_key.is_some()
        && cache.read(OPTIONS_KEY_FILE).as_deref() == index_key
        && cache.read(OPTIONS_FILE).is_some()
    {
        log::debug!("option declarations unchanged, keeping the cached option index");
        return true;
    }

    progress("Building the option index…");
    let on_skip = |_: &str, count: usize| {
        progress(&format!(
            "Evaluating option documentation (skipping {count} broken)"
        ));
    };

    let evaluated = option_index(
        nix,
        target,
        cache,
        || progress("Evaluating option documentation…"),
        &on_skip,
    );

    let saved = save(cache, OPTIONS_FILE, "option index", evaluated);

    if saved {
        cache.save_text(OPTIONS_KEY_FILE, index_key);
    }

    saved
}

fn save_installed(nix: &Nix, target: &Target, cache: &Cache) -> bool {
    match drakeflake_flake::installed_packages(nix, target) {
        Ok(names) => {
            cache.write_list(
                INSTALLED_FILE,
                &names.iter().map(ToString::to_string).collect::<Vec<_>>(),
            );
            true
        }
        Err(error) => {
            log::warn!("could not read installed packages: {error}");
            false
        }
    }
}

fn save_values(nix: &Nix, target: &Target, cache: &Cache, progress: Progress<'_>) -> bool {
    progress("Evaluating current option values…");
    let on_skip = |_: &str, count: usize| {
        progress(&format!(
            "Evaluating current option values (skipping {count} that fail)"
        ));
    };

    let evaluated = option_values(nix, target, cache, &on_skip);

    save(cache, VALUES_FILE, "option values", evaluated)
}

fn save_packages(nix: &Nix, target: &Target, cache: &Cache, progress: Progress<'_>) -> bool {
    progress("Indexing packages…");
    let on_skip = |_: &str, count: usize| {
        progress(&format!("Indexing packages (skipping {count} broken)"));
    };

    let evaluated = package_index(nix, target, cache, &on_skip);

    save(cache, PACKAGES_FILE, "package index", evaluated)
}
