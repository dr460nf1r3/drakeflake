mod darwin;
mod nixos;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use drakeflake_exec::Context;
use drakeflake_flake::{SystemKind, Target};

use super::Mode;
use super::argv::arg;
use super::generations::Generation;
use super::operations::{Exclusive, Outcome};
use super::rebuild::Rebuild;

const NH_HELP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    NixOs,
    Darwin,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NhFeatures {
    pub elevation_strategy: bool,
    pub diff: bool,
}

impl NhFeatures {
    fn from_help(help: &str) -> Self {
        Self {
            elevation_strategy: help.contains("--elevation-strategy"),
            diff: help.contains("--diff"),
        }
    }
}

impl System {
    pub const fn of(kind: SystemKind) -> Self {
        match kind {
            SystemKind::NixOs => Self::NixOs,
            SystemKind::Darwin => Self::Darwin,
        }
    }

    pub const fn native() -> Self {
        Self::of(SystemKind::native())
    }

    pub const fn nh_subcommand(self) -> &'static str {
        match self {
            Self::NixOs => "os",
            Self::Darwin => "darwin",
        }
    }

    pub const fn rebuild_program(self) -> &'static str {
        match self {
            Self::NixOs => "nixos-rebuild",
            Self::Darwin => "darwin-rebuild",
        }
    }

    pub fn nh_features(self, exec: &Context, nh: &Path) -> NhFeatures {
        let help = exec.command(
            nh,
            [self.nh_subcommand(), "switch", "--help"],
            NH_HELP_TIMEOUT,
        );

        match help {
            Ok(captured) => NhFeatures::from_help(&captured.stdout),
            Err(error) => {
                log::warn!(
                    "cannot ask {} for its options, assuming the oldest: {error}",
                    nh.display()
                );
                NhFeatures::default()
            }
        }
    }

    pub fn review_args(
        self,
        dir: &Path,
        host: &str,
        mode: Mode,
        elevation: Option<&Path>,
    ) -> Vec<OsString> {
        let mut args = vec![
            arg(self.nh_subcommand()),
            arg(mode.verb()),
            arg(dir),
            arg("--hostname"),
            arg(host),
            arg("--ask"),
        ];

        if let Some(program) = elevation {
            args.extend([arg("--elevation-strategy"), arg(program)]);
        }

        args
    }

    pub fn rebuild_as_root(self, run: &Exclusive, request: Rebuild<'_>) -> Outcome {
        match self {
            Self::NixOs => nixos::rebuild_as_root(run, request),
            Self::Darwin => darwin::rebuild_as_root(run, request),
        }
    }

    pub fn review(self, run: &Exclusive, target: &Target, mode: Mode, dir: &Path) -> Outcome {
        match self {
            Self::NixOs => nixos::review(run, target, mode, dir),
            Self::Darwin => darwin::review(run, target, mode, dir),
        }
    }

    pub fn activate_generation(
        self,
        run: &Exclusive,
        generation: &Generation,
        mode: Mode,
    ) -> Outcome {
        match self {
            Self::NixOs => nixos::activate_generation(run, generation, mode),
            Self::Darwin => darwin::activate_generation(run, generation, mode),
        }
    }
}
