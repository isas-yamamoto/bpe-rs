//! Image I/O — ImageSize/Read/Write in bpe_encoder.c / bpe_decoder.c
//!
//! Layout: `common` (pixel byte order), `size` (dimensions from file size),
//! `read` (raw input), `write` (raw output, integer and float paths).

mod common;
mod read;
mod size;
mod write;

pub use read::image_read;
pub use size::image_size;
pub use write::{image_write, image_write_float};
