use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use drakeflake_exec::Context;
use drakeflake_flake::Persistence;
use drakeflake_scan::{ScanLimits, ScanNode};

use super::argv::arg;
use super::{Hub, helper_path};

const SCAN_TIMEOUT: Duration = Duration::from_secs(600);

const MAX_SCAN_JSON: usize = 32 * 1024 * 1024;

pub enum PersistenceMsg {
    Loaded(Result<Option<Persistence>, String>),
    Scanned(Result<Scan, String>),
}

pub struct Scan {
    pub tree: ScanNode,
    pub file_system: Option<String>,
    pub snapshot: bool,
}

struct TestRoot {
    root: PathBuf,
    snapshot: Option<PathBuf>,
}

impl TestRoot {
    fn from_env() -> Option<Self> {
        let root = std::env::var_os("DRAKEFLAKE_PERSIST_ROOT")?;

        Some(Self {
            root: PathBuf::from(root),
            snapshot: std::env::var_os("DRAKEFLAKE_PERSIST_SNAPSHOT").map(PathBuf::from),
        })
    }

    fn scan(&self, compare: bool) -> Result<Scan, String> {
        let snapshot = self.snapshot.as_deref().filter(|_| compare);
        let tree = drakeflake_scan::scan(&self.root, snapshot, &[], ScanLimits::default())?;

        Ok(Scan {
            tree,
            file_system: Some("test".to_owned()),
            snapshot: snapshot.is_some(),
        })
    }
}

pub fn load_persistence(owner: &Arc<Hub>) {
    owner.spawn_background("persistence", |hub, nix| {
        let result = hub
            .target()
            .ok_or_else(|| "No system is selected yet".to_owned())
            .and_then(|target| {
                drakeflake_evaluate::persistence_current(nix, &target)
                    .map_err(|error| error.to_string())
            });

        if let Err(error) = &result {
            log::warn!("persisted entries: {error}");
        }

        hub.mail.persistence.post(PersistenceMsg::Loaded(result));
    });
}

pub fn scan_ephemeral(owner: &Arc<Hub>, subvolume: Option<String>) {
    owner.spawn_background("scan-ephemeral", move |hub, nix| {
        let result = scan(nix.context(), subvolume.as_deref());

        match &result {
            Ok(scan) => log::info!(
                "scanned {} ephemeral files on {} ({} bytes, {} top-level entries, {} hidden)",
                scan.tree.files,
                scan.file_system
                    .as_deref()
                    .unwrap_or("an unknown file system"),
                scan.tree.size,
                scan.tree.children.len(),
                scan.tree.hidden
            ),
            Err(error) => log::warn!("scan of the root file system: {error}"),
        }

        hub.mail.persistence.post(PersistenceMsg::Scanned(result));
    });
}

pub fn snapshot_offered() -> bool {
    TestRoot::from_env().map_or_else(
        || offers_snapshot(drakeflake_scan::root_file_system().as_deref()),
        |test| test.snapshot.is_some(),
    )
}

fn offers_snapshot(file_system: Option<&str>) -> bool {
    file_system == Some("btrfs")
}

fn scan(exec: &Context, subvolume: Option<&str>) -> Result<Scan, String> {
    TestRoot::from_env().map_or_else(
        || scan_elevated(exec, subvolume),
        |test| test.scan(subvolume.is_some()),
    )
}

fn scan_elevated(exec: &Context, subvolume: Option<&str>) -> Result<Scan, String> {
    let file_system = drakeflake_scan::root_file_system();
    let compared = subvolume.filter(|_| offers_snapshot(file_system.as_deref()));
    let helper =
        helper_path().ok_or("The privileged helper (drakeflake-helper) is not installed")?;

    let elevation = drakeflake_file::find_elevation(drakeflake_file::Prompt::DialogOnly)
        .ok_or("None of pkexec, run0 or the macOS administrator dialog is available")?;

    let mut command = elevation.prefix();

    command.extend([arg(&helper), arg("scan-ephemeral")]);
    command.extend(compared.map(arg));

    let (program, args) = command.split_first().ok_or("empty elevation command")?;
    let captured = exec
        .command(program, args, SCAN_TIMEOUT)
        .map_err(|error| error.to_string())?;

    if !captured.success {
        return Err(format!(
            "Could not list the files with administrator rights: {}",
            captured.stderr.trim()
        ));
    }

    if captured.stdout.len() > MAX_SCAN_JSON {
        return Err(format!(
            "Unexpected scan result: {} bytes is more than the {} byte limit",
            captured.stdout.len(),
            MAX_SCAN_JSON
        ));
    }

    let tree = serde_json::from_str(&captured.stdout)
        .map_err(|err| format!("Unexpected scan result: {err}"))?;

    Ok(Scan {
        tree,
        file_system,
        snapshot: compared.is_some(),
    })
}

#[cfg(test)]
mod tests;
