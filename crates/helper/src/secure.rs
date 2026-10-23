//! Root writes walk from `/` with `O_NOFOLLOW` so a directory swapped for a symlink cannot redirect them.

use std::fs::File;
use std::io::{self, Write as _};
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use drakeflake_plan::ValidatedPlan;
use rustix::fs::{AtFlags, Mode, OFlags};
use rustix::io::Errno;

const DIR_FLAGS: OFlags = OFlags::DIRECTORY
    .union(OFlags::RDONLY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// 0644 for unprivileged `nix` evaluation, from flags because the raw mode type is narrower on macOS.
const FILE_MODE: Mode = Mode::RUSR
    .union(Mode::WUSR)
    .union(Mode::RGRP)
    .union(Mode::ROTH);
const DIR_MODE: Mode = Mode::RWXU
    .union(Mode::RGRP)
    .union(Mode::XGRP)
    .union(Mode::ROTH)
    .union(Mode::XOTH);

pub struct Staged {
    pub dir: OwnedFd,
    pub name: std::ffi::OsString,
    pub temp: String,
}

fn invalid(path: &Path, what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{}: {what}", path.display()),
    )
}

fn split(path: &Path) -> io::Result<(&Path, &std::ffi::OsStr)> {
    match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) if path.is_absolute() => Ok((dir, name)),
        _ => Err(invalid(path, "not an absolute file path")),
    }
}

/// Resolved once so a flake behind a symlink works while the writes themselves never resolve again.
pub fn canonical(path: &Path) -> io::Result<PathBuf> {
    // A symlink as the file itself must stay visible to the checks and to `O_NOFOLLOW`.
    let (dir, name) = split(path)?;
    let existing = dir
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| invalid(path, "no existing ancestor"))?;

    let missing = dir
        .strip_prefix(existing)
        .map_err(|err| invalid(path, &format!("cannot split the path ({err})")))?;

    Ok(existing.canonicalize()?.join(missing).join(name))
}

fn open_dir(dir: &Path, create: bool) -> io::Result<OwnedFd> {
    let mut fd = rustix::fs::open("/", DIR_FLAGS, Mode::empty())?;

    for part in dir.components() {
        let name = match part {
            Component::RootDir => continue,
            Component::Normal(name) => name,
            Component::Prefix(_) | Component::CurDir | Component::ParentDir => {
                return Err(invalid(dir, "unexpected path component"));
            }
        };

        fd = match rustix::fs::openat(&fd, name, DIR_FLAGS, Mode::empty()) {
            Err(Errno::NOENT) if create => {
                rustix::fs::mkdirat(&fd, name, DIR_MODE)?;
                rustix::fs::openat(&fd, name, DIR_FLAGS, Mode::empty())?
            }
            other => other?,
        };
    }

    Ok(fd)
}

pub fn temp_name(name: &std::ffi::OsStr) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |age| age.as_nanos());

    temp_name_at(name, nanos)
}

/// The serial keeps two writes that read the same clock value apart.
pub fn temp_name_at(name: &std::ffi::OsStr, nanos: u128) -> String {
    format!(
        ".{}.tmp-{}-{nanos}-{}",
        name.to_string_lossy(),
        std::process::id(),
        drakeflake_plan::next_serial()
    )
}

pub fn write_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let (dir_path, name) = split(path)?;
    let dir = open_dir(dir_path, true)?;
    let temp = stage(&dir, name, content)?;

    rustix::fs::renameat(&dir, temp.as_str(), &dir, name)
        .map_err(io::Error::from)
        .inspect_err(|_| {
            // Best effort: the error being reported is the write's.
            rustix::fs::unlinkat(&dir, temp.as_str(), AtFlags::empty()).ok();
        })
}

fn stage(dir: &OwnedFd, name: &std::ffi::OsStr, content: &[u8]) -> io::Result<String> {
    let temp = temp_name(name);

    let fd = rustix::fs::openat(
        dir,
        temp.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        FILE_MODE,
    )?;

    let written = rustix::fs::fchmod(&fd, FILE_MODE)
        .map_err(io::Error::from)
        .and_then(|()| {
            let mut file = File::from(fd);
            file.write_all(content)?;
            file.sync_all()
        });

    match written {
        Ok(()) => Ok(temp),
        Err(err) => {
            rustix::fs::unlinkat(dir, temp.as_str(), AtFlags::empty()).ok();
            Err(err)
        }
    }
}

/// Every file is staged and flushed before the first swap, so a failure up to then leaves the files as they were.
pub fn apply_plan(plan: &ValidatedPlan) -> io::Result<()> {
    let writes = plan.ordered_writes();
    let mut staged = Vec::with_capacity(writes.len());

    for (path, content) in writes {
        match stage_path(path, content.as_bytes()) {
            Ok(file) => staged.push(file),
            Err(err) => {
                discard(&staged);
                return Err(err);
            }
        }
    }

    swap_all(&staged)?;

    plan.removals()
        .iter()
        .try_for_each(|path| remove_file(path))
}

