use std::ffi::OsString;
use std::process::{Child, Command, Stdio};

use super::argv::arg;
use super::find_program;

pub const OPENER: &str = if cfg!(target_os = "macos") {
    "open"
} else {
    "xdg-open"
};
const TERMINALS: &[(&str, &[&str])] = &[
    ("xdg-terminal-exec", &[]),
    ("konsole", &["-e"]),
    ("ghostty", &["-e"]),
    ("kitty", &[]),
    ("alacritty", &["-e"]),
    ("foot", &[]),
    ("wezterm", &["start", "--"]),
    ("kgx", &["--"]),
    ("gnome-terminal", &["--"]),
    ("xfce4-terminal", &["-x"]),
    ("xterm", &["-e"]),
];

pub fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

pub fn command_line(words: &[OsString]) -> String {
    words
        .iter()
        .map(|word| quote(&word.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn wait_after(script: &str) -> String {
    format!(
        "{script}; status=$?; echo; if [ $status -ne 0 ]; then echo \"Failed (exit $status).\"; fi; read -r -p 'Press Enter to close.' _"
    )
}

pub fn open(script: &str) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        log::debug!("opening Terminal.app");
        return spawn_detached(&[
            arg("osascript"),
            arg("-e"),
            arg(terminal_app_script(script)),
        ]);
    }

    let (program, args) = TERMINALS
        .iter()
        .find_map(|(name, args)| find_program(name).map(|path| (path, *args)))
        .ok_or("no terminal emulator found (install konsole or set up xdg-terminal-exec)")?;

    log::debug!("opening a terminal with {}", program.display());

    let mut command = vec![arg(&program)];
    command.extend(args.iter().map(arg));
    command.extend([arg("sh"), arg("-c"), arg(script)]);

    spawn_detached(&command)
}

fn terminal_app_script(script: &str) -> String {
    let line = format!("sh -c {}", quote(script));
    let escaped = line.replace('\\', "\\\\").replace('"', "\\\"");
    format!("tell application \"Terminal\"\nactivate\ndo script \"{escaped}\"\nend tell")
}

pub fn spawn_detached(command: &[OsString]) -> Result<(), String> {
    use std::os::unix::process::CommandExt as _;

    let (program, args) = command.split_first().ok_or("empty command")?;
    let shown = program.to_string_lossy();
    log::debug!("starting detached: {shown} {args:?}");

    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|err| format!("{shown}: {err}"))?;

    if let Err(err) = reap(child) {
        log::warn!("cannot watch {shown}: {err}");
    }

    Ok(())
}

fn reap(mut child: Child) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("df-reap".to_owned())
        .spawn(move || {
            if let Err(err) = child.wait() {
                log::warn!("waiting for a detached program failed: {err}");
            }
        })
}

#[cfg(test)]
mod tests;
