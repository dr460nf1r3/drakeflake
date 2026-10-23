use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use drakeflake_file::Elevation;
use drakeflake_flake::Target;

use super::System;
use crate::hub::argv::{arg, echo};
use crate::hub::generations::Generation;
use crate::hub::operations::{Exclusive, Outcome};
use crate::hub::rebuild::{Rebuild, console_elevation};
use crate::hub::{Mode, find_program, find_root_program, helper_path};

pub(super) fn review(run: &Exclusive, target: &Target, mode: Mode, dir: &Path) -> Outcome {
    let Some(nh) = find_program("nh") else {
        run.console()
            .error("Review needs nh (programs.nh.enable = true); use Apply instead.");
        return Outcome::Failed;
    };

    let elevation = console_elevation(run.console());
    let args = System::NixOs.review_args(
        dir,
        &target.host,
        mode,
        elevation.as_ref().map(Elevation::program),
    );

    run.console().log(echo(OsStr::new("nh"), &args));
    run.stream_pty(nh.as_os_str(), &args, dir)
}

pub(super) fn rebuild_as_root(run: &Exclusive, request: Rebuild<'_>) -> Outcome {
    let helper = helper_path();
    log::debug!("privileged helper {helper:?}");

    let Some(helper) = helper else {
        run.console().warn(
            "The privileged helper was not found; your password is asked for by the rebuild tool.",
        );
        return rebuild_without_helper(run, request);
    };

    let Some(elevation) = console_elevation(run.console()) else {
        return Outcome::Failed;
    };

    let system = match run.build(request.installable, System::NixOs, request.flake_dir) {
        Ok(system) => system,
        Err(failed) => return failed,
    };

    let command = activation_command(&elevation, &helper, request.mode, &system);

    run.work_dir()
        .map_or(Outcome::Failed, |dir| run.stream_quiet(&command, &dir))
}

pub(super) fn activation_command(
    elevation: &Elevation,
    helper: &Path,
    mode: Mode,
    system: &Path,
) -> Vec<OsString> {
    let mut command = elevation.prefix();
    command.extend([arg(helper), arg("activate"), arg(mode.verb()), arg(system)]);
    command
}

fn rebuild_without_helper(run: &Exclusive, request: Rebuild<'_>) -> Outcome {
    let Rebuild {
        installable,
        system,
        mode,
        flake_dir,
    } = request;

    let elevation = console_elevation(run.console());
    let rebuild = system.rebuild_program();

    let (program, args): (PathBuf, Vec<OsString>) = if let Some(nh) = find_program("nh") {
        let mut args = vec![
            arg(system.nh_subcommand()),
            arg(mode.verb()),
            arg(installable),
        ];

        let features = system.nh_features(run.nix().context(), &nh);
        log::debug!("rebuilding with {}, nh supports {features:?}", nh.display());

        if let (Some(elevation), true) = (&elevation, features.elevation_strategy) {
            args.push(arg("--elevation-strategy"));
            args.push(arg(elevation.program()));
        }

        (nh, args)
    } else if let (Ok(rebuild), Some(elevation)) = (find_root_program(rebuild), &elevation) {
        let command = rebuild_tool_command(elevation, &rebuild, mode, installable);
        return run.stream_command(&command, flake_dir);
    } else {
        run.console().error(format!(
            "Neither nh nor {rebuild} with a way to get root is available."
        ));
        return Outcome::Failed;
    };

    run.console().log(echo(program.as_os_str(), &args));
    run.stream_pty(program.as_os_str(), &args, flake_dir)
}

pub(super) fn rebuild_tool_command(
    elevation: &Elevation,
    rebuild: &Path,
    mode: Mode,
    installable: &str,
) -> Vec<OsString> {
    let mut command = elevation.prefix();
    command.extend([
        arg(rebuild),
        arg(mode.verb()),
        arg("--flake"),
        arg(installable),
    ]);
    command
}

pub(super) fn activate_generation(run: &Exclusive, generation: &Generation, mode: Mode) -> Outcome {
    if mode == Mode::Switch
        && let Some(nh) = find_program("nh")
    {
        return rollback(run, &nh, generation);
    }

    let Some(helper) = helper_path() else {
        run.console()
            .error("The privileged helper is not available; install the packaged app.");
        return Outcome::Failed;
    };

    let Some(elevation) = console_elevation(run.console()) else {
        return Outcome::Failed;
    };

    log::debug!("activating through {}", helper.display());
    let command = activation_command(&elevation, &helper, mode, Path::new(&generation.path));

    run.work_dir()
        .map_or(Outcome::Failed, |dir| run.stream_quiet(&command, &dir))
}

fn rollback(run: &Exclusive, nh: &Path, generation: &Generation) -> Outcome {
    log::debug!("switching with {}", nh.display());

    let elevation = console_elevation(run.console());
    let args = rollback_args(generation.id, elevation.as_ref().map(Elevation::program));
    let mut command = vec![arg(nh)];
    command.extend(args);

    run.work_dir()
        .map_or(Outcome::Failed, |dir| run.stream_quiet(&command, &dir))
}

pub(super) fn rollback_args(id: u32, elevation: Option<&Path>) -> Vec<OsString> {
    let mut args = vec![
        arg(System::NixOs.nh_subcommand()),
        arg("rollback"),
        arg("--to"),
        arg(id.to_string()),
    ];

    if let Some(program) = elevation {
        args.extend([arg("--elevation-strategy"), arg(program)]);
    }

    args
}
