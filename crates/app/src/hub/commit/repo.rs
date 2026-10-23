#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use drakeflake_exec::Context;
use drakeflake_file::{GitState, outside_work_tree};
use drakeflake_flake::lock::{LockedInput, parse_lock, read_lock};

use crate::hub::argv::arg;

const GIT_TIMEOUT: Duration = Duration::from_secs(20);

const FALLBACK_NAME: &str = "DrakeFlake";

pub fn ensure_repository(exec: &Context, dir: &Path) -> Result<bool, String> {
    let probe = exec.probe(
        "git",
        git_in(dir, &["rev-parse", "--is-inside-work-tree"]),
        GIT_TIMEOUT,
    );

    match outside_work_tree(&probe, dir) {
        None => Ok(false),
        Some(GitState::NotARepository) => {
            let created = exec
                .command("git", git_in(dir, &["init", "-q"]), GIT_TIMEOUT)
                .map_err(|error| error.to_string())?;

            if created.success {
                log::info!("created a git repository in {}", dir.display());
                Ok(true)
            } else {
                Err(created.stderr.trim().to_owned())
            }
        }
        Some(_) => Err(probe.map_or_else(
            |error| error.to_string(),
            |captured| captured.stderr.trim().to_owned(),
        )),
    }
}

pub fn stage_all(exec: &Context, dir: &Path) -> Result<Vec<String>, String> {
    let added = exec
        .command("git", git_in(dir, &["add", "-A", "--", "."]), GIT_TIMEOUT)
        .map_err(|error| error.to_string())?;

    if !added.success {
        return Err(added.stderr.trim().to_owned());
    }

    let staged = exec
        .command(
            "git",
            git_in(
                dir,
                &["diff", "--cached", "--name-only", "--relative", "--", "."],
            ),
            GIT_TIMEOUT,
        )
        .map_err(|error| error.to_string())?;

    if !staged.success {
        return Err(staged.stderr.trim().to_owned());
    }

    Ok(staged.stdout.lines().map(str::to_owned).collect())
}

pub fn commit_staged(
    exec: &Context,
    dir: &Path,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    let configured = |key: &str| {
        exec.probe("git", git_in(dir, &["config", key]), GIT_TIMEOUT)
            .is_ok_and(|captured| captured.success && !captured.stdout.trim().is_empty())
    };

    let identity = fallback_identity(
        configured("user.name"),
        configured("user.email"),
        drakeflake_flake::local_hostname().as_deref(),
    );

    let mut args = git_in(dir, &[]);
    args.extend(identity.iter().map(arg));
    args.extend(["commit", "-m", subject, "-m", body, "--", "."].map(arg));

    let captured = exec
        .command("git", &args, GIT_TIMEOUT)
        .map_err(|error| error.to_string())?;

    if captured.success {
        Ok(captured
            .stdout
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned())
    } else {
        Err(captured.stderr.trim().to_owned())
    }
}

fn git_in(dir: &Path, rest: &[&str]) -> Vec<OsString> {
    [arg("-C"), arg(dir)]
        .into_iter()
        .chain(rest.iter().map(arg))
        .collect()
}

fn fallback_identity(has_name: bool, has_email: bool, host: Option<&str>) -> Vec<String> {
    let mut options = Vec::new();

    if !has_name {
        options.extend(["-c".to_owned(), format!("user.name={FALLBACK_NAME}")]);
    }

    if !has_email {
        let host = host.filter(|host| !host.is_empty()).unwrap_or("localhost");
        options.extend(["-c".to_owned(), format!("user.email=drakeflake@{host}")]);
    }

    options
}

#[must_use = "only reads the committed lock, nothing is changed"]
pub fn head_lock(exec: &Context, dir: &Path) -> Option<Vec<LockedInput>> {
    let shown = exec
        .probe(
            "git",
            git_in(dir, &["show", "HEAD:./flake.lock"]),
            GIT_TIMEOUT,
        )
        .ok()
        .filter(|captured| captured.success)?;

    parse_lock(&shown.stdout).ok()
}

#[must_use = "only reads the lock, nothing is changed"]
pub fn worktree_lock(dir: &Path) -> Vec<LockedInput> {
    read_lock(dir).unwrap_or_default()
}
