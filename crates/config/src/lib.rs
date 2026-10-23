mod blur;
mod layout;
mod load;
mod save;

use std::path::PathBuf;

use anyhow::Result;
pub use blur::{BlurMode, UnknownBlurMode, blur_env_override, effective_blur, wayland_session};
use drakeflake_fs::DirsError;
use drakeflake_render::HostName;
pub use layout::{LayoutConfig, LayoutMode, LayoutRule, UnknownLayoutMode};
pub use load::{load, load_from, load_with_notes};
pub use save::save_to;
use serde::{Deserialize, Serialize};

pub const DEFAULT_FLAKE_PATH: &str = "/etc/nixos";
pub const DEFAULT_DARWIN_FLAKE_PATH: &str = "/etc/nix-darwin";

const FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub nix: Nix,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub tray: Tray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    Updates,
    Restart,
    FailedServices,
    LowSpace,
    Staged,
    WatchFlake,
}

impl Report {
    pub const ALL: [Self; 6] = [
        Self::Updates,
        Self::Restart,
        Self::FailedServices,
        Self::LowSpace,
        Self::Staged,
        Self::WatchFlake,
    ];

    #[must_use = "only names the report"]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Updates => "updates",
            Self::Restart => "restart",
            Self::FailedServices => "failed_services",
            Self::LowSpace => "low_space",
            Self::Staged => "staged",
            Self::WatchFlake => "watch_flake",
        }
    }

    #[must_use = "only looks the key up"]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|report| report.key() == key)
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "one on/off switch per report, as the settings file spells them"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tray {
    pub updates: bool,
    pub check_hours: u32,
    pub health_minutes: u32,
    pub restart: bool,
    pub failed_services: bool,
    pub low_space: bool,
    pub staged: bool,
    pub watch_flake: bool,
}

impl Default for Tray {
    fn default() -> Self {
        Self {
            updates: true,
            check_hours: 6,
            health_minutes: 1,
            restart: true,
            failed_services: true,
            low_space: true,
            staged: true,
            watch_flake: false,
        }
    }
}

impl Tray {
    pub const CHECK_HOURS: std::ops::RangeInclusive<u32> = 1..=168;
    pub const HEALTH_MINUTES: std::ops::RangeInclusive<u32> = 1..=60;

    /// A hand-edited file may say anything, so the value is clamped.
    #[must_use = "the check interval is only useful for scheduling"]
    pub fn check_every(&self) -> std::time::Duration {
        let hours = self
            .check_hours
            .clamp(*Self::CHECK_HOURS.start(), *Self::CHECK_HOURS.end());

        std::time::Duration::from_hours(u64::from(hours))
    }

    #[must_use = "the health interval is only useful for scheduling"]
    pub fn health_every(&self) -> std::time::Duration {
        let minutes = self
            .health_minutes
            .clamp(*Self::HEALTH_MINUTES.start(), *Self::HEALTH_MINUTES.end());

        std::time::Duration::from_mins(u64::from(minutes))
    }

    pub fn set_interval(&mut self, name: &str, value: u32) -> bool {
        let (field, range) = match name {
            "check_hours" => (&mut self.check_hours, Self::CHECK_HOURS),
            "health_minutes" => (&mut self.health_minutes, Self::HEALTH_MINUTES),
            _ => return false,
        };

        if !range.contains(&value) {
            return false;
        }

        *field = value;
        true
    }

    pub fn set(&mut self, name: &str, enabled: bool) -> bool {
        let Some(report) = Report::from_key(name) else {
            return false;
        };

        *self.switch(report) = enabled;
        true
    }

    #[must_use = "only reads the setting"]
    pub const fn enabled(mut self, report: Report) -> bool {
        *self.switch(report)
    }

    const fn switch(&mut self, report: Report) -> &mut bool {
        match report {
            Report::Updates => &mut self.updates,
            Report::Restart => &mut self.restart,
            Report::FailedServices => &mut self.failed_services,
            Report::LowSpace => &mut self.low_space,
            Report::Staged => &mut self.staged,
            Report::WatchFlake => &mut self.watch_flake,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct General {
    #[serde(default)]
    pub highlight_theme: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub blur: BlurMode,
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            highlight_theme: String::new(),
            log_level: default_log_level(),
            blur: BlurMode::default(),
            auto_commit: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nix {
    #[serde(default = "default_flake_path_owned")]
    pub flake_path: String,
    #[serde(default)]
    pub host: Option<HostName>,
    #[serde(default)]
    pub managed_path: Option<String>,
}

impl Default for Nix {
    fn default() -> Self {
        Self {
            flake_path: default_flake_path_owned(),
            host: None,
            managed_path: None,
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        save_to(self, &config_path()?)
    }
}

#[must_use = "only returns the path"]
pub const fn default_flake_path() -> &'static str {
    if cfg!(target_os = "macos") {
        DEFAULT_DARWIN_FLAKE_PATH
    } else {
        DEFAULT_FLAKE_PATH
    }
}

fn default_flake_path_owned() -> String {
    default_flake_path().to_owned()
}

fn default_log_level() -> String {
    "info".to_owned()
}

const fn default_true() -> bool {
    true
}

pub fn config_path() -> Result<PathBuf, DirsError> {
    Ok(drakeflake_fs::config_dir()?.join(FILE_NAME))
}

#[must_use = "reading the theme name has no other effect"]
pub fn desktop_theme_name() -> Option<String> {
    let path = drakeflake_fs::config_home()?.join("kdeglobals");

    match std::fs::read_to_string(&path) {
        Ok(text) => theme_name_from_kdeglobals(&text),
        Err(err) => {
            log::debug!("no desktop theme: cannot read {}: {err}", path.display());
            None
        }
    }
}

fn theme_name_from_kdeglobals(text: &str) -> Option<String> {
    let mut section = "";
    let mut color_scheme = None;
    let mut look_and_feel = None;

    for line in text.lines().map(str::trim) {
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name;
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = value.trim();

        match (section, key.trim()) {
            ("General", "ColorScheme") if !value.is_empty() => {
                color_scheme = Some(value.to_owned());
            }
            ("KDE", "LookAndFeelPackage") if !value.is_empty() => {
                look_and_feel = Some(value.to_owned());
            }
            _ => {}
        }
    }

    color_scheme.or(look_and_feel)
}

pub fn tray_lock_path() -> Result<PathBuf, DirsError> {
    Ok(drakeflake_fs::runtime_dir()?.join("drakeflake-tray.lock"))
}

#[must_use = "probing the lock has no other effect"]
pub fn tray_running() -> bool {
    tray_lock_path().is_ok_and(|path| {
        std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .is_ok_and(|file| matches!(file.try_lock(), Err(std::fs::TryLockError::WouldBlock)))
    })
}

#[cfg(test)]
mod tests;
