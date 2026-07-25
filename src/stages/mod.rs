//! Stage coding across gaggles — original/source/StagesCodingGaggles.c
//!
//! Layout: one encode/decode pair per gagglesN file; refine and orchestrate
//! each own a single pair; shared helpers live in common.

mod common;
mod gaggles1;
mod gaggles2;
mod gaggles3;
mod orchestrate;
mod refine;

pub use orchestrate::{stages_de_coding, stages_en_coding};
