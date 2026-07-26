//! Bitstream I/O — original/source/bitsIO.c
//!
//! Layout: `common` (BitStream buffer), `encode` (bit output + flush),
//! `decode` (bit input + flush).

mod common;
mod decode;
mod encode;

#[cfg(test)]
mod tests;

pub use common::BitStream;
pub use decode::{bits_read, segment_buffer_flush_decoder};
pub use encode::{bits_write, segment_buffer_flush_encoder};
