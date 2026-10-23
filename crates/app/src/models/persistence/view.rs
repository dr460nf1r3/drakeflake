use std::collections::BTreeSet;

use drakeflake_file::listed;
use drakeflake_flake::{PersistKind, PersistPlace, PersistScope, Persistence};
use drakeflake_plan::ManagedState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowState {
    #[serde(rename = "")]
    Kept,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersistedRow {
    pub path: String,
    pub entry: String,
    pub kind: PersistKind,
    pub storage: String,
    pub scope: PersistScope,
    pub user: String,
    pub option: String,
    pub state: RowState,
    pub locked: bool,
    pub source: String,
}

pub fn persisted_rows(
    data: &Persistence,
    saved: &ManagedState,
    plan: &ManagedState,
) -> Vec<PersistedRow> {
    let mut rows = evaluated_rows(data, saved, plan);
    let mut seen: BTreeSet<(String, String)> = rows
        .iter()
        .map(|row| (row.option.clone(), row.entry.clone()))
        .collect();

    rows.extend(staged_rows(data, saved, plan, &mut seen));
    rows.sort_by(|one, other| one.path.cmp(&other.path));
    rows
}

fn evaluated_rows(
    data: &Persistence,
    saved: &ManagedState,
    plan: &ManagedState,
) -> Vec<PersistedRow> {
    data.entries
        .iter()
        .map(|entry| {
            let option = PersistPlace::of(entry).option();
            let holds = |state: &ManagedState| {
                listed(state, &option).is_some_and(|list| list.contains(&entry.entry))
            };

            let in_saved = holds(saved);
            let state = if in_saved && !holds(plan) {
                RowState::Remove
            } else {
                RowState::Kept
            };

            PersistedRow {
                path: entry.path.clone(),
                entry: entry.entry.clone(),
                kind: entry.kind,
                storage: entry.storage.clone(),
                scope: entry.scope,
                user: entry.user.clone(),
                state,
                locked: !in_saved,
                source: if in_saved {
                    String::new()
                } else {
                    data.source_of(entry)
                },
                option,
            }
        })
        .collect()
}

fn staged_rows(
    data: &Persistence,
    saved: &ManagedState,
    plan: &ManagedState,
    seen: &mut BTreeSet<(String, String)>,
) -> Vec<PersistedRow> {
    let mut rows = Vec::new();

    for place in data.places() {
        let option = place.option();
        let saved_list = listed(saved, &option).unwrap_or_default();

        for entry in listed(plan, &option).unwrap_or_default() {
            if !seen.insert((option.clone(), entry.clone())) {
                continue;
            }

            rows.push(PersistedRow {
                path: data.absolute(&place, &entry),
                state: if saved_list.contains(&entry) {
                    RowState::Kept
                } else {
                    RowState::Add
                },
                kind: place.kind,
                storage: place.storage.clone(),
                scope: place.scope,
                user: place.user.clone(),
                option: option.clone(),
                locked: false,
                source: String::new(),
                entry,
            });
        }
    }

    rows
}
