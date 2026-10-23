use std::ffi::{OsStr, OsString};
use std::path::Path;

use drakeflake_file::Elevation;
use drakeflake_flake::Target;

use super::System;
use super::nixos::rebuild_tool_command;
use crate::hub::argv::arg;
use crate::hub::generations::Generation;
use crate::hub::operations::{Exclusive, Outcome};
use crate::hub::rebuild::{Rebuild, console_elevation};
use crate::hub::{Mode, find_program, find_root_program, in_store};

pub(super) const DARWIN_SYSTEM_PROFILE: &str = "/nix/var/nix/profiles/system";

pub(super) const DARWIN_SWITCH_SCRIPT: &str = "export HOME=/var/root; nix=\"$1\"; system=\"$2\"; profile=\"$3\"; \"$nix\" build --no-link --profile \"$profile\" \"$system\" && \"$system/sw/bin/darwin-rebuild\" activate";

const DARWIN_CONFIRM_SCRIPT: &str =
    "printf 'Apply the config?'; read -r answer; [ \"$answer\" = y ] || [ \"$answer\" = Y ]";

fn switch_only(run: &Exclusive, mode: Mode) -> bool {
    let only_switch = mode == Mode::Switch;

    if !only_switch {
        run.console()
            .error("nix-darwin hosts can only be switched.");
    }

    only_switch
}

pub(super) fn rebuild_as_root(run: &Exclusive, request: Rebuild<'_>) -> Outcome {
    let Rebuild {
        installable,
        system,
        mode,
        flake_dir,
    } = request;

    if !switch_only(run, mode) {
        return Outcome::Failed;
    }

    if find_program("nh").is_some() {
        let built = match run.build(installable, system, flake_dir) {
            Ok(built) => built,
            Err(failed) => return failed,
        };

        return activate(run, &built, flake_dir);
    }

    let Some(elevation) = console_elevation(run.console()) else {
        return Outcome::Failed;
    };

    let rebuild = match find_root_program(system.rebuild_program()) {
        Ok(rebuild) => rebuild,
        Err(error) => {
            run.console().error(error);
            return Outcome::Failed;
        }
    };

    let command = rebuild_tool_command(&elevation, &rebuild, mode, installable);
    run.stream_command(&command, flake_dir)
}

pub(super) fn review(run: &Exclusive, target: &Target, mode: Mode, dir: &Path) -> Outcome {
    if !switch_only(run, mode) {
        return Outcome::Failed;
    }

    let Some(nh) = find_program("nh") else {
        run.console()
            .error("Review needs nh (programs.nh.enable = true); use Apply instead.");
        return Outcome::Failed;
    };

    let installable = format!("{}#{}", target.flake.trim(), target.host);
    let system = System::Darwin;
    let built = match run.build(&installable, system, dir) {
        Ok(built) => built,
        Err(failed) => return failed,
    };

    if !system.nh_features(run.nix().context(), &nh).diff {
        run.console()
            .log("Changes from the running system to the rebuilt one:");
        let shown = run.diff_systems(Path::new("/run/current-system"), &built);
        if !shown.is_success() {
            return shown;
        }
    }

    let confirmed = run.stream_pty(
        OsStr::new("sh"),
        &[arg("-c"), arg(DARWIN_CONFIRM_SCRIPT)],
        dir,
    );

    if !confirmed.is_success() {
        run.console().log("Not applying.");
        return confirmed;
    }

    activate(run, &built, dir)
}

pub(super) fn activate_generation(run: &Exclusive, generation: &Generation, mode: Mode) -> Outcome {
    if !switch_only(run, mode) {
        return Outcome::Failed;
    }

    let rebuild = match find_root_program(System::Darwin.rebuild_program()) {
        Ok(rebuild) => rebuild,
        Err(error) => {
            run.console().error(error);
            return Outcome::Failed;
        }
    };

    let Some(elevation) = console_elevation(run.console()) else {
        return Outcome::Failed;
    };

    let command = switch_generation_command(&elevation, &rebuild, generation.id);
    run.work_dir()
        .map_or(Outcome::Failed, |dir| run.stream_quiet(&command, &dir))
}

pub(super) fn switch_generation_command(
    elevation: &Elevation,
    rebuild: &Path,
    id: u32,
) -> Vec<OsString> {
    let mut command = elevation.prefix();
    command.extend([
        arg(rebuild),
        arg("--switch-generation"),
        arg(id.to_string()),
    ]);
    command
}

fn activate(run: &Exclusive, system: &Path, workdir: &Path) -> Outcome {
    let Some(elevation) = console_elevation(run.console()) else {
        return Outcome::Failed;
    };

    let programs = find_root_program("nix").and_then(|nix| Ok((nix, in_store(system)?)));
    let (nix, built) = match programs {
        Ok(programs) => programs,
        Err(error) => {
            run.console().error(error);
            return Outcome::Failed;
        }
    };

    let command = activation_command(&elevation, &nix, &built);
    run.stream_quiet(&command, workdir)
}

pub(super) fn activation_command(
    elevation: &Elevation,
    nix: &Path,
    system: &Path,
) -> Vec<OsString> {
    let mut command = elevation.prefix();
    command.extend([
        arg("/bin/sh"),
        arg("-c"),
        arg(DARWIN_SWITCH_SCRIPT),
        arg("sh"),
        arg(nix),
        arg(system),
        arg(DARWIN_SYSTEM_PROFILE),
    ]);
    command
}
