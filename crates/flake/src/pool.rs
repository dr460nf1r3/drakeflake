//! Urgent work gets free slots before other work, so quick results the UI needs are not stuck behind long bulk evaluations.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use drakeflake_exec::{ExecError, POLL_INTERVAL};

use crate::context::Nix;

/// Whole-configuration evals of a real system grow to 2-3 GiB each.
const EVAL_MEMORY_KIB: u64 = 3 * 1024 * 1024;
const MAX_EVAL_JOBS: usize = 8;
const MAX_CONFIGURED_EVAL_JOBS: usize = 64;

static EVAL_SLOTS: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
static URGENT_WAITING: AtomicUsize = AtomicUsize::new(0);
static LOGGED_LIMIT: AtomicUsize = AtomicUsize::new(0);

/// Created on the thread that starts the work: an urgent priority counts as waiting right away, so work started later cannot take the slots first.
pub struct Priority {
    urgent: bool,
    pending: Mutex<Option<UrgentWaiter>>,
}

impl Priority {
    #[must_use = "the priority is only built, nothing is applied"]
    pub fn urgent() -> Self {
        Self {
            urgent: true,
            pending: Mutex::new(Some(UrgentWaiter::register())),
        }
    }

    #[must_use = "the priority is only built, nothing is applied"]
    pub const fn normal() -> Self {
        Self {
            urgent: false,
            pending: Mutex::new(None),
        }
    }

    #[must_use = "only reads the flag"]
    pub const fn is_urgent(&self) -> bool {
        self.urgent
    }

    #[must_use = "the priority is only built"]
    pub fn handed_on(&self) -> Self {
        if self.urgent {
            Self::urgent()
        } else {
            Self::normal()
        }
    }

    pub fn release(&self) {
        drop(self.take_pending());
    }

    fn take_pending(&self) -> Option<UrgentWaiter> {
        lock(&self.pending).take()
    }
}

pub struct EvalSlot;

impl Drop for EvalSlot {
    fn drop(&mut self) {
        let (used, freed) = &EVAL_SLOTS;
        {
            let mut used = lock(used);
            *used = used.saturating_sub(1);
        }

        // Waiters differ in urgency: let each check whether it may go.
        freed.notify_all();
    }
}

struct UrgentWaiter;

impl UrgentWaiter {
    fn register() -> Self {
        URGENT_WAITING.fetch_add(1, Ordering::AcqRel);
        Self
    }
}

impl Drop for UrgentWaiter {
    fn drop(&mut self) {
        URGENT_WAITING.fetch_sub(1, Ordering::AcqRel);
    }
}

pub fn with_eval_slot<R>(nix: &Nix, job: impl FnOnce() -> R) -> Result<R, ExecError> {
    let _slot = eval_slot(nix)
        .filter(|_| !nix.is_cancelled())
        .ok_or(ExecError::Cancelled(None))?;

    Ok(job())
}

#[must_use = "the job count is only computed"]
pub fn jobs_for(threads: usize, available_kib: Option<u64>) -> usize {
    let by_memory = available_kib.map_or(threads, |kib| {
        usize::try_from(kib / 2 / EVAL_MEMORY_KIB).unwrap_or(usize::MAX)
    });

    threads.min(by_memory).clamp(1, MAX_EVAL_JOBS)
}

fn mem_available_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/meminfo")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[must_use = "the job count is only computed"]
pub fn eval_jobs() -> usize {
    let configured = std::env::var("DRAKEFLAKE_EVAL_JOBS")
        .ok()
        .and_then(|value| {
            let jobs = value.trim().parse::<usize>().ok().filter(|jobs| *jobs > 0);

            if jobs.is_none() {
                log::warn!("ignoring DRAKEFLAKE_EVAL_JOBS={value:?}: not a positive number");
            }

            jobs
        });

    if let Some(jobs) = configured {
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        let budgeted = jobs_for(threads, mem_available_kib());
        let capped = jobs.min(MAX_CONFIGURED_EVAL_JOBS).min(budgeted.max(1));

        if capped < jobs {
            log::warn!("clamping DRAKEFLAKE_EVAL_JOBS={jobs} to {capped} for available memory");
        }

        return capped;
    }

    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let available_kib = mem_available_kib();
    jobs_for(threads, available_kib)
}

fn eval_limit() -> usize {
    let limit = eval_jobs();

    if LOGGED_LIMIT.swap(limit, Ordering::AcqRel) != limit {
        log::debug!("{limit} concurrent evaluations");
    }

    limit
}

pub fn eval_slot(nix: &Nix) -> Option<EvalSlot> {
    let limit = eval_limit();
    let priority = nix.priority();
    let urgent = priority.is_urgent();
    let pending = priority.take_pending();
    let _waiter = urgent.then(|| pending.unwrap_or_else(UrgentWaiter::register));
    let (used, freed) = &EVAL_SLOTS;
    let mut used = lock(used);

    while *used >= limit
        || (!urgent && URGENT_WAITING.load(Ordering::Acquire) > 0 && *used + 1 >= limit)
    {
        if nix.is_cancelled() {
            return None;
        }

        used = freed
            .wait_timeout(used, POLL_INTERVAL)
            .map_or_else(|poisoned| poisoned.into_inner().0, |(guard, _)| guard);
    }

    *used += 1;
    drop(used);
    Some(EvalSlot)
}

/// Locks also after a panic of another holder, the guarded data stays usable.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
