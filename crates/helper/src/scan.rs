use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use drakeflake_scan::{ScanLimits, ScanNode, mount_points, mount_source, scan};
use rustix::mount::{MountFlags, MountPropagationFlags, UnmountFlags};

use crate::error::HelperError;

/// Under `/run` because the scan skips it. The suffix is random so a
/// pre-planted directory cannot make `mkdir` fail or hijack the mount point.
const TOP_LEVEL: &str = "/run/drakeflake-scan-";

fn random_suffix() -> String {
    if let Ok(mut file) = fs::File::open("/dev/urandom") {
        let mut bytes = [0_u8; 8];
        if std::io::Read::read_exact(&mut file, &mut bytes).is_ok() {
            return bytes
                .iter()
                .fold(String::new(), |hex, byte| format!("{hex}{byte:02x}"));
        }
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |age| age.as_nanos());

    format!("{:x}", nanos.wrapping_mul(u128::from(std::process::id())))
}

pub fn scan_ephemeral(subvolume: Option<&str>) -> Result<(), HelperError> {
    let node = match subvolume {
        None => scan_root(None),
        Some(name) => {
            let name = checked_subvolume(name)?;
            let top = mount_top_level()?;
            let result = snapshot_in(&top, &name).and_then(|snapshot| scan_root(Some(&snapshot)));

            unmount_top_level(&top);

            result
        }
    }?;

    let json = serde_json::to_vec(&node)
        .map_err(|err| HelperError::ScanFailed(format!("cannot encode the result: {err}")))?;

    std::io::stdout()
        .write_all(&json)
        .map_err(|source| HelperError::ScanIo {
            context: "cannot write the result".to_owned(),
            source,
        })
}

fn scan_root(snapshot: Option<&Path>) -> Result<ScanNode, HelperError> {
    // Without the mount table, persistent bind mounts would be listed as wiped.
    let mounts = mount_points().map_err(HelperError::ScanFailed)?;

    scan(Path::new("/"), snapshot, &mounts, ScanLimits::default()).map_err(HelperError::ScanFailed)
}

pub fn checked_subvolume(name: &str) -> Result<PathBuf, HelperError> {
    let path = Path::new(name.trim_start_matches('/'));
    let plain = !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)));

    if !plain {
        return Err(HelperError::Denied(format!(
            "\"{name}\" is not a subvolume name"
        )));
    }

    Ok(path.components().collect())
}

/// Canonicalized so a symlink cannot lead out.
fn snapshot_in(top: &Path, name: &Path) -> Result<PathBuf, HelperError> {
    let path = top.join(name);
    let resolved = path.canonicalize().map_err(|source| HelperError::Resolve {
        path: path.clone(),
        source,
    })?;

    if !resolved.starts_with(top) || !resolved.is_dir() {
        return Err(HelperError::Denied(format!(
            "{} is not a folder of the btrfs file system",
            name.display()
        )));
    }

    Ok(resolved)
}

fn mount_top_level() -> Result<PathBuf, HelperError> {
    let table =
        fs::read_to_string("/proc/self/mountinfo").map_err(|source| HelperError::ScanIo {
            context: "cannot read the mount table".to_owned(),
            source,
        })?;

    let (kind, device) = mount_source(&table, Path::new("/"))
        .ok_or_else(|| HelperError::ScanFailed("/ is not in the mount table".to_owned()))?;

    if kind != "btrfs" {
        return Err(HelperError::Denied(format!(
            "/ is {kind}, not btrfs: there is no snapshot to compare"
        )));
    }

    #[expect(
        unsafe_code,
        reason = "unsharing the mount namespace of this single-threaded process"
    )]
    // SAFETY: the helper has one thread and unshares only the mount namespace.
    unsafe { rustix::thread::unshare_unsafe(rustix::thread::UnshareFlags::NEWNS) }.map_err(
        |source| HelperError::ScanIo {
            context: "cannot unshare mounts".to_owned(),
            source: std::io::Error::from(source),
        },
    )?;

    rustix::mount::mount_change(
        "/",
        MountPropagationFlags::REC | MountPropagationFlags::PRIVATE,
    )
    .map_err(|source| HelperError::ScanIo {
        context: "cannot make mounts private".to_owned(),
        source: std::io::Error::from(source),
    })?;

    let top = PathBuf::from(format!("{TOP_LEVEL}{}", random_suffix()));

    // Exclusive create, since a pre-existing directory means someone planted it and mounting over it is the hijack to refuse.
    rustix::fs::mkdir(&top, rustix::fs::Mode::from_raw_mode(0o700)).map_err(|source| {
        remove_top_level(&top);
        HelperError::ScanIo {
            context: format!("cannot create {}", top.display()),
            source: std::io::Error::from(source),
        }
    })?;

    rustix::mount::mount(
        device.as_str(),
        &top,
        "btrfs",
        MountFlags::RDONLY | MountFlags::NOSUID | MountFlags::NODEV | MountFlags::NOEXEC,
        c"subvolid=5",
    )
    .map_err(|source| {
        remove_top_level(&top);
        HelperError::ScanIo {
            context: format!("cannot mount {device}"),
            source: std::io::Error::from(source),
        }
    })?;

    Ok(top)
}

fn unmount_top_level(top: &Path) {
    if let Err(err) = rustix::mount::unmount(top, UnmountFlags::DETACH) {
        crate::note(&format!("cannot unmount {}: {err}", top.display()));
    }

    remove_top_level(top);
}

fn remove_top_level(top: &Path) {
    if let Err(err) = fs::remove_dir(top) {
        crate::note(&format!("cannot remove {}: {err}", top.display()));
    }
}
