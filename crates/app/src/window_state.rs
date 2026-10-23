#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MAX_GEOMETRY: i32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Geometry {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Default)]
pub struct WindowState {
    geometry: Geometry,
}

impl WindowState {
    #[must_use = "the loaded geometry is lost"]
    pub fn load() -> Self {
        Self {
            geometry: path().map_or_else(
                |err| {
                    log::debug!("window geometry not loaded: {err}");
                    Geometry::default()
                },
                |file| load_from(&file),
            ),
        }
    }

    #[must_use = "the returned geometry is lost"]
    pub const fn geometry(&self) -> Geometry {
        self.geometry
    }

    pub fn save(&mut self, geometry: Geometry) -> std::io::Result<()> {
        save_to(&geometry, &path().map_err(std::io::Error::other)?)?;
        self.geometry = geometry;
        Ok(())
    }
}

fn path() -> Result<PathBuf, drakeflake_fs::DirsError> {
    Ok(drakeflake_fs::state_dir()?.join("window.json"))
}

#[must_use = "the loaded geometry is lost"]
pub fn load_from(path: &Path) -> Geometry {
    let text = match std::fs::read(path) {
        Ok(text) => text,
        // A denied read recurs on the next save, so it is worth a debug line.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Geometry::default(),
        Err(err) => {
            log::debug!("window geometry in {} ignored: {err}", path.display());
            return Geometry::default();
        }
    };

    match serde_json::from_slice(&text) {
        Ok(geometry) => geometry,
        Err(err) => {
            log::debug!("window geometry in {} ignored: {err}", path.display());
            Geometry::default()
        }
    }
}

pub fn save_to(geometry: &Geometry, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let text = serde_json::to_string_pretty(geometry).map_err(std::io::Error::other)?;

    // A crash mid-write must not leave a truncated file behind.
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    if let Err(err) = std::fs::rename(&tmp, path) {
        if let Err(cleanup) = std::fs::remove_file(&tmp) {
            log::debug!("leftover window geometry {} kept: {cleanup}", tmp.display());
        }

        return Err(err);
    }

    Ok(())
}

#[must_use = "a rejected geometry would be saved"]
pub fn geometry_valid(x: i32, y: i32, width: i32, height: i32) -> bool {
    width > 0_i32
        && height > 0_i32
        && width <= MAX_GEOMETRY
        && height <= MAX_GEOMETRY
        && (-MAX_GEOMETRY..=MAX_GEOMETRY).contains(&x)
        && (-MAX_GEOMETRY..=MAX_GEOMETRY).contains(&y)
}
