use std::fs;
use std::path::{Path, PathBuf};

use drakeflake_flake::Target;
use drakeflake_fs::{DirsError, fnv1a};

pub const FINGERPRINT_FILE: &str = "fingerprint";
pub const HOSTS_FILE: &str = "hosts.json";
pub const INSTALLED_FILE: &str = "installed.json";
pub const OPTIONS_FILE: &str = "options.json";
pub const OPTIONS_KEY_FILE: &str = "options-key";
pub(crate) const OPTIONS_SKIP_FILE: &str = "options-skip.json";
pub const PACKAGES_FILE: &str = "packages.json";
pub const PACKAGES_SKIP_FILE: &str = "packages-skip.json";
pub(crate) const PERSISTENCE_FILE: &str = "persistence.json";
pub const VALUES_FILE: &str = "values.json";
pub(crate) const VALUES_SKIP_FILE: &str = "values-skip.json";

pub struct Cache {
    dir: Option<PathBuf>,
}

impl Cache {
    #[must_use = "the cache is only located, nothing is read or written"]
    pub const fn at(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }

    #[must_use = "the folder is only named, nothing is read"]
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    pub fn root() -> Result<PathBuf, DirsError> {
        drakeflake_fs::cache_dir()
    }

    #[must_use = "the cache is only located, nothing is read or written"]
    pub fn for_target(target: &Target) -> Self {
        Self::keyed(&format!(
            "{}#{}#{}",
            target.flake.trim(),
            target.kind.as_str(),
            target.host
        ))
    }

    #[must_use = "the cache is only located, nothing is read or written"]
    pub fn for_flake(flake: &str) -> Self {
        Self::keyed(&format!("{}#", flake.trim()))
    }

    fn keyed(key: &str) -> Self {
        let dir = Self::root()
            .inspect_err(|err| log::warn!("not caching: {err}"))
            .ok()
            .map(|root| root.join(format!("{:016x}", fnv1a(key.as_bytes()))));

        Self { dir }
    }

    #[must_use = "the cached text is the only result of reading it"]
    pub fn read(&self, name: &str) -> Option<String> {
        let path = self.dir.as_ref()?.join(name);

        match fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                log::debug!("cache miss: {}", path.display());
                None
            }
            Err(err) => {
                log::warn!("cannot read cache file {}: {err}", path.display());
                None
            }
        }
    }

    #[must_use = "the parsed file is the only result of reading it"]
    pub fn read_parsed<T, E>(
        &self,
        name: &str,
        parse: impl FnOnce(&str) -> Result<T, E>,
    ) -> Option<T>
    where
        E: std::fmt::Display,
    {
        parse(&self.read(name)?)
            .inspect_err(|err| log::warn!("ignoring corrupt cache file {name}: {err:#}"))
            .ok()
    }

    pub fn write(&self, name: &str, text: &str) {
        let Some(dir) = &self.dir else {
            return;
        };

        let path = dir.join(name);
        let written =
            fs::create_dir_all(dir).and_then(|()| drakeflake_fs::atomic_write(&path, text));

        if let Err(err) = written {
            log::warn!("cannot write cache file {}: {err}", path.display());
        }
    }

    #[must_use = "the cached list is the only result of reading it"]
    pub fn read_list(&self, name: &str) -> Vec<String> {
        self.read_parsed(name, |text| serde_json::from_str::<Vec<String>>(text))
            .unwrap_or_default()
    }

    pub fn write_list(&self, name: &str, items: &[String]) {
        match serde_json::to_string(items) {
            Ok(text) => self.write(name, &text),
            Err(err) => log::warn!("cannot serialise cache file {name}: {err}"),
        }
    }

    #[must_use = "the answer decides whether the cache is used"]
    pub fn is_current(&self, fingerprint: Option<&str>) -> bool {
        let Some(dir) = &self.dir else {
            return false;
        };

        let complete = [OPTIONS_FILE, VALUES_FILE, PACKAGES_FILE, INSTALLED_FILE]
            .iter()
            .all(|name| dir.join(name).is_file());

        let current = complete
            && fingerprint.is_some()
            && self.read(FINGERPRINT_FILE).as_deref() == fingerprint;

        log::debug!(
            "cache {}: complete {complete}, current {current}",
            dir.display()
        );
        current
    }

    pub fn save_fingerprint(&self, fingerprint: Option<&str>) {
        self.save_text(FINGERPRINT_FILE, fingerprint);
    }

    pub fn save_text(&self, name: &str, text: Option<&str>) {
        match (text, &self.dir) {
            (Some(value), _) => self.write(name, value),
            (None, Some(dir)) => {
                let path = dir.join(name);
                warn_unless_missing(&path, fs::remove_file(&path));
            }
            (None, None) => {}
        }
    }

    pub fn remove(&self, name: &str) {
        self.save_text(name, None);
    }

    pub fn clear(&self) {
        if let Some(dir) = &self.dir {
            log::debug!("clearing cache {}", dir.display());
            warn_unless_missing(dir, fs::remove_dir_all(dir));
        }
    }
}

fn warn_unless_missing(path: &Path, removed: std::io::Result<()>) {
    if let Err(err) = removed
        && err.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("cannot remove {}: {err}", path.display());
    }
}

#[cfg(test)]
mod tests;
