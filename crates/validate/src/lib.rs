//! Nothing here panics on any input.

mod error;
mod expression;
mod input;
mod search;
mod source;
mod types;
mod value;

pub use drakeflake_render::wrap_expression;
pub use error::{ExpressionError, InputError, Position};
pub use expression::parse_expression;
pub use input::{parse_input, parse_package};
pub use search::{HitKind, SourceHit, search_source};
pub use source::{Site, definition_sites, import_paths};
pub use types::{Editor, OptionType, ValueShape, parse_type, value_shape};
pub use value::{display_value, json_to_nix, value_to_input};

#[cfg(test)]
mod tests;
