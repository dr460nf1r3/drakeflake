use std::time::{Duration, Instant};

use drakeflake_exec::{Context, redact_urls};

use super::remote::{immutable_link, query_param, release_name, remote_rev, without_query};
use super::{InputKind, LockedInput, UpdateCheck, is_input_name};
use crate::error::LockError;
use crate::parallel::par_map;

const MAX_PARALLEL_CHECKS: usize = 8;
const MIN_REV_LEN: usize = 7;

#[must_use = "the network checks are wasted if the result is dropped"]
pub fn check_updates<'a>(
    exec: &Context,
    inputs: &'a [LockedInput],
) -> Vec<(&'a LockedInput, UpdateCheck)> {
    let started = Instant::now();
    let checks = par_map(
        inputs.iter().collect(),
        MAX_PARALLEL_CHECKS,
        |_, input| check_update(exec, input),
        |index| {
            log::warn!(
                "{}: the update check panicked",
                inputs.get(index).map_or("an input", |input| &input.name)
            );
            UpdateCheck::Failed {
                error: "the check crashed".to_owned(),
            }
        },
    );

    let checked: Vec<(&LockedInput, UpdateCheck)> = inputs.iter().zip(checks).collect();

    log_checks(&checked, started.elapsed());
    checked
}

fn log_checks(checked: &[(&LockedInput, UpdateCheck)], elapsed: Duration) {
    let (mut available, mut failed) = (0_usize, 0_usize);

    for (input, check) in checked {
        match check {
            UpdateCheck::Available {
                rev: _,
                compare_url: _,
                lock_url: _,
            } => available += 1,
            UpdateCheck::Failed { error } => {
                failed += 1;
                log::warn!(
                    "checking {} ({}) for updates failed: {}",
                    input.name,
                    redact_urls(&input.source),
                    redact_urls(error)
                );
                continue;
            }
            UpdateCheck::UpToDate | UpdateCheck::NotCheckable { reason: _ } => {}
        }

        log::debug!("{}: {}", input.name, redact_urls(&format!("{check:?}")));
    }

    log::info!(
        "flake.lock: {} inputs checked in {:.1}s, {available} updates available, {failed} checks failed",
        checked.len(),
        elapsed.as_secs_f32()
    );
}

#[must_use = "the network check is wasted if the result is dropped"]
pub fn check_update(exec: &Context, input: &LockedInput) -> UpdateCheck {
    let not_checkable = |reason: &str| UpdateCheck::NotCheckable {
        reason: reason.to_owned(),
    };

    if !is_input_name(&input.name) {
        return not_checkable("the input name is not a plain identifier");
    }

    if input.follows.is_some() {
        return not_checkable("follows another input");
    }

    if input.pinned {
        return not_checkable("pinned to a fixed revision");
    }

    if matches!(input.kind, InputKind::Tarball | InputKind::File) {
        return input.remote.as_deref().map_or_else(
            || not_checkable("not a web address"),
            |url| check_tarball(exec, url, input),
        );
    }

    let (Some(remote), Some(locked)) = (&input.remote, &input.rev) else {
        return not_checkable(match input.kind {
            InputKind::Path => "local path",
            InputKind::Git => "local repository",
            InputKind::Indirect => "flake registry entry",
            InputKind::Github
            | InputKind::Gitlab
            | InputKind::Sourcehut
            | InputKind::Tarball
            | InputKind::File
            | InputKind::Follows
            | InputKind::Other(_) => "source type cannot be checked",
        });
    };

    compare_rev(
        input,
        locked,
        remote_rev(exec, remote, input.reference.as_deref()),
    )
}

fn compare_rev(
    input: &LockedInput,
    locked: &str,
    latest: Result<String, LockError>,
) -> UpdateCheck {
    match latest {
        Ok(latest) if latest == locked => UpdateCheck::UpToDate,
        Ok(latest) => UpdateCheck::Available {
            compare_url: compare_url(input, &latest),
            lock_url: input
                .pin_base
                .as_ref()
                .map(|base| format!("{base}/{latest}")),
            rev: latest,
        },
        Err(error) => UpdateCheck::Failed {
            error: error.to_string(),
        },
    }
}

fn check_tarball(exec: &Context, url: &str, input: &LockedInput) -> UpdateCheck {
    compare_tarball(input, immutable_link(exec, url))
}

fn compare_tarball(input: &LockedInput, link: Result<Option<String>, LockError>) -> UpdateCheck {
    let link = match link {
        Ok(Some(link)) => link,
        Ok(None) => {
            return UpdateCheck::NotCheckable {
                reason: "the server does not say which version it serves".to_owned(),
            };
        }
        Err(error) => {
            return UpdateCheck::Failed {
                error: error.to_string(),
            };
        }
    };

    let latest = query_param(&link, "rev");

    let current = if let (Some(latest), Some(locked)) = (&latest, &input.rev) {
        latest == locked
    } else {
        log::debug!(
            "{}: no revision to compare; comparing tarball URLs",
            input.name
        );
        input
            .locked_url
            .as_deref()
            .is_some_and(|locked| without_query(locked) == without_query(&link))
    };

    if current {
        UpdateCheck::UpToDate
    } else {
        let rev = latest.unwrap_or_else(|| release_name(&link));
        UpdateCheck::Available {
            compare_url: compare_url(input, &rev),
            lock_url: (input.kind == InputKind::Tarball
                && (link.starts_with("https://") || link.starts_with("http://")))
            .then_some(link),
            rev,
        }
    }
}

pub fn compare_url(input: &LockedInput, new: &str) -> Option<String> {
    let old = input.rev.as_deref()?;
    let base = input.web_url.as_deref()?;
    let is_rev =
        |rev: &str| rev.len() >= MIN_REV_LEN && rev.chars().all(|ch| ch.is_ascii_hexdigit());

    if !is_rev(old) || !is_rev(new) {
        None
    } else if base.starts_with("https://github.com/") || input.kind == InputKind::Github {
        Some(format!("{base}/compare/{old}...{new}"))
    } else if input.kind == InputKind::Gitlab {
        Some(format!("{base}/-/compare/{old}...{new}"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
