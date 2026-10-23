use std::io::Write as _;
use std::path::PathBuf;

use crate::hub::Cache;

#[must_use = "the path is only computed, nothing is written there"]
pub fn crash_log_path() -> Option<PathBuf> {
    Cache::root().ok().map(|root| root.join("crash.log"))
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        log::error!(target: "panic", "{info}");
        let Some(path) = crash_log_path() else {
            return;
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let thread = std::thread::current();
            let since_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();

            writeln!(
                file,
                "[{}.{:09}] panic in thread {}: {info}\n{}",
                since_epoch.as_secs(),
                since_epoch.subsec_nanos(),
                thread.name().unwrap_or("?"),
                std::backtrace::Backtrace::force_capture()
            )
            .ok();
        }

        default_hook(info);
    }));
}
