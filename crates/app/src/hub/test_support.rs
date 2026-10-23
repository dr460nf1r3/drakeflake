use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use drakeflake_flake::{SystemKind, Target};

use super::{ConsoleMsg, Hub, SystemMsg};

pub fn hub_with(target: Option<Target>) -> Arc<Hub> {
    let hub = Arc::new(Hub::default());
    hub.session().update(|state| state.target = target);
    hub
}

pub fn local_target(dir: &Path) -> Target {
    Target {
        flake: dir.to_string_lossy().into_owned(),
        host: "box".into(),
        kind: SystemKind::NixOs,
    }
}

pub fn texts(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

pub fn console_lines(hub: &Hub) -> Vec<String> {
    hub.mail
        .console
        .take()
        .into_iter()
        .flat_map(|message| match message {
            ConsoleMsg::Lines(lines) => lines,
            ConsoleMsg::Screen {
                session: _,
                changes: _,
            }
            | ConsoleMsg::Prompt(_) => Vec::new(),
        })
        .collect()
}

/// Panics after a few seconds, so a lost `Finished` fails the test instead of hanging it.
pub fn system_until_finished(hub: &Hub) -> Vec<SystemMsg> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = Vec::new();

    loop {
        seen.extend(hub.mail.system.take());
        if seen
            .iter()
            .any(|message| matches!(message, SystemMsg::Finished { ok: _, backup: _ }))
        {
            return seen;
        }

        assert!(Instant::now() < deadline, "the operation never finished");
        std::thread::sleep(Duration::from_millis(5));
    }
}
