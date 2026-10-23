use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use drakeflake_exec::Context;
use drakeflake_render::OptionName;
use serde::Serialize;

use super::argv::arg;
use super::{Hub, find_program, terminal};

/// It runs on the Qt thread.
const MIME_QUERY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub display: String,
    pub local: bool,
}

#[must_use = "only looks the option up, nothing is opened"]
pub fn locate(option: &OptionName, files: &[String], flake_dir: Option<&Path>) -> Vec<Location> {
    files
        .iter()
        .map(|file| {
            let local = flake_dir.and_then(|dir| local_copy(file, dir));
            let mut path = local.clone().unwrap_or_else(|| PathBuf::from(file));
            if path.is_dir() {
                path = path.join("default.nix");
            }

            let line = std::fs::read_to_string(&path).map_or_else(
                |err| {
                    log::debug!("cannot read {} to find {option}: {err}", path.display());
                    1
                },
                |text| find_line(&text, option),
            );

            Location {
                file: path.to_string_lossy().into_owned(),
                line,
                display: match (&local, flake_dir) {
                    (Some(local), Some(dir)) => local.strip_prefix(dir).map_or_else(
                        |_| local.to_string_lossy().into_owned(),
                        |relative| relative.to_string_lossy().into_owned(),
                    ),
                    _ => path.to_string_lossy().into_owned(),
                },
                local: local.is_some(),
            }
        })
        .collect()
}

fn local_copy(file: &str, flake_dir: &Path) -> Option<PathBuf> {
    let rest = file.strip_prefix("/nix/store/")?;
    let (_, relative) = rest.split_once('/')?;
    let candidate = flake_dir.join(relative);

    let local = if candidate.is_dir() {
        candidate.join("default.nix")
    } else {
        candidate
    };

    local.is_file().then_some(local)
}

fn find_line(text: &str, option: &str) -> usize {
    let relative = option
        .strip_prefix("home-manager.users.")
        .and_then(|rest| rest.split_once('.'))
        .map(|(_, inside)| inside);

    std::iter::once(option)
        .chain(relative)
        .find_map(|name| {
            drakeflake_validate::definition_sites(text, &OptionName::from(name))
                .first()
                .copied()
        })
        .map_or_else(
            || {
                log::debug!("{option}: no literal definition in the file");
                1
            },
            |site| site.line,
        )
}

pub fn clamp_line(line: i32) -> usize {
    usize::try_from(line).unwrap_or(1).max(1)
}

pub fn open_and_report(hub: &Hub, file: &str, line: i32) {
    match open_in_editor(hub.nix().context(), file, clamp_line(line)) {
        Ok(done) => hub.log(done),
        Err(error) => hub.log_error(error),
    }
}

pub fn open_in_editor(exec: &Context, file: &str, line: usize) -> Result<String, String> {
    let path = Path::new(file);

    if !path.is_absolute() {
        return Err(format!("Could not open {file}: not an absolute path"));
    }

    if path.extension().is_none_or(|ext| ext != "nix") {
        return Err(format!("Could not open {file}: only .nix files are opened"));
    }

    if let Ok(meta) = std::fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(format!("Could not open {file}: it is a symlink"));
    }

    open_at(exec, path, line)
}

pub fn open_own_file(exec: &Context, file: &Path) -> Result<String, String> {
    open_at(exec, file, 1)
}

fn open_at(exec: &Context, file: &Path, line: usize) -> Result<String, String> {
    let opened = gui_editor_command(exec, file, line)
        .or_else(|| mac_editor_command(file, line))
        .map_or_else(
            || open_with_default_app(file),
            |command| terminal::spawn_detached(&command),
        );

    let shown = file.display();

    opened
        .map(|()| format!("Opened {shown} at line {line}."))
        .map_err(|error| format!("Could not open {shown}: {error}"))
}

fn at_line(file: &Path, line: usize) -> OsString {
    let mut text = file.as_os_str().to_owned();
    text.push(format!(":{line}"));
    text
}

fn mac_editor_command(file: &Path, line: usize) -> Option<Vec<OsString>> {
    let (app, bundle) = crate::macos::default_text_app()?;
    log::debug!("default text editor: {bundle} at {}", app.display());

    Some(mac_editor_arguments(&app, &bundle, file, line))
}

fn mac_editor_arguments(app: &Path, bundle: &str, file: &Path, line: usize) -> Vec<OsString> {
    let bundle = bundle.to_ascii_lowercase();
    let position = at_line(file, line);
    let tool = if bundle.starts_with("dev.zed.zed") {
        Some(("Contents/MacOS/cli", vec![position]))
    } else if bundle == "com.microsoft.vscode" {
        Some(("Contents/Resources/app/bin/code", vec![arg("-g"), position]))
    } else if bundle == "com.vscodium" {
        Some((
            "Contents/Resources/app/bin/codium",
            vec![arg("-g"), position],
        ))
    } else if bundle.starts_with("com.sublimetext.") {
        Some(("Contents/SharedSupport/bin/subl", vec![position]))
    } else {
        None
    };

    if let Some((relative, args)) = tool {
        let program = app.join(relative);

        if program.is_file() {
            let mut command = vec![arg(&program)];
            command.extend(args);
            return command;
        }

        log::debug!(
            "{} is missing, opening the app without a line",
            program.display()
        );
    }

    vec![arg("open"), arg("-a"), arg(app), arg(file)]
}

fn open_with_default_app(file: &Path) -> Result<(), String> {
    let mut command = vec![arg(terminal::OPENER)];

    if cfg!(target_os = "macos") {
        command.push(arg("-t"));
    }

    command.push(arg(file));
    terminal::spawn_detached(&command)
}

fn gui_editor_command(exec: &Context, file: &Path, line: usize) -> Option<Vec<OsString>> {
    let output = exec
        .command(
            "xdg-mime",
            ["query", "default", "text/plain"],
            MIME_QUERY_TIMEOUT,
        )
        .inspect_err(|err| log::debug!("xdg-mime failed: {err}"))
        .ok()?;

    let desktop = output.stdout.to_ascii_lowercase();
    log::debug!("default text editor: {}", desktop.trim());

    let (program, args) = editor_arguments(&desktop, file, line)?;
    let Some(path) = find_program(program) else {
        log::debug!("{program} is the default editor but not in PATH");
        return None;
    };

    let mut command = vec![arg(&path)];
    command.extend(args);
    Some(command)
}

fn editor_arguments(
    desktop: &str,
    file: &Path,
    line: usize,
) -> Option<(&'static str, Vec<OsString>)> {
    let plus_line = || vec![arg(format!("+{line}")), arg(file)];
    let line_flag = || vec![arg("--line"), arg(line.to_string()), arg(file)];

    Some(if desktop.contains("kate") {
        ("kate", line_flag())
    } else if desktop.contains("kwrite") {
        ("kwrite", line_flag())
    } else if desktop.contains("codium") {
        ("codium", vec![arg("-g"), at_line(file, line)])
    } else if desktop.contains("code") {
        ("code", vec![arg("-g"), at_line(file, line)])
    } else if desktop.contains("zed") {
        ("zeditor", vec![at_line(file, line)])
    } else if desktop.contains("gnome.texteditor") || desktop.contains("gnome-text-editor") {
        ("gnome-text-editor", plus_line())
    } else if desktop.contains("emacs") {
        ("emacs", plus_line())
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests;
