use std::time::Instant;

use anyhow::{Context as _, Result};
use drakeflake_render::PackageAttr;
use serde_json::Value;

use crate::search::{Haystacks, rank};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageInfo {
    pub attr: PackageAttr,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Default)]
pub struct PackageIndex {
    entries: Vec<PackageInfo>,
    haystacks: Haystacks,
}

impl PackageIndex {
    pub fn from_json_str(text: &str) -> Result<Self> {
        let started = Instant::now();
        let data: Value = serde_json::from_str(text).context("package index is not valid JSON")?;
        let map = data
            .as_object()
            .context("package index is not a JSON object")?;

        let mut entries: Vec<PackageInfo> = map
            .iter()
            .filter(|(_, entry)| entry.is_object())
            .map(|(attr, entry)| PackageInfo {
                attr: PackageAttr::new(attr),
                version: str_field(entry, "v"),
                description: str_field(entry, "d"),
            })
            .collect();

        entries.sort_by(|left, right| left.attr.cmp(&right.attr));
        let index = Self::from_entries(entries);

        log::debug!(
            "package index: {} of {} entries from {} bytes in {:?}",
            index.len(),
            map.len(),
            text.len(),
            started.elapsed()
        );
        Ok(index)
    }

    #[must_use = "building the index is costly and has no other effect"]
    pub fn from_entries(mut entries: Vec<PackageInfo>) -> Self {
        entries.sort_by(|left, right| left.attr.cmp(&right.attr));
        let haystacks = Haystacks::new(
            entries
                .iter()
                .map(|entry| (entry.attr.as_str(), entry.description.as_str())),
        );

        Self { entries, haystacks }
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
    pub fn entry(&self, index: usize) -> Option<&PackageInfo> {
        self.entries.get(index)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn position(&self, attr: &PackageAttr) -> Option<usize> {
        self.entries
            .binary_search_by(|entry| entry.attr.cmp(attr))
            .ok()
    }

    #[must_use = "the lookup has no other effect"]
    pub fn entry_by_attr(&self, attr: &PackageAttr) -> Option<&PackageInfo> {
        let index = self.position(attr)?;
        self.entries.get(index)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn contains(&self, attr: &PackageAttr) -> bool {
        self.position(attr).is_some()
    }

    #[must_use = "the ranked matches are the only result of the search"]
    pub fn search(&self, query: &str) -> Vec<usize> {
        rank(query, self.entries.len(), |index| self.haystacks.get(index))
    }
}

fn str_field(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
