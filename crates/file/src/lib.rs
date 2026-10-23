//! Generated and rewritten whole, so user-written Nix is never edited. The state line lets the next run read back exactly what the app manages.

mod backup;
mod direct;
mod elevated;
mod elevation;
mod error;
mod git;
mod imports;
mod layout;
mod module;
mod option;
mod paths;
mod render;
mod source_files;
mod staged;
mod state;
mod string_list;
mod syntax;
mod write;

#[cfg(test)]
mod integration;
#[cfg(test)]
mod tests;

pub use backup::{backup_dir, backup_to, staged_dir};
pub use elevation::{Elevation, Prompt, find_elevation};
pub use error::FileError;
pub use git::{GitState, git_state, git_track, outside_work_tree};
pub use imports::is_imported;
pub use layout::PACKAGES_PATH;
pub use module::host_condition;
pub use option::{ManagedOptionExt, OVERRIDE_PRIORITY, RenderedValue, host_scope_allowed};
pub use paths::{
    FOLDER_ENTRY, is_inside, managed_path, resolve_existing, resolve_managed_path, settings_folder,
    superseded_files,
};
pub use render::ManagedStateExt;
pub use source_files::nix_files;
pub use staged::{
    CheckNeeded, PriorityChoices, SettingsInput, StageError, Staged, StagedChanges, check_key,
    parse_mode, priority_choices,
};
pub use state::IMPORT_MARKER;
pub use string_list::{LIST_TYPE, list_input, listed};
pub use syntax::{check_expression, check_syntax};
pub use write::{WriteMethod, write_files};
