//! One polkit action per subcommand, matched on argv[1], and the helper never returns pkexec's own codes 126 and 127.

mod activate;
mod apply;
mod error;
mod exec;
mod lock;
#[cfg(target_os = "linux")]
mod scan;
mod secure;

#[cfg(test)]
mod tests;

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use error::HelperError;

const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024;
const SYSTEM_PROFILE: &str = "/nix/var/nix/profiles/system";
/// pkexec resets `PATH`. NixOS tools live here.
const SYSTEM_PATH: &str = "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin";

const USAGE: &str = "usage: drakeflake-helper apply | activate <switch|boot|test> <store-path> | install-lock <flake-dir> <lock-file> | scan-ephemeral [<btrfs-subvolume>]";

fn main() -> ExitCode {
    #[expect(
        unsafe_code,
        reason = "PATH set as the first statement, before any other thread exists"
    )]
    // SAFETY: first statement of `main`, so no other thread can read the environment.
    unsafe {
        std::env::set_var("PATH", SYSTEM_PATH);
    };

    // pkexec keeps the caller's umask, and root's files must never be writable by others.
    rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o022));

    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let result = match refs.as_slice() {
        ["apply"] => apply::apply(),
        ["activate", mode, system] => activate::activate(mode, Path::new(system)),
        ["install-lock", dir, lock_file] => {
            lock::install_lock(Path::new(dir), Path::new(lock_file), lock::caller_uid())
        }
        #[cfg(target_os = "linux")]
        ["scan-ephemeral"] => scan::scan_ephemeral(None),
        #[cfg(target_os = "linux")]
        ["scan-ephemeral", subvolume] => scan::scan_ephemeral(Some(subvolume)),
        #[cfg(not(target_os = "linux"))]
        ["scan-ephemeral", ..] => Err(HelperError::ScanFailed(
            "only a Linux root is wiped on reboot".to_owned(),
        )),
        _ => Err(HelperError::Usage),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            note(&error.to_string());
            ExitCode::from(error.exit_code())
        }
    }
}

fn note(message: &str) {
    // Nothing else can report a broken stderr.
    writeln!(std::io::stderr(), "drakeflake-helper: {message}").ok();
}
