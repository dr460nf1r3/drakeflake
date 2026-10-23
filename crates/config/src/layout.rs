use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
use toml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutMode {
    Single,
    Area,
    #[default]
    AreaModule,
}

impl fmt::Display for LayoutMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Single => "single",
            Self::Area => "area",
            Self::AreaModule => "area-module",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownLayoutMode;

impl FromStr for LayoutMode {
    type Err = UnknownLayoutMode;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        [Self::Single, Self::Area, Self::AreaModule]
            .into_iter()
            .find(|mode| mode.to_string() == name)
            .ok_or(UnknownLayoutMode)
    }
}

/// Only built through [`Self::new`] and deserialization, which both check the
/// file: it is joined to a folder, so it must stay inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawRule")]
pub struct LayoutRule {
    prefix: String,
    file: String,
}

#[derive(Deserialize)]
struct RawRule {
    prefix: String,
    file: String,
}

impl TryFrom<RawRule> for LayoutRule {
    type Error = String;

    fn try_from(raw: RawRule) -> Result<Self, String> {
        Self::new(&raw.prefix, &raw.file)
    }
}

impl LayoutRule {
    pub fn new(prefix: &str, file: &str) -> Result<Self, String> {
        let prefix = prefix.trim();

        if prefix.is_empty() || prefix.split('.').any(str::is_empty) {
            return Err(format!("\"{prefix}\" is not an option path"));
        }

        Ok(Self {
            prefix: prefix.to_owned(),
            file: clean_file(file)?,
        })
    }

    #[must_use = "only reads the prefix"]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    #[must_use = "only reads the file"]
    pub fn file(&self) -> &str {
        &self.file
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutConfig {
    #[serde(default)]
    pub mode: LayoutMode,
    #[serde(default, deserialize_with = "valid_rules")]
    pub rules: Vec<LayoutRule>,
}

impl LayoutConfig {
    pub fn parse_rules(text: &str) -> Result<Vec<LayoutRule>, String> {
        text.lines()
            .enumerate()
            .map(|(number, line)| {
                (
                    number + 1,
                    line.split('#').next().unwrap_or_default().trim(),
                )
            })
            .filter(|(_, line)| !line.is_empty())
            .map(|(number, line)| {
                let (prefix, file) = line
                    .split_once('=')
                    .ok_or_else(|| format!("line {number}: expected `prefix = file.nix`"))?;

                LayoutRule::new(prefix, file).map_err(|err| format!("line {number}: {err}"))
            })
            .collect()
    }

    #[must_use = "the rules are only rendered as text"]
    pub fn rules_text(&self) -> String {
        self.rules
            .iter()
            .map(|rule| format!("{} = {}", rule.prefix, rule.file))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[must_use = "only reads the text"]
pub fn dropped_rules(text: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct File {
        layout: Option<Layout>,
    }

    #[derive(Deserialize)]
    struct Layout {
        #[serde(default)]
        rules: Vec<Value>,
    }

    toml::from_str::<File>(text)
        .ok()
        .and_then(|file| file.layout)
        .map(|layout| sort_rules(layout.rules).1)
        .unwrap_or_default()
}

fn valid_rules<'de, D>(deserializer: D) -> Result<Vec<LayoutRule>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<Value>::deserialize(deserializer).map(|rules| sort_rules(rules).0)
}

fn sort_rules(rules: Vec<Value>) -> (Vec<LayoutRule>, Vec<String>) {
    let mut valid = Vec::new();
    let mut dropped = Vec::new();

    for (index, rule) in rules.into_iter().enumerate() {
        let named = rule
            .get("prefix")
            .and_then(Value::as_str)
            .map_or_else(String::new, |prefix| format!(" (\"{prefix}\")"));

        match rule.try_into::<LayoutRule>() {
            Ok(rule) => valid.push(rule),
            Err(err) => dropped.push(format!(
                "Ignored layout rule {}{named} in config.toml: {}",
                index + 1,
                err.message()
            )),
        }
    }

    (valid, dropped)
}

fn clean_file(file: &str) -> Result<String, String> {
    let trimmed = file.trim().trim_start_matches("./");

    if trimmed.is_empty() || trimmed.starts_with('/') {
        return Err(format!("\"{}\" must be a relative file name", file.trim()));
    }

    let valid = trimmed.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    });

    if !valid {
        return Err(format!(
            "\"{trimmed}\" may only use letters, digits, '.', '_', '-' and '/'"
        ));
    }

    let has_extension = std::path::Path::new(trimmed)
        .extension()
        .is_some_and(|ext| ext == "nix");

    Ok(if has_extension {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.nix")
    })
}

#[cfg(test)]
mod tests;
