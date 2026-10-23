mod check;
mod error;
pub mod evaluate;
mod health;
mod icons;
mod single_instance;
mod state;
pub mod watch;

#[cfg(test)]
mod tests;

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub use check::{InputUpdate, check, names, report};
pub use drakeflake_flake::Nix;
pub use error::{CheckError, EvaluationError};
pub use health::{Health, report_health};
pub use icons::icon_dir;
pub use single_instance::single_instance_or_exit;
pub use state::{Request, State};

/// The first update check waits until the session has settled after login.
pub const FIRST_CHECK: Duration = Duration::from_secs(120);
/// The longest sleep: changed settings apply within it.
const SETTINGS_EVERY: Duration = Duration::from_mins(1);

const MANAGER: &str = "drakeflake";

#[must_use = "the context runs every program of the tray"]
pub fn start(program: &str) -> Nix {
    // Default level first so the config load itself is logged.
    drakeflake_log::init(program, drakeflake_log::DEFAULT_LEVEL);
    let (config, _) = drakeflake_config::load();
    drakeflake_log::set_level(&config.general.log_level);
    let nix = Nix::system();

    if std::env::args().nth(1).as_deref() == Some("--check-once") {
        let (code, line) = check_once_result(check(&nix));

        // The exit code still tells the result when stdout is closed.
        writeln!(std::io::stdout(), "{line}").ok();
        for note in Health::read(config.tray).notes() {
            writeln!(std::io::stdout(), "{note}").ok();
        }
        #[expect(
            clippy::exit,
            reason = "--check-once replaces the tray, and the trays' `main` must not start one"
        )]
        std::process::exit(code);
    }

    nix
}

pub fn run(
    nix: &Nix,
    requests: &mpsc::Receiver<Request>,
    mut show: impl FnMut(&State),
    notify: impl Fn(&str, &str),
) {
    let started = Instant::now();
    let mut last_check: Option<Instant> = None;
    let mut last_health: Option<Instant> = None;
    let mut notified: Vec<String> = Vec::new();

    // The last result stays on show while the next check runs.
    let mut current = State::default();

    loop {
        let settings = drakeflake_config::load().0.tray;
        let (next_check, next_health, wake) =
            schedule(settings, started, last_check, last_health, Instant::now());

        let checking_now =
            match requests.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(Request::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Request::Check) => true,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    next_check.is_some_and(|at| Instant::now() >= at)
                }
            };

        if !settings.updates && !current.updates.is_empty() {
            current.updates.clear();
            notified.clear();
            show(&current);
        }

        if Instant::now() >= next_health {
            last_health = Some(Instant::now());
            let health = Health::read(settings);
            if health != current.health {
                report_health(&current.health, &health, &notify);
                current.health = health;
                show(&current);
            }
        }

        if checking_now {
            current.checking = true;
            show(&current);
            let result = check(nix);
            last_check = Some(Instant::now());
            report(&result, &mut notified, &notify);
            let checked = State::from_result(result, &current.updates);
            current = State {
                health: current.health,
                ..checked
            };
            show(&current);
        }
    }
}

fn check_once_result(result: Result<Vec<InputUpdate>, CheckError>) -> (i32, String) {
    match result {
        Ok(updates) if updates.is_empty() => (0_i32, "up to date".to_owned()),
        Ok(updates) => (0_i32, format!("updates available: {}", names(&updates))),
        Err(error) => (1_i32, format!("error: {error}")),
    }
}

/// The wake-up is never later than [`SETTINGS_EVERY`] after `now`, so changed settings apply.
fn schedule(
    settings: drakeflake_config::Tray,
    started: Instant,
    last_check: Option<Instant>,
    last_health: Option<Instant>,
    now: Instant,
) -> (Option<Instant>, Instant, Instant) {
    let next_check = settings
        .updates
        .then(|| last_check.map_or(started + FIRST_CHECK, |at| at + settings.check_every()));

    let next_health = last_health.map_or(started, |at| at + settings.health_every());
    let wake = next_check
        .unwrap_or(next_health)
        .min(next_health)
        .min(now + SETTINGS_EVERY);

    (next_check, next_health, wake)
}

#[must_use = "this only reads the clock"]
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |age| i64::try_from(age.as_secs()).unwrap_or(i64::MAX))
}

#[must_use = "this only builds the lines, the tray shows them"]
pub fn menu_lines(state: &State) -> (Vec<String>, Vec<(String, Option<String>)>) {
    let now = unix_now();
    let inputs = state
        .updates
        .iter()
        .map(|update| (update.label(now), update.link.clone()))
        .collect();

    (state.health.notes(), inputs)
}

/// Links come from `flake.lock`, which is untrusted text, so only `http(s)` ones are opened.
pub fn open_link(url: &str) {
    if !is_web_link(url) {
        drakeflake_log::warn!("not opening {url:?}: only http(s) links are opened");
        return;
    }

    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    spawn_detached(Command::new(opener).arg(url));
}

fn is_web_link(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

pub fn open_manager() {
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(MANAGER)))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(MANAGER));

    drakeflake_log::debug!("opening the manager: {}", program.display());

    spawn_detached(&mut Command::new(&program));
}

/// The child is reaped in the background, or it stays a zombie until the tray quits.
fn spawn_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();

    match child {
        Ok(mut child) => {
            let reaper = std::thread::Builder::new()
                .name("reap-manager".to_owned())
                .spawn(move || {
                    if let Err(error) = child.wait() {
                        drakeflake_log::warn!("waiting for the manager failed: {error}");
                    }
                });

            if let Err(error) = reaper {
                drakeflake_log::warn!("cannot watch the manager process: {error}");
            }
        }
        Err(error) => drakeflake_log::warn!(
            "cannot start {}: {error}",
            command.get_program().to_string_lossy()
        ),
    }
}
