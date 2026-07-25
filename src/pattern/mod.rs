//! Pattern mapping and Rice option selection — original/source/PatternCoding.c
//!
//! Mapping tables + map/demap live in mapping; encode-only option selection
//! lives in options. Refine/stage orchestration moved to crate::stages.

mod mapping;
pub(crate) mod options;

pub use mapping::{bit_plane_symbol_reset, de_mapping_pattern};
