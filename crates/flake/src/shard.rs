use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use drakeflake_exec::{ExecError, LONG_TIMEOUT};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::resolve;
use crate::nix::{failure, json_arg, parse_json, run_nix_raw};
use crate::parallel::par_map;
use crate::pool::{eval_jobs, eval_slot, lock};
use crate::tree::flatten_options;

const MAX_RETRIES: usize = 80;

const FRAME_MARKERS: [&str; 2] = [
    "while evaluating attribute '",
    "while evaluating the attribute '",
];

pub type OnSkip<'a> = &'a (dyn Fn(&str, usize) + Sync);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Shard {
    pub index: usize,
    pub count: usize,
}

impl Shard {
    #[must_use = "the shard is only built"]
    pub const fn single() -> Self {
        Self { index: 0, count: 1 }
    }
}

impl std::fmt::Display for Shard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.index, self.count)
    }
}

#[derive(Clone, Copy)]
pub struct EvalJob<'a> {
    pub what: &'a str,
    pub installable: &'a str,
    pub cwd: Option<&'a Path>,
    pub template: &'a str,
    pub degrade: bool,
    pub walker: bool,
    pub split: usize,
}

/// Walker output nests keys that extend each other, so the deepest frame of the chain is the broken entry.
#[must_use = "stderr is only parsed"]
pub fn failing_attribute(stderr: &str) -> Option<String> {
    let mut frames = stderr.lines().flat_map(|line| {
        FRAME_MARKERS.iter().filter_map(move |marker| {
            let (_, rest) = line.split_once(marker)?;
            let (name, _) = rest.rsplit_once('\'')?;
            Some(unquote_attr(name))
        })
    });

    let outermost = frames.next()?;

    Some(frames.fold(outermost, |deepest, frame| {
        let extends = frame
            .strip_prefix(deepest.as_str())
            .is_some_and(|rest| rest.starts_with('.'));

        if extends { frame } else { deepest }
    }))
}

fn unquote_attr(text: &str) -> String {
    text.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .map_or_else(
            || text.to_owned(),
            |inner| inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        )
}

fn next_skip(skip: &[String], name: &str, degrade: bool) -> Option<String> {
    let bare = format!("~{name}");

    if degrade && !skip.contains(&bare) {
        Some(bare)
    } else if !skip.iter().any(|entry| entry == name) {
        Some(name.to_owned())
    } else {
        None
    }
}

pub fn eval_with_retries(
    nix: &Nix,
    job: EvalJob<'_>,
    shard: &Shard,
    skip: &mut Vec<String>,
    on_skip: &dyn Fn(&str),
) -> Result<String, FlakeError> {
    let template = resolve(job.template);
    let shard_arg = json_arg(shard)?;

    for round in 1..=MAX_RETRIES {
        if nix.is_cancelled() {
            return Err(ExecError::Cancelled(None).into());
        }

        let Some(_slot) = eval_slot(nix).filter(|_| !nix.is_cancelled()) else {
            return Err(ExecError::Cancelled(None).into());
        };

        let apply = format!("({template}) {shard_arg} {}", json_arg(skip)?);
        let captured = run_nix_raw(
            nix,
            job.cwd,
            [
                "eval",
                "--json",
                "--show-trace",
                job.installable,
                "--apply",
                &apply,
            ],
            LONG_TIMEOUT,
            log::Level::Debug,
        )?;

        if captured.success {
            return Ok(captured.stdout);
        }

        let Some(entry) =
            failing_attribute(&captured.stderr).and_then(|bad| next_skip(skip, &bad, job.degrade))
        else {
            let error = failure(&captured.stderr);
            log::warn!("shard {shard}: evaluation failed and names nothing to skip: {error}");
            return Err(error);
        };

        log::debug!("shard {shard}: skipping {entry}, which fails to evaluate (round {round})");
        on_skip(&entry);
        skip.push(entry);
    }

    log::warn!(
        "shard {shard}: gave up after {MAX_RETRIES} retries ({} entries skipped)",
        skip.len()
    );
    Err(FlakeError::GaveUp {
        skipped: skip.len(),
    })
}

