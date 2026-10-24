use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use drakeflake_flake::{SystemKind, Target};
use drakeflake_render::HostName;

use super::generations::{Generation, PROFILES, list_generations};
use super::{ConsoleMsg, Hub, ImagesMsg, SystemMsg};

#[cfg(test)]
mod tests;

const CONSOLE_TAIL: usize = 200;
const POLL_INTERVAL: Duration = Duration::from_millis(100);

struct Finished {
    ok: bool,
    backup: Option<PathBuf>,
    console: Vec<String>,
}

#[must_use = "the target is only read, nothing runs yet"]
fn hub_target() -> Option<Target> {
    Some(Target {
        flake: std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?,
        host: std::env::var("DRAKEFLAKE_TEST_HOST")
            .ok()
            .map(HostName::from)?,
        kind: SystemKind::NixOs,
    })
}

fn copied_hub_target() -> Option<(Target, tempfile::TempDir)> {
    let source = std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?;
    let host = std::env::var("DRAKEFLAKE_TEST_HOST").ok()?;
    let parent = tempfile::TempDir::new().expect("temp dir for the fixture copy");
    let dir = parent.path().join("flake");

    let copy = std::process::Command::new("cp")
        .args(["-r", &source, &dir.to_string_lossy()])
        .status()
        .expect("cp -r the fixture");

    assert!(copy.success(), "cp -r {source} failed");

    let writable = std::process::Command::new("chmod")
        .args(["-R", "u+w", &dir.to_string_lossy()])
        .status()
        .expect("chmod the fixture copy");

    assert!(writable.success(), "chmod the fixture copy failed");

    Some((
        Target {
            flake: dir.to_string_lossy().into_owned(),
            host: host.into(),
            kind: SystemKind::NixOs,
        },
        parent,
    ))
}

#[must_use = "the target is only read, nothing runs yet"]
fn lifecycle_target() -> Option<Target> {
    Some(Target {
        flake: std::env::var("DRAKEFLAKE_TEST_LIFECYCLE_FLAKE").ok()?,
        host: "lifecyclehost".into(),
        kind: SystemKind::NixOs,
    })
}

#[must_use = "only switches the system when true"]
fn vm_guest() -> bool {
    std::env::var("DRAKEFLAKE_VM_GUEST").is_ok_and(|value| value == "1")
}

fn lifecycle_state_file() -> PathBuf {
    std::env::var("DRAKEFLAKE_TEST_LIFECYCLE_STATE").map_or_else(
        |_| PathBuf::from("/tmp/vm-itest/lifecycle-state"),
        PathBuf::from,
    )
}

fn record_generation(role: &str, id: u32) {
    let file = lifecycle_state_file();

    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).expect("cannot create the lifecycle state folder");
    }

    let previous = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => panic!("cannot read the lifecycle state: {err}"),
    };

    let prefix = format!("{role}=");
    let kept: Vec<&str> = previous
        .lines()
        .filter(|line| !line.starts_with(prefix.as_str()))
        .collect();

    let mut state = kept.join("\n");
    if !state.is_empty() {
        state.push('\n');
    }

    writeln!(state, "{prefix}{id}").unwrap();
    std::fs::write(&file, state).expect("cannot write the lifecycle state");
}

#[must_use = "only reads the state file, nothing runs"]
fn recorded_generation(role: &str) -> Option<u32> {
    let prefix = format!("{role}=");

    std::fs::read_to_string(lifecycle_state_file())
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str())?.parse().ok())
}

fn generation_id(pick: impl Fn(&Generation) -> bool) -> Option<u32> {
    list_generations(Path::new(PROFILES))
        .into_iter()
        .find(pick)
        .map(|generation| generation.id)
}

#[must_use = "only reads the profiles, nothing is activated"]
fn current_generation() -> Option<u32> {
    generation_id(|generation| generation.current)
}

#[must_use = "only reads the profiles, nothing is activated"]
fn running_generation() -> Option<u32> {
    generation_id(|generation| generation.running)
}

fn poll<T>(timeout: Duration, mut check: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(found) = check() {
            return Some(found);
        }

        if Instant::now() >= deadline {
            return None;
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_finish(hub: &Hub, timeout: Duration) -> Option<Finished> {
    let mut console = Vec::new();

    let (ok, backup) = poll(timeout, || {
        for message in hub.mail.console.take() {
            match message {
                ConsoleMsg::Prompt(_) => hub.answer(true),
                ConsoleMsg::Lines(mut lines) => console.append(&mut lines),
                ConsoleMsg::Screen {
                    session: _,
                    changes,
                } => {
                    console.extend(changes.rows);
                    let excess = console.len().saturating_sub(CONSOLE_TAIL);
                    console.drain(..excess);
                }
            }
        }

        hub.mail.system.take().into_iter().find_map(|message| {
            if let SystemMsg::Finished {
                ok,
                backup,
                commit: _,
            } = message
            {
                Some((ok, backup))
            } else {
                None
            }
        })
    })?;

    Some(Finished {
        ok,
        backup,
        console,
    })
}

fn wait_for_formats(hub: &Hub, timeout: Duration) -> Option<Result<Vec<String>, String>> {
    poll(timeout, || {
        hub.mail.images.take().into_iter().find_map(|message| {
            if let ImagesMsg::Formats(result) = message {
                Some(result)
            } else {
                None
            }
        })
    })
}
