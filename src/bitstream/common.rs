//! Shared bitstream buffer state (`BitStream`).

use std::fs::File;

use crate::error::{BpeError, BpeResult};

#[derive(Debug)]
pub struct BitStream {
    pub seg_bit_counter: u32,
    pub total_bit_counter: u32,
    pub byte_buffer_4bytes: u32,
    pub code_word_alignment_bits: u32,
    pub code_word_length: u8,
    pub file: Option<File>,
}

impl BitStream {
    pub fn new(code_word_length: u8) -> Self {
        Self {
            seg_bit_counter: 0,
            total_bit_counter: 0,
            byte_buffer_4bytes: 0,
            code_word_alignment_bits: 0,
            code_word_length,
            file: None,
        }
    }

    pub fn open_write(&mut self, path: &str) -> BpeResult<()> {
        self.file = Some(File::create(path).map_err(|_| BpeError::FileError)?);
        Ok(())
    }

    pub fn open_read(&mut self, path: &str) -> BpeResult<()> {
        self.file = Some(File::open(path).map_err(|_| BpeError::FileError)?);
        Ok(())
    }
}
