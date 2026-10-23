//! Each step returns whether it produced current data, because only then may the cache be marked current.

use std::sync::Arc;
use std::time::Instant;

use drakeflake_evaluate::cache::{
    Cache, INSTALLED_FILE, OPTIONS_FILE, OPTIONS_KEY_FILE, PACKAGES_FILE, VALUES_FILE,
};

use super::console::Progress;
use super::load::{IndexKey, Loader, log_step_failure};
use super::{OptionsMsg, PackagesMsg};

impl Loader<'_> {
    pub(super) fn refresh_options_index(&self, cache: &Cache, index_key: IndexKey<'_>) -> bool {
        let (nix, target) = (self.nix, self.target);

        if index_key.reusable {
            log::info!("option declarations unchanged, keeping the cached option index");
            return true;
        }

        let started = Instant::now();
        let evaluated = {
            let activity = self
                .console
                .activity("options", "Building the option index…");

            let fallback = format!(
                "Evaluating option documentation, {} in parallel (the {} manual could not be built)…",
                drakeflake_flake::eval_jobs(),
                target.kind.as_str(),
            );

            let on_skip = |_: &str, count: usize| {
                activity.update(format!("{fallback} (skipping {count} broken)"));
            };

            drakeflake_evaluate::option_index(
                nix,
                target,
                cache,
                || activity.update(fallback.clone()),
                &on_skip,
            )
        };

        for warning in skipped_docs_warnings(&evaluated.skipped) {
            self.console.warn(warning);
        }

        if !self.is_current() {
            return false;
        }

        match evaluated.result {
            Ok((json, index)) => {
                log::info!(
                    "option index loaded ({} KiB) in {:.1?}",
                    json.len() / 1024,
                    started.elapsed()
                );
                if cache.read(OPTIONS_FILE).as_deref() == Some(json.as_str()) {
                    log::debug!("option index unchanged from the cache");
                } else {
                    cache.write(OPTIONS_FILE, &json);
                    self.mail.post_options(OptionsMsg::Index(Arc::new(index)));
                }

                cache.save_text(OPTIONS_KEY_FILE, index_key.key);
                true
            }
            Err(error) => {
                log_step_failure("option index", &error);
                self.mail
                    .post_options(OptionsMsg::IndexError(error.to_string()));
                false
            }
        }
    }

    pub(super) fn refresh_installed(&self, cache: &Cache) -> bool {
        let (nix, target) = (self.nix, self.target);

        let result = {
            let _activity = self
                .console
                .activity("installed", "Reading installed packages…");

            drakeflake_flake::installed_packages(nix, target)
        };

        if !self.is_current() {
            return false;
        }

        match result {
            Ok(names) => {
                log::debug!("{} installed packages", names.len());
                cache.write_list(
                    INSTALLED_FILE,
                    &names.iter().map(ToString::to_string).collect::<Vec<_>>(),
                );
                self.mail
                    .packages
                    .post(PackagesMsg::Installed(names.into_iter().collect()));
                true
            }
            Err(error) => {
                self.console
                    .warn(format!("Could not read installed packages: {error}"));
                false
            }
        }
    }

    pub(super) fn refresh_snapshot(&self, cache: &Cache) -> bool {
        let (nix, target) = (self.nix, self.target);

        let label = format!(
            "Evaluating current option values, {} in parallel…",
            drakeflake_flake::eval_jobs()
        );

        let progress = Progress {
            key: "values",
            label: &label,
            skipped: "options that fail to evaluate",
        };

        let started = Instant::now();
        let evaluated = self.console.with_progress(Some(progress), |on_skip| {
            drakeflake_evaluate::option_values(nix, target, cache, on_skip)
        });

        if !self.is_current() {
            return false;
        }

        match evaluated.result {
            Ok((text, snapshot)) => {
                log::info!(
                    "{} options, {} values of user-defined ones evaluated in {:.1?} ({} skipped)",
                    snapshot.options(),
                    snapshot.len(),
                    started.elapsed(),
                    evaluated.skipped.len()
                );
                cache.write(VALUES_FILE, &text);
                self.mail
                    .post_options(OptionsMsg::Snapshot(Arc::new(snapshot)));
                true
            }
            Err(error) => {
                log_step_failure("option values", &error);
                self.mail
                    .post_options(OptionsMsg::SnapshotError(error.to_string()));
                false
            }
        }
    }

    pub(super) fn refresh_packages(&self, cache: &Cache) -> bool {
        let (nix, target) = (self.nix, self.target);

        let label = format!(
            "Indexing packages, {} in parallel…",
            drakeflake_flake::eval_jobs()
        );

        let progress = Progress {
            key: "packages",
            label: &label,
            skipped: "broken",
        };

        let started = Instant::now();
        let evaluated = self.console.with_progress(Some(progress), |on_skip| {
            drakeflake_evaluate::package_index(nix, target, cache, on_skip)
        });

        if !self.is_current() {
            return false;
        }

        match evaluated.result {
            Ok((text, index)) => {
                log::info!(
                    "package index loaded ({} KiB, {} skipped) in {:.1?}",
                    text.len() / 1024,
                    evaluated.skipped.len(),
                    started.elapsed()
                );
                if cache.read(PACKAGES_FILE).as_deref() == Some(text.as_str()) {
                    log::debug!("package index unchanged from the cache");
                } else {
                    cache.write(PACKAGES_FILE, &text);
                    self.mail.packages.post(PackagesMsg::Index(Arc::new(index)));
                }

                true
            }
            Err(error) => {
                log_step_failure("package index", &error);
                self.mail
                    .packages
                    .post(PackagesMsg::IndexError(error.to_string()));
                false
            }
        }
    }
}

fn skipped_docs_warnings(skip: &[String]) -> Vec<String> {
    let (reduced, dropped): (Vec<&str>, Vec<&str>) = skip
        .iter()
        .map(String::as_str)
        .partition(|entry| entry.starts_with('~'));

    let mut warnings = Vec::new();

    if !reduced.is_empty() {
        warnings.push(format!(
            "{} options have documentation that fails to evaluate (e.g. lib.mdDoc, removed from nixpkgs); they are listed without descriptions",
            reduced.len()
        ));
    }

    if !dropped.is_empty() {
        warnings.push(format!(
            "Skipped options that fail to evaluate: {}",
            dropped.join(", ")
        ));
    }

    warnings
}

#[cfg(test)]
mod tests;
