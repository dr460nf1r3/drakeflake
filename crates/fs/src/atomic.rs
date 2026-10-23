#[cfg(test)]
mod tests;

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells apart names made within the same nanosecond.
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub fn atomic_write(path: &Path, text: &str) -> io::Result<()> {
    let temp = stage(path, text)?;

    fs::rename(&temp, path).inspect_err(|_| {
        fs::remove_file(&temp).ok();
    })
}

/// A hidden sibling of `path`, so it is never taken for a module.
#[must_use = "only builds a name, nothing is created"]
pub fn sibling(path: &Path, tag: &str) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.{tag}-{}", unique_suffix()))
}

pub fn stage(path: &Path, text: &str) -> io::Result<PathBuf> {
    let temp = sibling(path, "tmp");

    let written = File::options()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });

    match written {
        Ok(()) => Ok(temp),
        Err(err) => {
            fs::remove_file(&temp).ok();
            Err(err)
        }
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |age| age.as_nanos());

    format!(
        "{}-{nanos}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}