pub fn stage_path(path: &Path, content: &[u8]) -> io::Result<Staged> {
    let (dir_path, name) = split(path)?;
    let dir = open_dir(dir_path, true)?;
    let temp = stage(&dir, name, content)?;

    Ok(Staged {
        dir,
        name: name.to_owned(),
        temp,
    })
}

pub fn swap_all(staged: &[Staged]) -> io::Result<()> {
    let mut done: Vec<Option<String>> = Vec::with_capacity(staged.len());

    for file in staged {
        match swap(file) {
            Ok(backup) => done.push(backup),
            Err(err) => {
                restore(staged, &done);
                discard(staged);
                return Err(err);
            }
        }
    }

    for (file, backup) in staged.iter().zip(&done) {
        if let Some(kept) = backup {
            rustix::fs::unlinkat(&file.dir, kept.as_str(), AtFlags::empty()).ok();
        }
    }

    Ok(())
}

fn swap(file: &Staged) -> io::Result<Option<String>> {
    let backup = temp_name(&file.name);

    // A hard link keeps the target in place until the rename replaces it.
    let kept = match rustix::fs::linkat(
        &file.dir,
        file.name.as_os_str(),
        &file.dir,
        backup.as_str(),
        AtFlags::empty(),
    ) {
        Ok(()) => true,
        Err(Errno::NOENT) => false,
        Err(_) => {
            match rustix::fs::renameat(&file.dir, file.name.as_os_str(), &file.dir, backup.as_str())
            {
                Ok(()) => true,
                Err(Errno::NOENT) => false,
                Err(err) => return Err(err.into()),
            }
        }
    };

    match rustix::fs::renameat(
        &file.dir,
        file.temp.as_str(),
        &file.dir,
        file.name.as_os_str(),
    ) {
        Ok(()) => Ok(kept.then_some(backup)),
        Err(err) => {
            if kept {
                rustix::fs::renameat(&file.dir, backup.as_str(), &file.dir, file.name.as_os_str())
                    .ok();

                // Renaming one hard link onto the other leaves both in place.
                rustix::fs::unlinkat(&file.dir, backup.as_str(), AtFlags::empty()).ok();
            }

            Err(err.into())
        }
    }
}

/// Best effort: the error being reported is the swap's.
fn restore(staged: &[Staged], done: &[Option<String>]) {
    for (file, backup) in staged.iter().zip(done).rev() {
        match backup {
            Some(kept) => {
                rustix::fs::renameat(&file.dir, kept.as_str(), &file.dir, file.name.as_os_str())
                    .ok();
            }
            None => {
                rustix::fs::unlinkat(&file.dir, file.name.as_os_str(), AtFlags::empty()).ok();
            }
        }
    }
}

fn discard(staged: &[Staged]) {
    for file in staged {
        rustix::fs::unlinkat(&file.dir, file.temp.as_str(), AtFlags::empty()).ok();
    }
}

pub fn remove_file(path: &Path) -> io::Result<()> {
    let (dir_path, name) = split(path)?;
    let dir = open_dir(dir_path, false)?;

    match rustix::fs::unlinkat(&dir, name, AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => {}
        Err(err) => return Err(err.into()),
    }

    if let Ok((parent, dir_name)) = split(dir_path)
        && let Ok(parent) = open_dir(parent, false)
    {
        rustix::fs::unlinkat(&parent, dir_name, AtFlags::REMOVEDIR).ok();
    }

    Ok(())
}

pub fn read_user_file(path: &Path, limit: u64, owner: Option<u32>) -> io::Result<Vec<u8>> {
    let (dir_path, name) = split(path)?;
    let dir = open_dir(dir_path, false)?;
    let fd = rustix::fs::openat(
        &dir,
        name,
        // Neither a FIFO nor a device may block the open or become the
        // controlling terminal. The `fstat` below refuses them.
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY,
        Mode::empty(),
    )?;

    let stat = rustix::fs::fstat(&fd)?;

    // The helper must not copy out a secret that only root or another user can read.
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::RegularFile
        || stat.st_uid == 0
        || owner.is_some_and(|uid| uid != stat.st_uid)
        || u64::try_from(stat.st_size).map_or(true, |size| size > limit)
    {
        return Err(invalid(
            path,
            "not a regular file of the user within the size limit",
        ));
    }

    let mut content = Vec::new();
    io::Read::read_to_end(&mut io::Read::take(File::from(fd), limit), &mut content)?;
    Ok(content)
}
