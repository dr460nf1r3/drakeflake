use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::DirsError;
use crate::private::private_dir;

const APP_DIR: &str = "drakeflake";

#[derive(Debug, Clone)]
pub struct Env {
    pub home: Option<OsString>,
    pub config_home: Option<OsString>,
    pub state_home: Option<OsString>,
    pub cache_home: Option<OsString>,
    pub runtime_dir: Option<OsString>,
    pub temp: PathBuf,
    pub uid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Config,
    State,
    Cache,
}

impl Kind {
    const fn name(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::State => "state",
            Self::Cache => "cache",
        }
    }

    const fn home_default(self) -> &'static str {
        match self {
            Self::Config => ".config",
            Self::State => ".local/state",
            Self::Cache => ".cache",
        }
    }
}

impl Env {
    pub fn from_process() -> Self {
        let var = std::env::var_os;

        Self {
            home: var("HOME"),
            config_home: var("XDG_CONFIG_HOME"),
            state_home: var("XDG_STATE_HOME"),
            cache_home: var("XDG_CACHE_HOME"),
            runtime_dir: var("XDG_RUNTIME_DIR"),
            temp: std::env::temp_dir(),
            uid: rustix::process::getuid().as_raw(),
        }
    }

    pub fn home(&self) -> Option<PathBuf> {
        absolute(self.home.as_ref())
    }

    pub fn base(&self, kind: Kind) -> Option<PathBuf> {
        let variable = match kind {
            Kind::Config => &self.config_home,
            Kind::State => &self.state_home,
            Kind::Cache => &self.cache_home,
        };

        absolute(variable.as_ref()).or_else(|| Some(self.home()?.join(kind.home_default())))
    }

    pub fn app_dir(&self, kind: Kind) -> Result<PathBuf, DirsError> {
        self.base(kind).map_or_else(
            || private_dir(&self.temp, self.uid).map(|root| root.join(kind.name())),
            |base| Ok(base.join(APP_DIR)),
        )
    }

    pub fn runtime(&self) -> Result<PathBuf, DirsError> {
        absolute(self.runtime_dir.as_ref()).map_or_else(|| self.app_dir(Kind::State), Ok)
    }
}

fn absolute(value: Option<&OsString>) -> Option<PathBuf> {
    value
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests;
