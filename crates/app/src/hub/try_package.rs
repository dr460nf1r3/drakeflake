use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::Nix;

use super::argv::arg;
use super::{Cache, Hub, HubError, terminal};

const EXEC_SHELL: &str = "exec \"${SHELL:-sh}\"";

#[derive(Debug, PartialEq, Eq)]
enum Launch {
    App {
        name: String,
        command: Vec<OsString>,
        terminal: bool,
    },
    Shell {
        bins: Vec<PathBuf>,
    },
    Nothing,
}

pub fn try_package(owner: &Arc<Hub>, attr: String) -> Result<(), HubError> {
    if !is_attr_path(&attr) {
        return Err(HubError::invalid(format!(
            "\"{attr}\" is not a package attribute"
        )));
    }

    let target = owner.target().ok_or(HubError::NoTarget)?;
    let installable = format!(
        "{}.pkgs.{attr}",
        target
            .config_ref()
            .map_err(|error| HubError::Library(Box::new(error)))?
    );

    owner.spawn_background("try", move |hub, nix| {
        let built = {
            let _activity = hub
                .console()
                .activity("try", format!("Fetching {attr} to try it…"));

            build_for_trying(nix, &installable, target.local_dir().as_deref(), &attr)
        };

        let outcome = built
            .map_err(|error| format!("Could not build {attr} to try it: {error}"))
            .and_then(|paths| start(&attr, &launch_for(&paths)));

        match outcome {
            Ok(message) => hub.log(message),
            Err(error) => hub.log_error(error),
        }
    });

    Ok(())
}

fn build_for_trying(
    nix: &Nix,
    installable: &str,
    cwd: Option<&Path>,
    attr: &str,
) -> Result<Vec<PathBuf>, String> {
    let root = Cache::root().map_err(|err| err.to_string())?;

    build_linked(nix, installable, cwd, &root.join("try"), attr)
}

pub fn build_tool(
    nix: &Nix,
    target: &drakeflake_flake::Target,
    attr: &str,
) -> Result<Vec<PathBuf>, String> {
    if !is_attr_path(attr) {
        return Err(format!("\"{attr}\" is not a package attribute"));
    }

    let installable = format!(
        "{}.pkgs.{attr}",
        target.config_ref().map_err(|error| error.to_string())?
    );

    let root = Cache::root().map_err(|err| err.to_string())?;

    build_linked(
        nix,
        &installable,
        target.local_dir().as_deref(),
        &root.join("tools"),
        attr,
    )
}

fn build_linked(
    nix: &Nix,
    installable: &str,
    cwd: Option<&Path>,
    dir: &Path,
    attr: &str,
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(dir)
        .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    let link = dir.join(attr);
    log::debug!("building {installable} linked at {}", link.display());

    let output = drakeflake_flake::run_nix(
        nix,
        cwd,
        [
            arg("build"),
            arg(installable),
            arg("--out-link"),
            arg(&link),
            arg("--print-out-paths"),
        ],
        drakeflake_exec::LONG_TIMEOUT,
    )
    .map_err(|error| error.to_string())?;

    let paths: Vec<PathBuf> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();

    log::debug!("built {attr}: {paths:?}");
    Ok(paths)
}

fn launch_for(paths: &[PathBuf]) -> Launch {
    let desktop = paths.iter().find_map(|path| {
        let entries = std::fs::read_dir(path.join("share/applications")).ok()?;
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|file| file.extension().is_some_and(|ext| ext == "desktop"))
            .filter_map(|file| {
                std::fs::read_to_string(&file)
                    .inspect_err(|err| log::warn!("cannot read {}: {err}", file.display()))
                    .ok()
            })
            .find_map(|text| parse_desktop_entry(&text, path))
    });

    if let Some(app) = desktop {
        log::debug!("launch: {app:?}");
        return app;
    }

    let bins: Vec<PathBuf> = paths
        .iter()
        .map(|path| path.join("bin"))
        .filter(|bin| bin.is_dir())
        .collect();

    let launch = if bins.is_empty() {
        Launch::Nothing
    } else {
        Launch::Shell { bins }
    };

    log::debug!("launch: {launch:?}");
    launch
}

fn parse_desktop_entry(text: &str, package: &Path) -> Option<Launch> {
    let mut in_entry = false;
    let (mut name, mut exec, mut terminal, mut hidden) = (None, None, false, false);

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }

        let Some((key, value)) = line.split_once('=').filter(|_| in_entry) else {
            continue;
        };

        match key.trim() {
            "Name" => name = Some(value.trim().to_owned()),
            "Exec" => exec = Some(value.trim().to_owned()),
            "Terminal" => terminal = value.trim() == "true",
            "NoDisplay" | "Hidden" => hidden |= value.trim() == "true",
            _ => {}
        }
    }

    let mut command: Vec<OsString> = exec?
        .split_whitespace()
        .filter(|word| !(word.len() == 2 && word.starts_with('%')))
        .map(|word| arg(word.trim_matches('"')))
        .collect();

    let program = command.first_mut()?;
    let local = package.join("bin").join(&*program);

    if !Path::new(&*program).is_absolute() && local.is_file() {
        *program = local.into_os_string();
    }

    let name = name.unwrap_or_else(|| program.to_string_lossy().into_owned());

    (!hidden).then_some(Launch::App {
        name,
        command,
        terminal,
    })
}

fn start(attr: &str, launch: &Launch) -> Result<String, String> {
    match launch {
        Launch::App {
            name,
            command,
            terminal: false,
        } => terminal::spawn_detached(command)
            .map(|()| format!("Started {name} from {attr} (not installed)."))
            .map_err(|error| format!("Could not start {name}: {error}")),
        Launch::App {
            command,
            name: _,
            terminal: _,
        } => report(
            attr,
            terminal::open(&terminal::wait_after(&terminal::command_line(command))),
        ),
        Launch::Shell { bins } => report(attr, terminal::open(&shell_script(attr, bins))),
        Launch::Nothing => Ok(format!(
            "{attr} has no programs to run (it is a library or data package)."
        )),
    }
}

fn report(attr: &str, opened: Result<(), String>) -> Result<String, String> {
    opened
        .map(|()| format!("Opened a terminal to try {attr} (not installed)."))
        .map_err(|error| format!("Could not open a terminal to try {attr}: {error}"))
}

fn shell_script(attr: &str, bins: &[PathBuf]) -> String {
    let path: Vec<String> = bins
        .iter()
        .map(|bin| bin.to_string_lossy().into_owned())
        .collect();

    let listed: Vec<String> = path.iter().map(|bin| terminal::quote(bin)).collect();

    format!(
        "export PATH={}:\"$PATH\"; echo {}; ls {}; echo; {EXEC_SHELL}",
        terminal::quote(&path.join(":")),
        terminal::quote(&format!(
            "{attr} is available in this shell, not installed. Its programs:"
        )),
        listed.join(" ")
    )
}

fn is_attr_path(attr: &str) -> bool {
    !attr.is_empty()
        && attr.split('.').all(|part| {
            !part.is_empty()
                && !part.starts_with('-')
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '\''))
        })
}

#[cfg(test)]
mod tests;
