//! DC encoding/decoding — original/source/DC_EnDeCoding.c
//!
//! One encode/decode concern per file (twos_comp, dpcm, entropy, coding).

mod coding;
mod dpcm;
mod entropy;
mod twos_comp;

pub use coding::{dc_decoding, dc_encoding};
pub use twos_comp::deconv_twos_comp;
