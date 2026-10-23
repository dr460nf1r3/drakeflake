use std::path::Path;

use anyhow::{Context as _, Result};

use crate::layout::dropped_rules;
use crate::{Config, config_path};

#[must_use = "the loaded configuration and its load error are lost"]
pub fn load() -> (Config, Option<String>) {
    let path = match config_path() {
        Ok(path) => path,
        Err(err) => {
            log::warn!("using default settings: {err}");
            return (
                Config::default(),
                Some(format!("Using default settings: {err}")),
            );
        }
    };

    if !path.exists() {
        log::debug!("no config at {}, using defaults", path.display());
        return (Config::default(), None);
    }

    match load_with_notes(&path) {
        Ok((config, notes)) => {
            log::info!("config loaded from {}", path.display());
            (config, (!notes.is_empty()).then(|| notes.join(" ")))
        }
        Err(err) => {
            log::warn!("using default settings: {err:#}");
            (
                Config::default(),
                Some(format!("Using default settings: {err:#}")),
            )
        }
    }
}

pub fn load_from(path: &Path) -> Result<Config> {
    load_with_notes(path).map(|(config, _)| config)
}

pub fn load_with_notes(path: &Path) -> Result<(Config, Vec<String>)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;

    let config =
        toml::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))?;

    let notes = dropped_rules(&text);

    for note in &notes {
        log::warn!("{note}");
    }

    Ok((config, notes))
}
