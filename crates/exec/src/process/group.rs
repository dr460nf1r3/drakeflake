use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal};

pub const TERM_GRACE: Duration = Duration::from_secs(2);
const EXIT_POLL: Duration = Duration::from_millis(10);

pub fn own_group(command: &mut Command) {
    command.process_group(0);
}

pub fn stop(child: &mut Child, program: &str) {
    signal_group(child, Signal::TERM, program);

    let deadline = Instant::now() + TERM_GRACE;

    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(None) => std::thread::sleep(EXIT_POLL),
            Ok(Some(_)) | Err(_) => break,
        }
    }

    // Also after a clean exit: descendants may have ignored SIGTERM.
    signal_group(child, Signal::KILL, program);

    if let Err(err) = child.wait() {
        log::warn!("reaping {program} after stopping it failed: {err}");
    }
}

fn signal_group(child: &Child, signal: Signal, program: &str) {
    let Some(group) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) else {
        log::debug!("{program} has no usable process id");
        return;
    };

    // ESRCH just means the group is already gone.
    if let Err(err) = rustix::process::kill_process_group(group, signal)
        && err != rustix::io::Errno::SRCH
    {
        log::debug!("signalling the group of {program} failed: {err}");
    }
}

/// Evaluations must not outlive the app and hold gigabytes.
/// Cleared for setuid programs (pkexec), whose writes are not interrupted.
#[cfg(target_os = "linux")]
pub fn kill_with_parent(command: &mut Command) {
    #[expect(
        unsafe_code,
        reason = "pre_exec in the child between fork and exec: only async-signal-safe calls"
    )]
    // SAFETY: prctl(PR_SET_PDEATHSIG) is async-signal-safe and touches no
    // memory of the parent. Nothing else runs between fork and exec.
    unsafe {
        command.pre_exec(|| {
            rustix::process::set_parent_process_death_signal(Some(Signal::KILL))
                .map_err(std::io::Error::from)
        });
    }
}

/// macOS has no parent death signal, so children end by timeout or cancel.
#[cfg(not(target_os = "linux"))]
pub const fn kill_with_parent(_command: &mut Command) {}
