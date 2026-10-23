// Hand-written indexing is denied (`indexing_slicing`), so what this catches
// is the slot dispatch that `#[qobject]` generates for slots with several arguments.
#![expect(
    clippy::missing_asserts_for_indexing,
    reason = "qtbridge's generated slot dispatch indexes its argument list"
)]
#![expect(
    clippy::needless_pass_by_value,
    reason = "qtbridge slots receive owned arguments"
)]

pub mod changes;
pub mod config;
pub mod console;
pub mod docs;
pub mod functions;
pub mod generations;
pub mod images;
pub mod inputs;
pub mod options;
pub mod packages;
pub mod persistence;
pub mod relay;
pub mod search;
pub mod settings;
pub mod setup;
pub mod system;
mod vec_model;

#[cfg(test)]
mod tests;

use std::ffi::OsString;

use qtbridge::{QListModel, QListModelBase};
use serde::Serialize;

pub const PAGE_SIZE: usize = 100;

#[derive(Serialize)]
pub struct TopResult<Row> {
    pub total: usize,
    pub rows: Vec<Row>,
}

/// A reset makes every view recreate all delegates, which is slow on every keystroke.
pub fn replace_rows<M>(model: &mut M, rows: Vec<<M as QListModel>::Item>)
where
    M: QListModelBase,
    <M as QListModel>::Item: PartialEq,
{
    let mut incoming = rows.into_iter();

    for position in 0..model.len() {
        let Some(row) = incoming.next() else {
            while model.len() > position {
                model.pop();
            }

            return;
        };

        if model.get(position) != Some(&row) {
            model.set(position, row);
        }
    }

    for row in incoming {
        model.push(row);
    }
}

#[must_use = "the clamped count is the only result"]
pub fn count_i32(count: usize) -> i32 {
    i32::try_from(count).unwrap_or(i32::MAX)
}

#[must_use = "the shortened line is the only result"]
pub fn one_line(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if flat.chars().nth(max).is_none() {
        return flat;
    }

    let mut cut: String = flat.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

pub fn open_folder(folder: String) -> Result<(), String> {
    // `--` so a folder named `-something` is not read as a flag of the opener.
    crate::hub::terminal::spawn_detached(&[
        OsString::from(crate::hub::terminal::OPENER),
        OsString::from("--"),
        OsString::from(folder),
    ])
}
