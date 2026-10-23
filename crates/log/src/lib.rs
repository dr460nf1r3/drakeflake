mod file;
mod format;
mod terminal;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::OnceLock;

use drakeflake_fs::DirsError;
use file::FileLogger;
pub use log::{LevelFilter, debug, error, info, trace, warn};
pub use terminal::set_terminal;

pub const LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];
pub const DEFAULT_LEVEL: &str = "info";

/// Target of records read back from the program's own stderr (the app's
/// stderr tee): they already reached the terminal, so only the file gets them.
pub const STDERR_TARGET: &str = "stderr";

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

#[must_use = "this only parses the name, `set_level` applies it"]
pub fn parse_level(name: &str) -> Option<LevelFilter> {
    match name.trim().to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}

pub fn log_dir() -> Result<PathBuf, DirsError> {
    drakeflake_fs::state_dir()
}

pub fn log_file(program: &str) -> Result<PathBuf, DirsError> {
    Ok(log_dir()?.join(format!("{program}.log")))
}

pub fn init(program: &str, level: &str) -> Option<PathBuf> {
    let path = log_file(program);
    let logger = LOGGER.get_or_init(|| match &path {
        Ok(path) => FileLogger::open(path.clone()),
        Err(err) => FileLogger::unavailable(err.to_string()),
    });

    log::set_logger(logger).ok()?;
    set_level(level);
    info!(
        "{program} {} started (log level {})",
        env!("CARGO_PKG_VERSION"),
        self::level()
    );

    if let Some(reason) = logger.failure() {
        warn!("no log file: {reason}. Logging to the terminal only");
        return None;
    }

    path.ok()
}

pub fn set_level(level: &str) {
    let filter = parse_level(level).unwrap_or_else(|| {
        warn!("unknown log level {level:?}; using {DEFAULT_LEVEL}");
        LevelFilter::Info
    });

    log::set_max_level(filter);
}

#[must_use = "this only reads the current level"]
pub fn level() -> String {
    log::max_level().as_str().to_ascii_lowercase()
}
