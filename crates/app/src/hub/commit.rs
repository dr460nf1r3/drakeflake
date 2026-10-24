mod message;
mod repo;

use std::path::{Path, PathBuf};

use drakeflake_flake::Target;
use drakeflake_plan::ManagedState;
use message::commit_message;
use repo::{commit_staged, ensure_repository, head_lock, stage_all, worktree_lock};

use super::operations::Exclusive;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    pub dir: Option<PathBuf>,
    pub created_repository: bool,
}

pub fn prepare(run: &Exclusive, target: &Target) -> Prepared {
    if !run.session().auto_commit() {
        return Prepared::default();
    }

    let Some(dir) = target.local_dir() else {
        log::debug!("automatic commit skipped: the flake is not local");
        return Prepared::default();
    };

    let context = run.nix().context();

    match ensure_repository(context, &dir) {
        Ok(false) => Prepared {
            dir: Some(dir),
            created_repository: false,
        },
        Ok(true) => {
            run.console()
                .log(format!("Created a git repository in {}", dir.display()));

            match stage_all(context, &dir) {
                Ok(_) => Prepared {
                    dir: Some(dir),
                    created_repository: true,
                },
                Err(error) => {
                    run.console()
                        .warn(format!("Cannot stage the flake in git: {error}"));
                    Prepared {
                        dir: None,
                        created_repository: true,
                    }
                }
            }
        }
        Err(error) => {
            run.console().warn(format!(
                "Automatic commit skipped, no git repository: {error}"
            ));
            Prepared::default()
        }
    }
}

pub fn commit_applied(
    run: &Exclusive,
    dir: &Path,
    previous: Option<&ManagedState>,
    plan: &ManagedState,
) {
    let context = run.nix().context();
    let old_lock = head_lock(context, dir);
    let (subject, mut body) =
        commit_message(previous, plan, old_lock.as_deref(), &worktree_lock(dir));

    let staged = match stage_all(context, dir) {
        Ok(staged) => staged,
        Err(error) => {
            run.console()
                .warn(format!("Automatic commit skipped, cannot stage: {error}"));
            return;
        }
    };

    if staged.is_empty() {
        log::debug!("automatic commit skipped: nothing changed");
        return;
    }

    if body.is_empty() {
        body = staged
            .iter()
            .map(|file| format!("- {file} updated"))
            .collect::<Vec<_>>()
            .join("\n");
    }

    match commit_staged(context, dir, &subject, &body) {
        Ok(summary) => {
            run.console()
                .log(format!("Committed the changes ({summary})"));
            run.record_commit(summary);
        }
        Err(error) => run
            .console()
            .warn(format!("Automatic commit failed: {error}")),
    }
}
