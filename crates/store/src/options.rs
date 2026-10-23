use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use anyhow::{Context as _, Result};
use drakeflake_render::OptionName;
use serde_json::{Map, Value};

use crate::search::{Haystacks, rank};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionInfo {
    pub name: OptionName,
    pub description: String,
    pub type_name: String,
    pub default_text: Option<String>,
    pub example_text: Option<String>,
    pub declarations: Vec<String>,
    pub read_only: bool,
    pub templated: bool,
}

impl OptionInfo {
    #[must_use = "the summary is only computed"]
    pub fn summary(&self) -> String {
        let first = self.description.split("\n\n").next().unwrap_or_default();
        let mut joined = String::new();

        for word in first.split_whitespace() {
            if !joined.is_empty() {
                joined.push(' ');
            }

            joined.push_str(word);
        }

        strip_roles(&joined)
    }

    #[must_use = "the namespace is only computed"]
    pub fn namespace(&self) -> &str {
        self.name.as_str().split('.').next().unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub struct OptionIndex {
    entries: Vec<OptionInfo>,
    haystacks: Haystacks,
    by_name: HashMap<OptionName, usize>,
}

impl OptionIndex {
    pub fn from_json_str(text: &str) -> Result<Self> {
        let started = Instant::now();
        let data: Value = serde_json::from_str(text).context("options.json is not valid JSON")?;
        let map = data
            .as_object()
            .context("options.json is not a JSON object")?;

        let index = Self::from_map(map);

        log::debug!(
            "option index: {} of {} entries from {} bytes in {:?}",
            index.len(),
            map.len(),
            text.len(),
            started.elapsed()
        );
        Ok(index)
    }

    fn from_map(map: &Map<String, Value>) -> Self {
        let mut entries: Vec<OptionInfo> = map
            .iter()
            .filter_map(|(name, entry)| parse_option(name, entry.as_object()?))
            .collect();

        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Self::from_entries(entries)
    }

    #[must_use = "building the index is costly and has no other effect"]
    pub fn from_entries(mut entries: Vec<OptionInfo>) -> Self {
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        let haystacks = Haystacks::new(
            entries
                .iter()
                .map(|entry| (entry.name.as_str(), entry.description.as_str())),
        );

        let by_name = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.name.clone(), index))
            .collect();

        Self {
            entries,
            haystacks,
            by_name,
        }
    }

    #[must_use = "counting has no other effect"]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use = "checking emptiness has no other effect"]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use = "the lookup has no other effect"]
    pub fn entry(&self, index: usize) -> Option<&OptionInfo> {
        self.entries.get(index)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn get(&self, name: &OptionName) -> Option<&OptionInfo> {
        let index = self.by_name.get(name)?;
        self.entries.get(*index)
    }

    #[must_use = "the ranked matches are the only result of the search"]
    pub fn search(&self, query: &str, keep: impl Fn(&OptionInfo) -> bool) -> Vec<usize> {
        rank(query, self.entries.len(), |index| {
            self.entries
                .get(index)
                .is_some_and(&keep)
                .then(|| self.haystacks.get(index))
                .flatten()
        })
    }

    #[must_use = "counting the namespaces has no other effect"]
    pub fn namespaces(&self) -> Vec<(String, usize)> {
        self.namespaces_by(OptionInfo::namespace)
    }

    #[must_use = "counting the namespaces has no other effect"]
    pub fn namespaces_by<'a>(
        &'a self,
        key: impl Fn(&'a OptionInfo) -> &'a str,
    ) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();

        for entry in &self.entries {
            *counts.entry(key(entry)).or_default() += 1;
        }

        let mut sorted: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(name, count)| (name.to_owned(), count))
            .collect();

        sorted.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        sorted
    }
}

#[must_use = "the stripped text is the only result"]
pub fn strip_roles(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some((before, after)) = rest.split_once('{') {
        let role_code = after
            .split_once('}')
            .filter(|(role, _)| !role.is_empty() && role.chars().all(|ch| ch.is_ascii_lowercase()))
            .and_then(|(_, tail)| tail.strip_prefix('`'));

        out.push_str(before);

        if let Some(code) = role_code {
            if let Some((inner, remainder)) = code.split_once('`') {
                out.push_str(inner);
                rest = remainder;
            } else {
                out.push_str(code);
                rest = "";
            }
        } else {
            out.push('{');
            rest = after;
        }
    }

    out.push_str(rest);
    out
}

fn parse_option(name: &str, entry: &Map<String, Value>) -> Option<OptionInfo> {
    if name.is_empty() || name.starts_with('_') {
        return None;
    }

    Some(OptionInfo {
        name: OptionName::new(name),
        description: doc_text(entry.get("description")).unwrap_or_default(),
        type_name: entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unspecified")
            .to_owned(),
        default_text: doc_text(entry.get("default")),
        example_text: doc_text(entry.get("example")),
        declarations: entry
            .get("declarations")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(declaration_text).collect())
            .unwrap_or_default(),
        read_only: entry
            .get("readOnly")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        templated: name.contains('<') || name.contains('*'),
    })
}

fn doc_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) if map.contains_key("_type") => {
            map.get("text").and_then(Value::as_str).map(str::to_owned)
        }
        other @ (Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Array(_)
        | Value::Object(_)) => Some(serde_json::to_string_pretty(other).unwrap_or_default()),
    }
}

fn declaration_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map
            .get("name")
            .or_else(|| map.get("url"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => None,
    }
}
