use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use drakeflake_tray_core::watch::{Event, Source, is_skipped};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::inotify::{self, CreateFlags, ReadFlags, Reader, WatchFlags};
use rustix::io::Errno;

/// A runaway tree must not exhaust the inotify limit.
const MAX_FOLDERS: usize = 4096;

const EVENTS: WatchFlags = WatchFlags::CREATE
    .union(WatchFlags::DELETE)
    .union(WatchFlags::MODIFY)
    .union(WatchFlags::CLOSE_WRITE)
    .union(WatchFlags::MOVED_FROM)
    .union(WatchFlags::MOVED_TO)
    .union(WatchFlags::ATTRIB)
    .union(WatchFlags::DELETE_SELF)
    .union(WatchFlags::MOVE_SELF);

enum Drained {
    Nothing,
    Changed,
    RootGone,
}

#[derive(Default)]
struct Tree {
    folders: HashMap<i32, PathBuf>,
    full: bool,
}

impl Tree {
    fn add(&mut self, fd: &impl AsFd, dir: &Path) -> Result<(), String> {
        if self.full {
            return Ok(());
        }

        if self.folders.len() >= MAX_FOLDERS {
            self.full = true;
            drakeflake_log::warn!("watching only the first {MAX_FOLDERS} folders of the flake");
            return Ok(());
        }

        let wd = match inotify::add_watch(fd, dir, EVENTS) {
            Ok(wd) => wd,
            // The folder vanished between its listing and the watch.
            Err(Errno::NOENT) => return Ok(()),
            Err(error) => return Err(format!("cannot watch {}: {error}", dir.display())),
        };

        self.folders.insert(wd, dir.to_owned());

        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };

        for entry in entries.flatten() {
            if self.full {
                break;
            }

            // A symlink can point outside the flake or form a loop, so only real folders count.
            let is_folder = entry.file_type().is_ok_and(|kind| kind.is_dir());

            if is_folder && !is_skipped(&entry.file_name()) {
                self.add(fd, &entry.path())?;
            }
        }

        Ok(())
    }
}

pub struct Inotify {
    fd: OwnedFd,
    tree: Tree,
    buffer: [MaybeUninit<u8>; 8192],
    root: PathBuf,
}

impl Source for Inotify {
    fn wait(&mut self, timeout: Duration) -> Result<Event, String> {
        let timeout = Timespec::try_from(timeout).map_err(|error| error.to_string())?;
        let mut fds = [PollFd::new(&self.fd, PollFlags::IN)];

        match poll(&mut fds, Some(&timeout)) {
            Ok(_) => {}
            Err(Errno::INTR) => return Ok(Event::Quiet),
            Err(error) => return Err(format!("poll failed: {error}")),
        }

        let readable = fds
            .iter()
            .any(|polled| polled.revents().contains(PollFlags::IN));

        if !readable {
            return Ok(Event::Quiet);
        }

        Ok(
            match drain(&self.fd, &mut self.buffer, &self.root, &mut self.tree)? {
                Drained::Nothing => Event::Quiet,
                Drained::Changed => Event::Changed,
                Drained::RootGone => Event::RootGone,
            },
        )
    }
}

pub fn open(folder: &Path) -> Result<Inotify, String> {
    let fd = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)
        .map_err(|error| format!("inotify is unavailable: {error}"))?;

    let mut tree = Tree::default();
    tree.add(&fd, folder)?;

    if tree.folders.is_empty() {
        return Err(format!("{} does not exist", folder.display()));
    }

    Ok(Inotify {
        fd,
        tree,
        buffer: [MaybeUninit::uninit(); 8192],
        root: folder.to_owned(),
    })
}

fn drain(
    fd: &impl AsFd,
    buffer: &mut [MaybeUninit<u8>],
    root: &Path,
    tree: &mut Tree,
) -> Result<Drained, String> {
    let mut reader = Reader::new(fd, buffer);
    let mut drained = Drained::Nothing;

    loop {
        let event = match reader.next() {
            Ok(event) => event,
            Err(Errno::AGAIN) => return Ok(drained),
            Err(Errno::INTR) => continue,
            Err(error) => return Err(format!("reading inotify events failed: {error}")),
        };

        drained = Drained::Changed;
        let flags = event.events();
        let wd = event.wd();

        if flags.contains(ReadFlags::IGNORED) {
            tree.folders.remove(&wd);
            continue;
        }

        let is_root = tree.folders.get(&wd).is_some_and(|path| path == root);

        if is_root && flags.intersects(ReadFlags::DELETE_SELF | ReadFlags::MOVE_SELF) {
            return Ok(Drained::RootGone);
        }

        let created = flags.contains(ReadFlags::ISDIR)
            && flags.intersects(ReadFlags::CREATE | ReadFlags::MOVED_TO);

        let folder = event
            .file_name()
            .map(|name| OsStr::from_bytes(name.to_bytes()))
            .filter(|name| created && !is_skipped(name))
            .and_then(|name| tree.folders.get(&wd).map(|parent| parent.join(name)));

        if let Some(path) = folder {
            tree.add(fd, &path)?;
        }
    }
}

#[cfg(test)]
mod tests;
