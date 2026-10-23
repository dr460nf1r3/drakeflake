#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use qtbridge::qobject;
use serde::Serialize;
use serde_json::{Value, json};

use crate::hub::portal::{Outcome, Pick, Picked};
use crate::hub::setup::ImportCheck;
use crate::hub::{Hub, SetupMsg};

#[derive(Default)]
enum FlakeScan {
    #[default]
    NotStarted,
    Running,
    Done(Vec<String>),
}

#[derive(Default)]
enum ImportState {
    #[default]
    Unknown,
    Checking,
    Checked(ImportCheck),
}

impl fmt::Display for ImportState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unknown => "",
            Self::Checking => "checking",
            Self::Checked(ImportCheck::Imported) => "imported",
            Self::Checked(ImportCheck::Missing) => "missing",
            Self::Checked(ImportCheck::Failed(_)) => "failed",
        })
    }
}

#[derive(Serialize)]
struct PickedRow {
    serial: u64,
    purpose: String,
    path: Option<String>,
    error: Option<String>,
}

#[derive(Default)]
pub struct SetupModel {
    hub: Arc<Hub>,
    needed: bool,
    flake_scan: FlakeScan,
    import: ImportState,
    picked: Value,
    picked_serial: u64,
}

impl SetupModel {
    pub fn new(hub: Arc<Hub>, needed: bool) -> Self {
        Self {
            hub,
            needed,
            ..Self::default()
        }
    }

    fn handle(&mut self, message: SetupMsg) {
        match message {
            SetupMsg::ImportCheck(result) => {
                self.import = ImportState::Checked(result);
                log::debug!("import check: {}", self.import);
            }
            SetupMsg::Picked(picked) => self.on_picked(picked),
            SetupMsg::FlakeCandidates(found) => {
                log::debug!("flake scan done: {} found", found.len());
                self.flake_scan = FlakeScan::Done(
                    found
                        .iter()
                        .map(|dir| dir.to_string_lossy().into_owned())
                        .collect(),
                );
            }
            SetupMsg::TargetChanged => self.import = ImportState::Unknown,
        }
    }

    fn on_picked(&mut self, picked: Picked) {
        self.picked_serial += 1;
        self.picked = picked_value(self.picked_serial, picked);
    }
}

#[qobject]
impl SetupModel {
    qproperty!("needs_setup", Read = needs_setup, Notify = state_changed);
    qproperty!(
        "flake_candidates",
        Read = flake_candidates,
        Notify = state_changed
    );
    qproperty!(
        "flake_scanning",
        Read = flake_scanning,
        Notify = state_changed
    );
    qproperty!(
        "import_status",
        Read = import_status,
        Notify = state_changed
    );
    qproperty!("import_error", Read = import_error, Notify = state_changed);
    qproperty!("picked", Read = picked, Notify = state_changed);

    pub const fn needs_setup(&self) -> bool {
        self.needed
    }

    pub fn flake_candidates(&self) -> Value {
        match &self.flake_scan {
            FlakeScan::Done(found) => json!(found),
            FlakeScan::NotStarted | FlakeScan::Running => json!([]),
        }
    }

    pub const fn flake_scanning(&self) -> bool {
        matches!(self.flake_scan, FlakeScan::Running)
    }

    pub fn import_status(&self) -> String {
        self.import.to_string()
    }

    pub fn import_error(&self) -> String {
        match &self.import {
            ImportState::Checked(ImportCheck::Failed(error)) => error.clone(),
            ImportState::Unknown | ImportState::Checking | ImportState::Checked(_) => String::new(),
        }
    }

    pub fn picked(&self) -> Value {
        self.picked.clone()
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().setup.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn scan_flakes(&mut self) {
        if !matches!(self.flake_scan, FlakeScan::NotStarted) {
            return;
        }

        log::debug!("scanning for system flakes");
        self.flake_scan = FlakeScan::Running;
        self.hub.scan_flakes();
        self.state_changed();
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn flake_root(&self, path: String) -> String {
        let dir = Path::new(path.trim());

        if !dir.is_absolute() {
            return String::new();
        }

        drakeflake_plan::find_flake_root(dir)
            .map(|root| root.canonicalize().unwrap_or(root))
            .map(|root| root.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn normalize_folder(&self, text: String) -> String {
        normalize_folder(&text).to_owned()
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn effective_folder(&self, folder: String, flake_root: String) -> String {
        effective_folder(&folder, &flake_root).to_owned()
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn folder_in_flake(&self, folder: String, flake: String) -> bool {
        let (folder, flake) = (Path::new(folder.trim()), Path::new(flake.trim()));

        folder.is_absolute()
            && flake.is_absolute()
            && resolve_existing(folder).starts_with(resolve_existing(flake))
    }

    #[qslot]
    pub fn check_import(&mut self) {
        log::info!("checking the import of the generated file");
        self.import = ImportState::Checking;
        self.hub.check_import();
        self.state_changed();
    }

    #[qslot]
    pub fn import_line(&self) -> String {
        let dir = self
            .hub
            .target()
            .and_then(|target| drakeflake_flake::local_flake_dir(target.flake.trim()));

        let (Ok(entry), Some(dir)) = (self.hub.managed_file(), dir) else {
            return String::new();
        };

        crate::hub::setup::import_line(&entry, &dir).unwrap_or_default()
    }

    #[qslot]
    pub fn pick_folder(&self, purpose: String, title: String, start: String) {
        self.hub
            .pick(purpose, title, Pick::Folder, PathBuf::from(start.trim()));
    }

    #[qslot]
    pub fn pick_save_file(&self, purpose: String, title: String, start: String, name: String) {
        self.hub.pick(
            purpose,
            title,
            Pick::SaveFile(name),
            PathBuf::from(start.trim()),
        );
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn folder_exists(&self, path: String) -> bool {
        Path::new(path.trim()).is_dir()
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn create_folder(&self, path: String) -> String {
        let dir = Path::new(path.trim());

        if !dir.is_absolute() {
            return format!("\"{}\" is not an absolute path.", dir.display());
        }

        match std::fs::create_dir_all(dir) {
            Ok(()) => {
                log::info!("created folder {}", dir.display());
                String::new()
            }
            Err(error) => {
                log::error!("cannot create {}: {error}", dir.display());
                format!("Cannot create {}: {error}", dir.display())
            }
        }
    }

    #[qslot]
    pub fn finish_setup(&mut self) {
        log::info!("setup finished");
        self.needed = false;
        self.state_changed();
    }
}

fn picked_value(serial: u64, picked: Picked) -> Value {
    let (path, error) = match picked.outcome {
        Outcome::Chosen(path) => (Some(path.to_string_lossy().into_owned()), None),
        Outcome::Cancelled => (None, None),
        Outcome::Unavailable(reason) => (None, Some(reason)),
    };

    json!(PickedRow {
        serial,
        purpose: picked.purpose,
        path,
        error,
    })
}

fn normalize_folder(text: &str) -> &str {
    text.trim().trim_end_matches('/')
}

fn effective_folder<'a>(folder: &'a str, flake_root: &'a str) -> &'a str {
    if folder.trim().is_empty() {
        flake_root
    } else {
        normalize_folder(folder)
    }
}

fn resolve_existing(path: &Path) -> PathBuf {
    path.ancestors()
        .find_map(|ancestor| {
            let real = ancestor.canonicalize().ok()?;
            let rest = path.strip_prefix(ancestor).ok()?;
            Some(real.join(rest))
        })
        .unwrap_or_else(|| path.to_path_buf())
}
