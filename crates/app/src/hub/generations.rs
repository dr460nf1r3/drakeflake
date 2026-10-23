use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use super::argv::arg;
use super::operations::{Exclusive, Outcome, Reporting};
use super::system::System;
use super::{Hub, HubError, Mode, find_program};

pub(super) const PROFILES: &str = "/nix/var/nix/profiles";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Generation {
    pub id: u32,
    pub path: String,
    pub date: Option<u64>,
    pub version: String,
    pub kernel: String,
    pub specialisations: Vec<String>,
    pub current: bool,
    pub running: bool,
    pub running_specialisation: Option<String>,
    pub booted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationAction {
    Compare,
    Switch,
    Test,
    Boot,
}

impl GenerationAction {
    pub const fn mode(self) -> Option<Mode> {
        match self {
            Self::Compare => None,
            Self::Switch => Some(Mode::Switch),
            Self::Test => Some(Mode::Test),
            Self::Boot => Some(Mode::Boot),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAction(pub String);

impl std::fmt::Display for UnknownAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown generation action \"{}\"", self.0)
    }
}

impl std::error::Error for UnknownAction {}

impl FromStr for GenerationAction {
    type Err = UnknownAction;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "compare" => Ok(Self::Compare),
            "switch" => Ok(Self::Switch),
            "test" => Ok(Self::Test),
            "boot" => Ok(Self::Boot),
            _ => Err(UnknownAction(text.to_owned())),
        }
    }
}

struct Pointers {
    current: Option<PathBuf>,
    running: Option<PathBuf>,
    booted: Option<PathBuf>,
}

impl Exclusive {
    fn compare_generation(&self, generation: &Generation) -> Outcome {
        self.console().log(format!(
            "Changes from the running system to generation {}:",
            generation.id
        ));
        self.diff_systems(
            Path::new("/run/current-system"),
            Path::new(&generation.path),
        )
    }

    pub(super) fn diff_systems(&self, current: &Path, target: &Path) -> Outcome {
        let (program, args): (PathBuf, Vec<OsString>) = if let Some(nvd) = find_program("nvd") {
            (nvd, vec![arg("diff"), arg(current), arg(target)])
        } else if let Some(nix) = find_program("nix") {
            (
                nix,
                vec![
                    arg("--extra-experimental-features"),
                    arg("nix-command"),
                    arg("store"),
                    arg("diff-closures"),
                    arg(current),
                    arg(target),
                ],
            )
        } else {
            self.console()
                .error("Neither nvd nor nix is available to compare systems.");
            return Outcome::Failed;
        };

        log::debug!("comparing systems with {}", program.display());
        self.work_dir().map_or(Outcome::Failed, |dir| {
            self.stream_pty(program.as_os_str(), &args, &dir)
        })
    }
}

pub fn act(hub: &Arc<Hub>, id: u32, action: GenerationAction) -> Result<(), HubError> {
    let can = hub.capabilities();
    if !can.generations {
        return Err(HubError::invalid(
            "This machine's generations are not this host's.",
        ));
    }

    if let Some(mode) = action.mode()
        && !can.generation_modes.contains(&mode)
    {
        return Err(HubError::invalid(format!(
            "Generations of this host cannot be activated with {}.",
            mode.verb()
        )));
    }

    let system = hub
        .target()
        .map_or_else(System::native, |target| System::of(target.kind));

    let generation = list_generations(Path::new(PROFILES))
        .into_iter()
        .find(|generation| generation.id == id)
        .ok_or_else(|| HubError::invalid(format!("Generation {id} does not exist")))?;

    log::info!("generation {id}: {action:?} ({})", generation.path);

    hub.spawn_exclusive("generation", Reporting::Shown, move |run| {
        action.mode().map_or_else(
            || run.compare_generation(&generation),
            |mode| system.activate_generation(run, &generation, mode),
        )
    })
}

fn system_version(target: &Path) -> Option<String> {
    ["nixos-version", "darwin-version"]
        .iter()
        .find_map(|file| std::fs::read_to_string(target.join(file)).ok())
        .map(|text| text.trim().to_owned())
}

fn canonical(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok()
}

#[must_use = "only reads the profiles, nothing is activated"]
pub fn list_generations(profiles: &Path) -> Vec<Generation> {
    let pointers = Pointers {
        current: std::fs::read_link(profiles.join("system")).ok(),
        running: canonical(Path::new("/run/current-system")),
        booted: canonical(Path::new("/run/booted-system")),
    };

    generations_in(profiles, &pointers)
}

fn generations_in(profiles: &Path, pointers: &Pointers) -> Vec<Generation> {
    let entries = match std::fs::read_dir(profiles) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!("cannot list generations in {}: {err}", profiles.display());
            return Vec::new();
        }
    };

    let mut generations: Vec<Generation> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| generation(&entry, pointers))
        .collect();

    generations.sort_by_key(|generation| std::cmp::Reverse(generation.id));
    log::debug!(
        "generations: {} listed in {}, current {:?}, running {:?}",
        generations.len(),
        profiles.display(),
        generations
            .iter()
            .find(|generation| generation.current)
            .map(|generation| generation.id),
        generations
            .iter()
            .find(|generation| generation.running)
            .map(|generation| generation.id),
    );
    generations
}

fn generation(entry: &std::fs::DirEntry, pointers: &Pointers) -> Option<Generation> {
    let name = entry.file_name().to_string_lossy().into_owned();
    let id: u32 = name
        .strip_prefix("system-")?
        .strip_suffix("-link")?
        .parse()
        .ok()?;

    let link = entry.path();
    let Some(target) = canonical(&link) else {
        log::debug!("generation {id}: {} is dangling", link.display());
        return None;
    };

    let specialisations = specialisations(&target);
    let running_specialisation = pointers.running.as_ref().and_then(|running| {
        specialisations
            .iter()
            .find(|special| {
                canonical(&target.join("specialisation").join(special)).as_ref() == Some(running)
            })
            .cloned()
    });

    Some(Generation {
        id,
        date: std::fs::symlink_metadata(&link)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|age| age.as_secs()),
        version: system_version(&target).unwrap_or_else(|| {
            log::debug!("generation {id}: no nixos-version or darwin-version");
            String::new()
        }),
        kernel: kernel_name(&target),
        current: pointers.current.as_deref().and_then(Path::file_name)
            == Some(std::ffi::OsStr::new(&name)),
        running: pointers.running.as_ref() == Some(&target) || running_specialisation.is_some(),
        running_specialisation,
        specialisations,
        booted: pointers.booted.as_ref() == Some(&target),
        path: target.to_string_lossy().into_owned(),
    })
}

fn specialisations(system: &Path) -> Vec<String> {
    std::fs::read_dir(system.join("specialisation"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|special| special.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn kernel_name(system: &Path) -> String {
    std::fs::read_link(system.join("kernel"))
        .ok()
        .and_then(|kernel| {
            let package = kernel.parent()?.file_name()?.to_string_lossy().into_owned();
            let (_, name) = package.split_once('-')?;
            let version = name
                .split('-')
                .skip_while(|part| !part.starts_with(|ch: char| ch.is_ascii_digit()))
                .collect::<Vec<_>>()
                .join("-");

            Some(if version.is_empty() {
                name.to_owned()
            } else {
                format!("Linux {version}")
            })
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
