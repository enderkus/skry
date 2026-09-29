//! Remote collection: the batched command and pure parsers for its output.

pub mod parse;
pub mod script;
pub mod types;

pub use parse::RawSample;
pub use script::Section;
pub use types::*;
