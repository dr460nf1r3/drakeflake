use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::STDERR_TARGET;
use crate::format::{Style, format_record};
use crate::terminal::echo;

const MAX_SIZE: u64 = 5 * 1024 * 1024;
/// After a failed rotation the file grows this much before the next try.
const RETRY_BYTES: u64 = 256 * 1024;
const KEEP: u32 = 3;

pub struct FileLogger {
    path: PathBuf,
    limit: u64,
    sink: Mutex<Option<Sink>>,
    failure: Option<String>,
}

struct Sink {
    file: File,
    size: u64,
    rotate_at: u64,
}

impl FileLogger {
    pub fn open(path: PathBuf) -> Self {
        Self::open_with_limit(path, MAX_SIZE)
    }

    pub const fn unavailable(reason: String) -> Self {
        Self {
            path: PathBuf::new(),
            limit: MAX_SIZE,
            sink: Mutex::new(None),
            failure: Some(reason),
        }
    }

    fn open_with_limit(path: PathBuf, limit: u64) -> Self {
        let (sink, failure) = match open_sink(&path, limit) {
            Ok(sink) => (Some(sink), None),
            Err(err) => (None, Some(format!("cannot open {}: {err}", path.display()))),
        };

        Self {
            path,
            limit,
            sink: Mutex::new(sink),
            failure,
        }
    }

    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    fn write_line(&self, line: &str) {
        let Ok(mut guard) = self.sink.lock() else {
            return;
        };

        if let Some(sink) = guard.as_mut()
            && sink.size >= sink.rotate_at
        {
            // The open file is renamed along with its path. On failure the
            // lines keep going to it and the next try comes later.
            match rotate(&self.path).and_then(|()| open_sink(&self.path, self.limit)) {
                Ok(fresh) => *sink = fresh,
                Err(_) => sink.rotate_at = sink.size.saturating_add(RETRY_BYTES),
            }
        }

        if let Some(sink) = guard.as_mut()
            && sink.file.write_all(line.as_bytes()).is_ok()
        {
            sink.size = sink
                .size
                .saturating_add(u64::try_from(line.len()).unwrap_or(u64::MAX));
        }
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let (time, message) = (SystemTime::now(), record.args().to_string());
        let (level, target) = (record.level(), record.target());

        self.write_line(&format_record(time, level, target, &message, Style::Plain));
        if target != STDERR_TARGET {
            echo(time, level, target, &message);
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.sink.lock()
            && let Some(sink) = guard.as_mut()
        {
            // A logger has nowhere to report its own failures.
            sink.file.flush().ok();
        }
    }
}

/// Logs can hold paths and URLs, so only the user may read them.
fn open_sink(path: &Path, limit: u64) -> std::io::Result<Sink> {
    if let Some(dir) = path.parent() {
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;

    let size = file.metadata().map_or(0, |meta| meta.len());

    Ok(Sink {
        file,
        size,
        rotate_at: limit,
    })
}

pub fn rotate(path: &Path) -> std::io::Result<()> {
    let numbered = |n: u32| {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{n}"));
        PathBuf::from(name)
    };

    // Older files are missing until the history fills up, and a logger has
    // nowhere to report its own failures.
    for n in (1..KEEP).rev() {
        fs::rename(numbered(n), numbered(n + 1)).ok();
    }

    fs::rename(path, numbered(1))
}

#[cfg(test)]
mod tests;
