use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use drakeflake_exec::{Captured, ExecError, Invocation};
use drakeflake_render::render_string;

use crate::context::Nix;
use crate::error::FlakeError;
use crate::flake_target::Target;

pub const NIX_FEATURES: [&str; 2] = ["--extra-experimental-features", "nix-command flakes"];

const ERROR_SUMMARY_LINES: usize = 6;
const TAIL_SUMMARY_LINES: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Query<'a> {
    pub attr: &'a str,
    pub apply: Option<&'a str>,
    pub what: &'a str,
    pub timeout: Duration,
}

impl<'a> Query<'a> {
    #[must_use = "the query is only built"]
    pub const fn new(what: &'a str, timeout: Duration) -> Self {
        Self {
            attr: "",
            apply: None,
            what,
            timeout,
        }
    }

    #[must_use = "the query is only built"]
    pub const fn attr(mut self, attr: &'a str) -> Self {
        self.attr = attr;
        self
    }

    #[must_use = "the query is only built"]
    pub const fn apply(mut self, apply: &'a str) -> Self {
        self.apply = Some(apply);
        self
    }
}

impl Target {
    pub fn eval_json<T>(&self, nix: &Nix, query: &Query<'_>) -> Result<T, FlakeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let installable = format!("{}{}", self.config_ref()?, query.attr);
        let mut args = vec!["eval", "--json", installable.as_str()];
        args.extend(query.apply.iter().flat_map(|apply| ["--apply", apply]));
        let stdout = run_nix(nix, self.local_dir().as_deref(), &args, query.timeout)?;

        parse_json(&stdout, query.what)
    }
}

/// `cwd` should be the flake's directory: Lix resolves relative `git+file:`
/// inputs (e.g. `./submodule`) against the working directory.
pub fn run_nix(
    nix: &Nix,
    cwd: Option<&Path>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    timeout: Duration,
) -> Result<String, FlakeError> {
    let captured = run_nix_raw(nix, cwd, args, timeout, log::Level::Warn)?;

    if captured.success {
        Ok(captured.stdout)
    } else {
        Err(failure(&captured.stderr))
    }
}

pub fn run_nix_raw(
    nix: &Nix,
    cwd: Option<&Path>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    timeout: Duration,
    failure: log::Level,
) -> Result<Captured, ExecError> {
    let features = NIX_FEATURES.iter().map(|arg| OsStr::new(*arg));
    let given: Vec<_> = args.into_iter().collect();
    let full = features.chain(given.iter().map(AsRef::as_ref));
    let invocation = Invocation::new("nix", full, timeout)
        .cwd(cwd)
        .failure(failure)
        .summarize(summarize_error);

    nix.context().run(invocation)
}

pub fn parse_json<T>(stdout: &str, what: &str) -> Result<T, FlakeError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(stdout).map_err(|source| FlakeError::UnexpectedOutput {
        what: what.to_owned(),
        source,
    })
}

#[must_use = "the error is only built"]
pub fn failure(stderr: &str) -> FlakeError {
    let summary = summarize_error(stderr);

    match missing_attribute(&summary) {
        Some(attr) => FlakeError::MissingAttribute { attr, summary },
        None => FlakeError::Nix(summary),
    }
}

fn missing_attribute(summary: &str) -> Option<String> {
    summary.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("error:")?.trim_start();
        let (name, after) = rest.strip_prefix("attribute '")?.split_once('\'')?;
        let after = after.trim_start();

        (after.starts_with("missing")
            || (after.starts_with("in selection path") && after.ends_with("not found")))
        .then(|| name.to_owned())
    })
}

pub fn json_arg(value: &impl serde::Serialize) -> Result<String, FlakeError> {
    let json = serde_json::to_string(value).map_err(|err| FlakeError::UnexpectedOutput {
        what: "argument".to_owned(),
        source: err,
    })?;

    Ok(format!("(builtins.fromJSON {})", render_string(&json)))
}

#[must_use = "the summary is only built, nothing is logged"]
pub fn summarize_error(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim_end)
        .filter(|line| {
            let text = line.trim_start();
            !text.is_empty() && !text.starts_with("warning:") && !text.starts_with("trace:")
        })
        .collect();

    let summary: Vec<&str> = lines
        .iter()
        .rposition(|line| line.trim_start().starts_with("error:"))
        .map_or_else(
            || {
                let tail = lines.len().saturating_sub(TAIL_SUMMARY_LINES);
                lines.iter().skip(tail).copied().collect()
            },
            |position| {
                let error = lines.iter().skip(position).take(ERROR_SUMMARY_LINES);
                error.map(|line| line.trim()).collect()
            },
        );

    summary.join("\n")
}

#[cfg(test)]
mod tests;
