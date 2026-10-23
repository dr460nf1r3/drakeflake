//! Without a usable `HOME` files go to a private folder in the temp dir or nowhere, never to a shared place.

mod atomic;
mod env;
mod hash;
mod private;

use std::fmt;
use std::path::PathBuf;

pub use atomic::{atomic_write, sibling, stage};
use env::{Env, Kind};
pub use hash::{FNV_OFFSET, fnv1a, fnv1a_from};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirsError {
    pub path: PathBuf,
    pub reason: String,
}

impl fmt::Display for DirsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no safe place for the app's files: {} {}",
            self.path.display(),
            self.reason
        )
    }
}

impl std::error::Error for DirsError {}

#[must_use = "only reads the environment"]
pub fn home_dir() -> Option<PathBuf> {
    Env::from_process().home()
}

#[must_use = "only reads the environment"]
pub fn config_home() -> Option<PathBuf> {
    Env::from_process().base(Kind::Config)
}

pub fn config_dir() -> Result<PathBuf, DirsError> {
    Env::from_process().app_dir(Kind::Config)
}

pub fn state_dir() -> Result<PathBuf, DirsError> {
    Env::from_process().app_dir(Kind::State)
}

pub fn cache_dir() -> Result<PathBuf, DirsError> {
    Env::from_process().app_dir(Kind::Cache)
}

pub fn runtime_dir() -> Result<PathBuf, DirsError> {
    Env::from_process().runtime()
}
