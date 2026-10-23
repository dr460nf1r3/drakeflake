//! No persistent search index: a scored match over precomputed haystacks is fast enough for ~25k entries.

mod options;
mod packages;
mod search;
mod values;

#[cfg(test)]
mod tests;

pub use drakeflake_render::{OptionName, PackageAttr};
pub use options::{OptionIndex, OptionInfo, strip_roles};
pub use packages::{PackageIndex, PackageInfo};
pub use search::{Haystacks, rank};
pub use values::ValueSnapshot;
