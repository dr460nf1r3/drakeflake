//! The root helper links only this crate, so rendering and evaluation stay out of it.

pub mod exit;

mod header;
mod option;
mod plan;
mod refusal;
mod root;

mod state;
mod syntax;
mod target;
mod unique;

#[cfg(test)]
mod tests;

pub use header::{
    FILE_NAME, FILES_PREFIX, HEADER, STATE_PREFIX, generated_files, has_generated_header,
    state_line,
};
pub use option::{InputMode, ManagedOption, Priority, UnknownMode, UnknownPriority};
pub use plan::{Rendered, RenderedFile, ValidatedPlan, WritePlan};
pub use refusal::Refusal;
pub use root::find_flake_root;
pub use state::{ManagedState, StateError, check_state_line};
pub use syntax::{
    RunError, SYNTAX_CHECK_TIMEOUT, SyntaxError, SyntaxRun, check_syntax, check_syntax_with,
};
pub use target::{
    check_generated_content, check_local_remove_target, check_local_write_target,
    check_remove_target, check_write_content, check_write_target,
};
pub use unique::next_serial;
