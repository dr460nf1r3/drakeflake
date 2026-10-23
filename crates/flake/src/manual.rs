use std::path::{Path, PathBuf};

use drakeflake_exec::LONG_TIMEOUT;
use drakeflake_render::render_string;

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::MANUAL;
use crate::flake_target::{Target, get_flake_ref};
use crate::nix::run_nix;
use crate::pool::with_eval_slot;

pub fn build_options_json(nix: &Nix, target: &Target) -> Result<PathBuf, FlakeError> {
    let own = format!(
        "{}.config.system.build.manual.optionsJSON",
        target.config_ref()?
    );

    let cwd = target.local_dir();
    let expr = manual_expr(target);

    let stdout = build(
        nix,
        cwd.as_deref(),
        &[
            "build",
            "--no-link",
            "--print-out-paths",
            "--impure",
            "--expr",
            &expr,
        ],
    )
    .or_else(|forced| {
        // A replaced load is not a failure: no fallback build.
        if forced.is_cancelled() || nix.is_cancelled() {
            return Err(forced);
        }

        log::warn!(
            "building the manual with all modules failed; trying the configuration's own manual"
        );
        build(
            nix,
            cwd.as_deref(),
            &["build", "--no-link", "--print-out-paths", &own],
        )
        .map_err(|plain| match plain {
            stopped if stopped.is_cancelled() || stopped.is_timeout() => stopped,
            other => FlakeError::ManualBuilds {
                forced: forced.to_string(),
                plain: other.to_string(),
            },
        })
    })?;

    let file = options_json_in(&stdout)?;

    log::debug!("options.json: {}", file.display());
    Ok(file)
}

fn options_json_in(stdout: &str) -> Result<PathBuf, FlakeError> {
    let out = stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with('/'))
        .ok_or(FlakeError::NoOutputPath)?;

    // NixOS installs the manual under `share/doc/nixos`, nix-darwin under
    // `share/doc/darwin`.
    ["nixos", "darwin"]
        .iter()
        .map(|doc| {
            Path::new(out)
                .join("share/doc")
                .join(doc)
                .join("options.json")
        })
        .find(|file| file.is_file())
        .ok_or_else(|| FlakeError::NoOptionsJson(out.to_owned()))
}

fn manual_expr(target: &Target) -> String {
    MANUAL
        .replace("__FLAKE__", &render_string(&get_flake_ref(&target.flake)))
        .replace("__ATTR__", target.kind.attr())
        .replace("__HOST__", &render_string(&target.host))
}

fn build(nix: &Nix, cwd: Option<&Path>, args: &[&str]) -> Result<String, FlakeError> {
    with_eval_slot(nix, || run_nix(nix, cwd, args, LONG_TIMEOUT))?
}

#[cfg(test)]
mod tests;
