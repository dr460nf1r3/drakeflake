use std::path::Path;

use anyhow::{Context as _, Result};
use drakeflake_fs::atomic_write;

use crate::Config;

pub fn save_to(config: &Config, path: &Path) -> Result<()> {
    write_config(config, path)
        .inspect(|()| log::debug!("config saved to {}", path.display()))
        .inspect_err(|err| log::error!("cannot save config: {err:#}"))
}

fn write_config(config: &Config, path: &Path) -> Result<()> {
    refuse_to_replace_broken(path)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }

    let text = toml::to_string_pretty(config).context("cannot encode config")?;

    atomic_write(path, &text).with_context(|| format!("cannot write {}", path.display()))
}

fn refuse_to_replace_broken(path: &Path) -> Result<()> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str::<Config>(&text).map(drop).map_err(|err| {
            anyhow::anyhow!(
                "{} exists but cannot be parsed, so settings are not saved: {err}. \
                 Fix or remove the file",
                path.display()
            )
        }),
        // A parent that is no directory fails later with a clearer message.
        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(())
        }
        Err(err) => Err(anyhow::anyhow!(
            "{} exists but cannot be read, so settings are not saved: {err}. \
             Fix or remove the file",
            path.display()
        )),
    }
}
