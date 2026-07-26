//! Bit input and decoder-side segment flush.

use std::io::Read;

use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

pub fn bits_read(coding: &mut CodingPara, length: i16) -> BpeResult<u32> {
    let mut bit: u32 = 0;

    if length == 0 || coding.segment_full {
        return Ok(0);
    }
    if coding.rate_reached {
        return Ok(0);
    }

    if !coding.segment_full {
        for i in 0..length {
            if coding.bits.code_word_alignment_bits == 0 {
                let mut buf = [0u8; 1];
                let file = coding.bits.file.as_mut().ok_or(BpeError::FileError)?;
                match file.read(&mut buf) {
                    Ok(0) => {
                        // EOF — C getc returns EOF (-1); treat as 0xFF-like or error
                        coding.bits.byte_buffer_4bytes = 0xFF;
                    }
                    Ok(_) => coding.bits.byte_buffer_4bytes = buf[0] as u32,
                    Err(_) => return Err(BpeError::FileError),
                }
                coding.bits.code_word_alignment_bits = 8;
            }
            bit <<= 1;
            bit += (coding.bits.byte_buffer_4bytes >> (coding.bits.code_word_alignment_bits - 1))
                & 0x01;
            coding.bits.code_word_alignment_bits -= 1;
            coding.bits.seg_bit_counter += 1;
            coding.bits.total_bit_counter += 1;

            if coding.decoding_allowed_bits_size_in_segment != 0
                && coding.bits.seg_bit_counter >= coding.decoding_allowed_bits_size_in_segment
            {
                let mut current_total_bytes =
                    (coding.bits.seg_bit_counter + coding.bits.code_word_alignment_bits) / 8;
                coding.rate_reached = true;
                coding.decoding_stop_locations.bit_plane_stop_decoding = coding.bit_plane as i8 - 1;
                coding.decoding_stop_locations.total_bits_read_this_time = i + 1;
                bit <<= (length - i - 1) as u32;
                while current_total_bytes < coding.header.part2.seg_byte_limit_27bits {
                    let mut discard = [0u8; 1];
                    if let Some(file) = coding.bits.file.as_mut() {
                        let _ = file.read(&mut discard);
                    }
                    current_total_bytes += 1;
                }
                coding.segment_full = true;
                return Ok(bit);
            }
        }
    }
    Ok(bit)
}

pub fn segment_buffer_flush_decoder(coding: &mut CodingPara) -> BpeResult<()> {
    if coding.header.part2.seg_byte_limit_27bits != 0
        && !coding.segment_full
        && coding.header.part2.use_fill
    {
        while !coding.segment_full {
            let _ = bits_read(coding, 8)?;
        }
    }
    coding.bits.total_bit_counter += coding.bits.code_word_alignment_bits;
    coding.bits.seg_bit_counter = 0;
    coding.bits.byte_buffer_4bytes = 0;
    coding.bits.code_word_alignment_bits = 0;
    Ok(())
}