fn eval_sharded(
    nix: &Nix,
    job: EvalJob<'_>,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<Vec<String>, FlakeError> {
    let count = eval_jobs() * job.split.max(1);
    let total = AtomicUsize::new(skip.len());
    let shared = Mutex::new(std::mem::take(skip));
    let workers = nix.child().workers(count);

    let run_shard = |index: usize, worker: &Nix| {
        let mut local = lock(&shared).clone();
        let selection = Shard { index, count };
        let result = eval_with_retries(worker, job, &selection, &mut local, &|entry| {
            on_skip(entry, total.fetch_add(1, Ordering::SeqCst) + 1);
        });

        let mut merged = lock(&shared);
        for entry in local {
            if !merged.contains(&entry) {
                merged.push(entry);
            }
        }

        result
    };

    let results = run_shards(workers, job.what, &run_shard);

    *skip = shared.into_inner().unwrap_or_else(PoisonError::into_inner);
    first_failure(results)
}

/// The first real failure cancels the siblings, which would otherwise run to their long timeout.
fn run_shards(
    workers: Vec<Nix>,
    what: &str,
    work: &(dyn Fn(usize, &Nix) -> Result<String, FlakeError> + Sync),
) -> Vec<Result<String, FlakeError>> {
    let threads = workers.len();

    par_map(
        workers,
        threads,
        |index, worker| {
            let result = work(index, &worker);

            if result.as_ref().is_err_and(|error| !error.is_cancelled()) {
                worker.cancel();
            }

            result
        },
        |_| {
            log::warn!("{what}: an evaluation thread panicked");
            Err(FlakeError::ThreadCrashed("an evaluation thread"))
        },
    )
}

/// All outputs, or the first real error: a sibling stopped because of it
/// reports `Cancelled`, which must not hide the cause.
fn first_failure(results: Vec<Result<String, FlakeError>>) -> Result<Vec<String>, FlakeError> {
    let mut outputs = Vec::with_capacity(results.len());
    let mut cancelled = None;

    for result in results {
        match result {
            Ok(output) => outputs.push(output),
            Err(error) if error.is_cancelled() => {
                cancelled.get_or_insert(error);
            }
            Err(error) => return Err(error),
        }
    }

    cancelled.map_or(Ok(outputs), Err)
}

pub fn eval_merged(
    nix: &Nix,
    job: EvalJob<'_>,
    skip: &mut Vec<String>,
    on_skip: OnSkip<'_>,
) -> Result<Map<String, Value>, FlakeError> {
    let started = Instant::now();
    let skipped_before = skip.len();
    let outputs = eval_sharded(nix, job, skip, on_skip)?;
    let merged = merge_shards(&outputs, job.walker)?;

    log::info!(
        "evaluated {} {} in {:.1}s ({} shards, {} newly skipped, {} skipped in total)",
        merged.len(),
        job.what,
        started.elapsed().as_secs_f32(),
        outputs.len(),
        skip.len().saturating_sub(skipped_before),
        skip.len()
    );
    if !skip.is_empty() && log::log_enabled!(log::Level::Debug) {
        log::debug!("{} skipped: {}", job.what, skip.join(", "));
    }

    Ok(merged)
}

pub fn merge_shards(outputs: &[String], walker: bool) -> Result<Map<String, Value>, FlakeError> {
    let mut merged = Map::new();

    for output in outputs {
        let value: Value = parse_json(output, "evaluation")?;

        if walker {
            merged.extend(flatten_options(value));
        } else if let Value::Object(map) = value {
            merged.extend(map);
        } else {
            log::warn!("ignoring evaluation output that is not an object");
        }
    }

    Ok(merged)
}

#[cfg(test)]
mod tests;
