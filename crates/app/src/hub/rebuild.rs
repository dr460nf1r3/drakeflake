use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::Target;

use super::argv::{arg, echo};
use super::console::Console;
use super::operations::{Exclusive, Outcome, Reporting};
use super::system::System;
use super::{Hub, HubError, Mode, find_program};

#[derive(Clone, Copy)]
pub struct Rebuild<'a> {
    pub installable: &'a str,
    pub system: System,
    pub mode: Mode,
    pub flake_dir: &'a Path,
}

impl Exclusive {
    pub fn rebuild_with_review(&self, target: &Target, mode: Mode) -> Outcome {
        let Some(dir) = target.local_dir() else {
            self.console()
                .error("Reviewing a rebuild needs a local flake directory.");
            return Outcome::Failed;
        };

        System::of(target.kind).review(self, target, mode, &dir)
    }

    pub fn rebuild(&self, target: &Target, mode: Mode) -> Outcome {
        let installable = format!("{}#{}", target.flake.trim(), target.host);

        let Some(flake_dir) = target.local_dir().or_else(|| self.work_dir()) else {
            return Outcome::Failed;
        };

        let request = Rebuild {
            installable: &installable,
            system: System::of(target.kind),
            mode,
            flake_dir: &flake_dir,
        };

        if !mode.needs_root() {
            return match self.build(&installable, request.system, &flake_dir) {
                Ok(_) => Outcome::Succeeded,
                Err(failed) => failed,
            };
        }

        request.system.rebuild_as_root(self, request)
    }

    pub(super) fn build(
        &self,
        installable: &str,
        system: System,
        flake_dir: &Path,
    ) -> Result<PathBuf, Outcome> {
        let mut out_link = self.work_dir().ok_or(Outcome::Failed)?.join("result");
        let rebuild = system.rebuild_program();

        let (program, args): (PathBuf, Vec<OsString>) = if let Some(nh) = find_program("nh") {
            let args = vec![
                arg(system.nh_subcommand()),
                arg("build"),
                arg(installable),
                arg("--out-link"),
                arg(&out_link),
            ];

            (nh, args)
        } else if let Some(rebuild) = find_program(rebuild) {
            out_link = flake_dir.join("result");
            let args = vec![arg("build"), arg("--flake"), arg(installable)];

            (rebuild, args)
        } else {
            self.console()
                .error(format!("Neither nh nor {rebuild} is installed."));
            return Err(Outcome::Failed);
        };

        log::debug!("building with {}", program.display());
        self.console().log(echo(program.as_os_str(), &args));

        let built = self.stream_pty(program.as_os_str(), &args, flake_dir);
        if !built.is_success() {
            return Err(built);
        }

        out_link.canonicalize().map_err(|err| {
            self.console()
                .error(format!("The build produced no system ({err})."));
            Outcome::Failed
        })
    }

    pub fn stream_command(&self, command: &[OsString], workdir: &Path) -> Outcome {
        self.run_command(command, workdir, false)
    }

    pub fn stream_quiet(&self, command: &[OsString], workdir: &Path) -> Outcome {
        if !command.is_empty() {
            self.console()
                .log("Activating the system (asks for administrator rights)…");
        }

        self.run_command(command, workdir, true)
    }

    fn run_command(&self, command: &[OsString], workdir: &Path, quiet: bool) -> Outcome {
        let Some((program, args)) = command.split_first() else {
            self.console().error("Nothing to run.");
            return Outcome::Failed;
        };

        if quiet {
            return self.stream_pty_quiet(program, args, workdir);
        }

        self.console().log(echo(program, args));
        self.stream_pty(program, args, workdir)
    }

    pub(super) fn work_dir(&self) -> Option<PathBuf> {
        super::programs::build_dir()
            .map_err(|err| self.console().error(err))
            .ok()
    }
}

pub fn start(hub: &Arc<Hub>, mode: Mode) -> Result<(), HubError> {
    let target = hub.target().ok_or(HubError::NoTarget)?;

    hub.spawn_exclusive("rebuild", Reporting::Shown, move |run| {
        run.rebuild(&target, mode)
    })
}

pub(super) fn console_elevation(console: &Console) -> Option<drakeflake_file::Elevation> {
    let found = drakeflake_file::find_elevation(drakeflake_file::Prompt::TerminalAllowed);

    match &found {
        Some(drakeflake_file::Elevation::Pkexec(_) | drakeflake_file::Elevation::MacDialog(_)) => {}
        Some(other) => console.warn(format!(
            "No setuid pkexec on this system; using {} instead. If it asks for your password here, answer in the bar above the output.",
            other.name()
        )),
        None => console.warn(
            "No way to get administrator rights: neither a setuid pkexec, run0 nor sudo was found.",
        ),
    }

    found
}

#[cfg(test)]
mod tests;
