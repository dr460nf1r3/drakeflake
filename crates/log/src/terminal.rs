use std::fs::File;
use std::io::{IsTerminal as _, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::SystemTime;

use crate::format::{Style, format_record};

/// `None` echoes to stderr. [`set_terminal`] replaces it once stderr is
/// redirected into the log, where echoing would loop.
static TERMINAL: Mutex<Option<Arc<File>>> = Mutex::new(None);
/// Decided once per terminal: records are written far more often than the
/// terminal or `NO_COLOR` change.
static COLOR: AtomicBool = AtomicBool::new(false);
static COLOR_DECIDED: Once = Once::new();

pub fn set_terminal(terminal: File) {
    // Keeps a first echo from deciding for stderr afterwards.
    COLOR_DECIDED.call_once(|| {});
    COLOR.store(wants_color(terminal.is_terminal()), Ordering::Relaxed);

    if let Ok(mut guard) = TERMINAL.lock() {
        *guard = Some(Arc::new(terminal));
    }
}

pub fn echo(time: SystemTime, level: log::Level, target: &str, message: &str) {
    COLOR_DECIDED.call_once(|| {
        COLOR.store(
            wants_color(std::io::stderr().is_terminal()),
            Ordering::Relaxed,
        );
    });

    let style = if COLOR.load(Ordering::Relaxed) {
        Style::Color
    } else {
        Style::Plain
    };

    let line = format_record(time, level, target, message, style);

    // The lock only picks the target: a slow terminal must not stall other threads.
    let terminal = TERMINAL.lock().ok().and_then(|guard| guard.clone());

    // Nothing sensible to do when the terminal is gone.
    terminal
        .map_or_else(
            || std::io::stderr().write_all(line.as_bytes()),
            |file| (&*file).write_all(line.as_bytes()),
        )
        .ok();
}

fn wants_color(is_terminal: bool) -> bool {
    is_terminal && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}

#[cfg(test)]
mod tests;
