mod blocks;
mod index;
mod signature;

#[cfg(test)]
mod tests;

pub use blocks::{Block, blocks};
pub use index::{
    FORMAT_VERSION, Filter, Function, FunctionIndex, IndexFile, Namespace, Position, Source,
    index_path,
};
pub use signature::{Shape, TypeKind, UnknownTypeKind, signature_from_doc};
