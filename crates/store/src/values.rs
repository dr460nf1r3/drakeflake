use std::collections::HashMap;
use std::time::Instant;

use anyhow::{Context as _, Result};
use drakeflake_render::OptionName;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct ValueSnapshot {
    values: HashMap<OptionName, Value>,
    user_files: HashMap<OptionName, Vec<String>>,
    entries: usize,
}

impl ValueSnapshot {
    pub fn from_json_str(text: &str) -> Result<Self> {
        let started = Instant::now();
        let data: Value = serde_json::from_str(text).context("value snapshot is not valid JSON")?;
        let Value::Object(map) = data else {
            anyhow::bail!("value snapshot is not a JSON object");
        };

        let mut snapshot = Self {
            entries: map.len(),
            ..Self::default()
        };

        for (name, entry) in map {
            let Value::Object(mut fields) = entry else {
                continue;
            };

            let name = OptionName::new(name);
            let value = fields.remove("v");
            let files: Vec<String> = fields
                .remove("u")
                .and_then(|files| match files {
                    Value::Array(items) => Some(items),
                    Value::Null
                    | Value::Bool(_)
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Object(_) => None,
                })
                .map(|items| {
                    items
                        .into_iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();

            if files.is_empty() {
                if let Some(value) = value {
                    snapshot.values.insert(name, value);
                }
            } else {
                if let Some(value) = value {
                    snapshot.values.insert(name.clone(), value);
                }

                snapshot.user_files.insert(name, files);
            }
        }

        log::debug!(
            "value snapshot: {} options, {} values, {} user-set, from {} bytes in {:?}",
            snapshot.entries,
            snapshot.values.len(),
            snapshot.user_files.len(),
            text.len(),
            started.elapsed()
        );
        Ok(snapshot)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn value(&self, name: &OptionName) -> Option<&Value> {
        self.values.get(name)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn user_files(&self, name: &OptionName) -> &[String] {
        self.user_files.get(name).map_or(&[], Vec::as_slice)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn is_user_set(&self, name: &OptionName) -> bool {
        self.user_files.contains_key(name)
    }

    #[must_use = "counting has no other effect"]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use = "checking emptiness has no other effect"]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[must_use = "counting has no other effect"]
    pub const fn options(&self) -> usize {
        self.entries
    }
}
