use std::ffi::{OsStr, OsString};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

use super::argv::{arg, echo};
use super::operations::{Exclusive, Outcome, Reporting};
use super::rebuild::console_elevation;
use super::{Cache, Hub, HubError, find_program, find_root_program, generations};

#[derive(Debug, Serialize)]
pub struct Usage {
    pub total: u64,
    pub free: u64,
    pub generations: usize,
    pub tried: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Optimise {
    Run,
    Skip,
}

impl From<bool> for Optimise {
    fn from(run: bool) -> Self {
        if run { Self::Run } else { Self::Skip }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirm {
    Ask,
    Skip,
}

impl Exclusive {
    fn clean(&self, keep: u32, days: u32, optimise: Optimise) -> Outcome {
        let elevation = console_elevation(self.console());

        let Some(nh) = find_program("nh") else {
            let Some(elevation) = elevation else {
                self.console()
                    .error("Cleaning up needs nh, or a way to get root (pkexec, run0 or sudo).");
                return Outcome::Failed;
            };

            let collector = match find_root_program("nix-collect-garbage") {
                Ok(collector) => collector,
                Err(error) => {
                    self.console().error(error);
                    return Outcome::Failed;
                }
            };

            log::warn!(
                "nh not found: falling back to nix-collect-garbage (keep and optimise ignored)"
            );
            let mut command = elevation.prefix();
            command.extend([
                arg(&collector),
                arg("--delete-older-than"),
                arg(format!("{days}d")),
            ]);
            return self
                .work_dir()
                .map_or(Outcome::Failed, |dir| self.stream_command(&command, &dir));
        };

        let program = elevation.as_ref().map(drakeflake_file::Elevation::program);

        let confirm = if matches!(elevation, Some(drakeflake_file::Elevation::MacDialog(_))) {
            Confirm::Skip
        } else {
            Confirm::Ask
        };

        let args = clean_args(keep, days, optimise, program, confirm);
        self.console().log(echo(OsStr::new("nh"), &args));
        self.work_dir().map_or(Outcome::Failed, |dir| {
            self.stream_pty(nh.as_os_str(), &args, &dir)
        })
    }
}

pub fn clean_up(hub: &Arc<Hub>, keep: u32, days: u32, optimise: Optimise) -> Result<(), HubError> {
    hub.spawn_exclusive("cleanup", Reporting::Shown, move |run| {
        run.clean(keep, days, optimise)
    })
}

#[must_use = "only measures the store, nothing is cleaned"]
pub fn usage() -> Usage {
    let (total, free) = rustix::fs::statvfs("/nix/store").map_or_else(
        |err| {
            log::warn!("cannot read the size of /nix/store: {err}");
            (0, 0)
        },
        |stats| {
            (
                stats.f_blocks.saturating_mul(stats.f_frsize),
                stats.f_bavail.saturating_mul(stats.f_frsize),
            )
        },
    );

    Usage {
        total,
        free,
        generations: generations::list_generations(Path::new(generations::PROFILES)).len(),
        tried: tried_dir()
            .and_then(|dir| links_in(&dir))
            .map_or(0, Iterator::count),
    }
}

fn links_in(dir: &Path) -> Option<std::fs::ReadDir> {
    std::fs::read_dir(dir)
        .inspect_err(|err| {
            if err.kind() != ErrorKind::NotFound {
                log::warn!("cannot list {}: {err}", dir.display());
            }
        })
        .ok()
}

fn tried_dir() -> Option<PathBuf> {
    Cache::root().ok().map(|root| root.join("try"))
}

#[must_use = "the number of removed links is what the user is told"]
pub fn remove_tried() -> usize {
    tried_dir().map_or(0, |dir| remove_links(&dir))
}

fn remove_links(dir: &Path) -> usize {
    let Some(entries) = links_in(dir) else {
        return 0;
    };

    let removed = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            std::fs::remove_file(&path)
                .inspect_err(|err| log::warn!("cannot remove {}: {err}", path.display()))
                .is_ok()
        })
        .count();

    log::info!("removed {removed} tried packages");
    removed
}

fn clean_args(
    keep: u32,
    days: u32,
    optimise: Optimise,
    elevation: Option<&Path>,
    confirm: Confirm,
) -> Vec<OsString> {
    let mut args = vec![
        arg("clean"),
        arg("all"),
        arg("--keep"),
        arg(keep.max(1).to_string()),
        arg("--keep-since"),
        arg(format!("{days}d")),
    ];

    if confirm == Confirm::Ask {
        args.push(arg("--ask"));
    }

    if optimise == Optimise::Run {
        args.push(arg("--optimise"));
    }

    if let Some(program) = elevation {
        args.extend([arg("--elevation-strategy"), arg(program)]);
    }

    args
}

#[cfg(test)]
mod tests;
