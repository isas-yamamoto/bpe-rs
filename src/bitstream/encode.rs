//! Bit output and encoder-side segment flush.

use std::io::Write;

use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

fn output_code_word(coding: &mut CodingPara) -> BpeResult<()> {
    coding.bits.code_word_alignment_bits += 1;
    coding.bits.seg_bit_counter += 1;
    coding.bits.total_bit_counter += 1;

    if coding.bits.code_word_alignment_bits == coding.bits.code_word_length as u32 {
        let file = coding.bits.file.as_mut().ok_or(BpeError::FileError)?;
        match coding.bits.code_word_length {
            8 => {
                let temp = coding.bits.byte_buffer_4bytes as u8;
                file.write_all(&[temp]).map_err(|_| BpeError::FileError)?;
            }
            16 => {
                // Host little-endian fwrite of WORD16 (matches Windows/x86 C)
                let temp = coding.bits.byte_buffer_4bytes as u16;
                file.write_all(&temp.to_le_bytes())
                    .map_err(|_| BpeError::FileError)?;
            }
            24 => {
                // C writes low byte, then mid, then high via putc of masked values
                let buf = coding.bits.byte_buffer_4bytes;
                file.write_all(&[(buf & 0xFF) as u8])
                    .map_err(|_| BpeError::FileError)?;
                file.write_all(&[((buf & 0xFF00) >> 8) as u8])
                    .map_err(|_| BpeError::FileError)?;
                file.write_all(&[((buf & 0xFF0000) >> 16) as u8])
                    .map_err(|_| BpeError::FileError)?;
            }
            32 => {
                let temp = coding.bits.byte_buffer_4bytes;
                file.write_all(&temp.to_le_bytes())
                    .map_err(|_| BpeError::FileError)?;
            }
            _ => return Err(BpeError::StreamError),
        }
        coding.bits.code_word_alignment_bits = 0;
    }
    Ok(())
}

pub fn bits_write(coding: &mut CodingPara, bit: u32, mut length: i32) -> BpeResult<()> {
    if length == 0 {
        return Ok(());
    }
    if coding.segment_full {
        return Ok(());
    }

    if coding.header.part2.seg_byte_limit_27bits != 0 {
        if coding.bits.seg_bit_counter.wrapping_add(length as u32)
            >= coding.header.part2.seg_byte_limit_27bits * 8
        {
            coding.segment_full = true;
            let remainder_bits = (coding.header.part2.seg_byte_limit_27bits * 8
                - coding.bits.seg_bit_counter) as i32;
            let mut i = remainder_bits - 1;
            while i >= 0 {
                let temp_bits = (0x01 & (bit >> (length - 1))) as u8;
                length -= 1;
                coding.bits.byte_buffer_4bytes <<= 1;
                coding.bits.byte_buffer_4bytes += temp_bits as u32;
                output_code_word(coding)?;
                i -= 1;
            }
            return Ok(());
        }
    }

    if length > 32 {
        let tt = length - 32;
        coding.bits.byte_buffer_4bytes <<= tt;
        output_code_word(coding)?;
        length = 32;
    }

    let mut i = length - 1;
    while i >= 0 {
        let temp_bits = (0x01 & (bit >> i)) as u8;
        coding.bits.byte_buffer_4bytes <<= 1;
        coding.bits.byte_buffer_4bytes += temp_bits as u32;
        output_code_word(coding)?;
        i -= 1;
    }
    Ok(())
}

pub fn segment_buffer_flush_encoder(coding: &mut CodingPara) -> BpeResult<()> {
    if coding.bits.code_word_alignment_bits != 0 {
        let shift =
            coding.bits.code_word_length as i32 - coding.bits.code_word_alignment_bits as i32;
        bits_write(coding, 0, shift)?;
    }
    if coding.header.part2.seg_byte_limit_27bits != 0
        && !coding.segment_full
        && coding.header.part2.use_fill
    {
        while !coding.segment_full {
            bits_write(coding, 0, 8)?;
        }
    }
    coding.bits.seg_bit_counter = 0;
    coding.bits.byte_buffer_4bytes = 0;
    coding.bits.code_word_alignment_bits = 0;
    Ok(())
}
