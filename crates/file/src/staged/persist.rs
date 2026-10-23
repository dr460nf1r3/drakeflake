use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use serde::Deserialize as _;
use serde_json::{Value, json};

use super::{Staged, StagedChanges};

impl StagedChanges {
    #[must_use = "only builds the text"]
    pub fn to_json(&self) -> String {
        json!({ "options": self.options, "packages": self.packages }).to_string()
    }

    #[must_use = "only parses the value"]
    pub fn from_value(saved: &Value) -> Self {
        Self {
            options: parse_field(saved, "options"),
            packages: parse_field(saved, "packages"),
        }
    }

    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };

        let saved = serde_json::from_str::<Value>(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        Ok(Some(Self::from_value(&saved)))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if self.is_empty() {
            return match std::fs::remove_file(path) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                Ok(()) | Err(_) => Ok(()),
            };
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        drakeflake_fs::atomic_write(path, &self.to_json())
    }
}

fn parse_field<MapKey>(saved: &Value, field: &str) -> BTreeMap<MapKey, Staged>
where
    MapKey: Ord + for<'de> serde::Deserialize<'de>,
{
    let Some(value) = saved.get(field) else {
        return BTreeMap::new();
    };

    let Ok(map) = serde_json::Map::deserialize(value) else {
        log::warn!("ignoring invalid staged {field}: not an object");
        return BTreeMap::new();
    };

    let mut kept = BTreeMap::new();
    for (raw_key, raw_value) in map {
        let key: MapKey = match MapKey::deserialize(Value::String(raw_key)) {
            Ok(key) => key,
            Err(error) => {
                log::warn!("ignoring invalid staged {field} key: {error}");
                continue;
            }
        };

        match Staged::deserialize(&raw_value) {
            Ok(staged) => {
                kept.insert(key, staged);
            }
            Err(error) => {
                log::warn!("ignoring invalid staged {field} entry: {error}");
            }
        }
    }

    kept
}
