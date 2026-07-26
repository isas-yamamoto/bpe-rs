//! Rice coding — original/source/ricecoding.c
//!
//! Layout: `encode` / `decode` (fixed-table symbol codec), `select_k`
//! (gaggle `k` selection used by DC and AC depth).
//!
//! `option` corresponds to the C `UCHAR8 *Option` / `splitOption` array indexed
//! as `option[0]` (2-bit codewords), `option[1]` (3-bit codewords) and
//! `option[2]` (4-bit codewords).

mod decode;
mod encode;
mod select_k;

pub use decode::rice_decoding;
pub use encode::rice_coding;
pub(crate) use select_k::{select_rice_k, UNCODED_FLAG};
