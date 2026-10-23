use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[cfg(target_os = "linux")]
pub(super) fn open_master() -> std::io::Result<std::os::fd::OwnedFd> {
    use rustix::pty::{OpenptFlags, openpt};

    Ok(openpt(
        OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC,
    )?)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn open_master() -> std::io::Result<std::os::fd::OwnedFd> {
    use rustix::pty::{OpenptFlags, openpt};

    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    rustix::io::fcntl_setfd(&master, rustix::io::FdFlags::CLOEXEC)?;
    Ok(master)
}

pub(super) fn stage_error(stage: &str, err: impl Into<std::io::Error>) -> std::io::Error {
    let err = err.into();
    std::io::Error::new(err.kind(), format!("{stage}: {err}"))
}

pub(super) fn spawn_in_pty(
    program: &OsStr,
    args: &[OsString],
    workdir: &Path,
    size: rustix::termios::Winsize,
) -> std::io::Result<(Child, File)> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::process::CommandExt as _;

    use rustix::pty::{grantpt, ptsname, unlockpt};

    if !workdir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("the working directory {} does not exist", workdir.display()),
        ));
    }

    let master = open_master().map_err(|err| stage_error("open a terminal master", err))?;
    grantpt(&master).map_err(|err| stage_error("grant a terminal", err))?;
    unlockpt(&master).map_err(|err| stage_error("unlock a terminal", err))?;
    let name = ptsname(&master, Vec::new()).map_err(|err| stage_error("name a terminal", err))?;
    let slave = File::options()
        .read(true)
        .write(true)
        .open(Path::new(OsStr::from_bytes(name.to_bytes())))
        .map_err(|err| stage_error("open a terminal slave", err))?;

    rustix::termios::tcsetwinsize(&slave, size)
        .map_err(|err| stage_error("size a terminal", err))?;

    let mut command = Command::new(program);
    command
        .args(args)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave))
        .current_dir(workdir);

    #[expect(
        unsafe_code,
        reason = "pre_exec in the child between fork and exec: only async-signal-safe calls"
    )]
    // SAFETY: only async-signal-safe syscalls run between fork and exec:
    // a new session, then the pseudo-terminal (stdin) as its controlling tty.
    unsafe {
        command.pre_exec(|| {
            rustix::process::setsid()?;
            rustix::process::ioctl_tiocsctty(rustix::stdio::stdin())?;
            Ok(())
        });
    }

    let child = command.spawn()?;
    Ok((child, File::from(master)))
}
