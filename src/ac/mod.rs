//! AC bit-plane coding — original/source/AC_BitPlaneCoding.c
//!
//! depth: AC max-bit depth coding; bpe: bit-plane loop orchestration.

mod bpe;
mod depth;

pub use bpe::{ac_bpe_decoding, ac_bpe_encoding};
