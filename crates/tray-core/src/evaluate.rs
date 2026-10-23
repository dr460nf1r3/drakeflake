use std::sync::atomic::{AtomicBool, Ordering};

use drakeflake_config::Config;
use drakeflake_evaluate::cache::Cache;
use drakeflake_flake::{
    FlakeError, Nix, Target, choose_host, list_hosts, local_hostname, normalize_flake,
};

use crate::error::EvaluationError;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[must_use = "a change is lost unless a start that did not happen is tried again"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spawned {
    Started,
    AlreadyRunning,
    Failed,
}

struct Running;

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

pub fn spawn(nix: Nix, config: Config) -> Spawned {
    if RUNNING
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        drakeflake_log::debug!("a background evaluation is already running");
        return Spawned::AlreadyRunning;
    }

    let started = std::thread::Builder::new()
        .name("evaluate".to_owned())
        .spawn(move || {
            // Cleared on every exit, a panic included.
            let _running = Running;
            lower_priority();

            if let Err(error) = run(&nix, &config) {
                drakeflake_log::warn!("background evaluation failed: {error}");
            }
        });

    match started {
        Ok(_) => Spawned::Started,
        Err(error) => {
            RUNNING.store(false, Ordering::Release);
            drakeflake_log::warn!("cannot start the background evaluation: {error}");
            Spawned::Failed
        }
    }
}

/// Linux niceness is per thread, so only the evaluation and the `nix` it starts are slowed.
#[cfg(target_os = "linux")]
fn lower_priority() {
    const LOWEST_PRIORITY: i32 = 19;

    let thread = rustix::thread::gettid();

    if let Err(error) = rustix::process::setpriority_process(Some(thread), LOWEST_PRIORITY) {
        drakeflake_log::debug!("cannot lower the evaluation priority: {error}");
    }
}

/// Elsewhere niceness is per process and cannot be raised again, so the tray keeps its priority.
#[cfg(not(target_os = "linux"))]
const fn lower_priority() {}

fn run(nix: &Nix, config: &Config) -> Result<(), EvaluationError> {
    let target = resolve_target(nix, config)?;
    let fingerprint = drakeflake_flake::fingerprint(nix, &target)?;

    if Cache::for_target(&target).is_current(Some(fingerprint.as_str())) {
        drakeflake_log::info!("the flake is unchanged since the last evaluation");
        return Ok(());
    }

    drakeflake_log::info!(
        "the flake changed, evaluating {}#{}",
        target.flake,
        target.host
    );

    let generated = drakeflake_evaluate::generated_paths(
        &target.flake,
        config.nix.managed_path.as_deref(),
        config.layout.mode,
    );

    let progress = |line: &str| drakeflake_log::debug!("{line}");

    if drakeflake_evaluate::refresh(nix, &target, &fingerprint, &generated, &progress) {
        drakeflake_log::info!("background evaluation finished");
        Ok(())
    } else {
        Err(EvaluationError::Incomplete)
    }
}

fn resolve_target(nix: &Nix, config: &Config) -> Result<Target, FlakeError> {
    let flake = normalize_flake(&config.nix.flake_path);
    let hosts = list_hosts(nix, &flake)?;
    let local = local_hostname();
    let host = choose_host(&hosts, config.nix.host.as_deref(), local.as_deref())?;

    Ok(Target {
        flake,
        host: host.name.into(),
        kind: host.kind,
    })
}
