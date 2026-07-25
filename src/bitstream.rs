//! Bitstream I/O — original/source/bitsIO.c

use std::fs::File;
use std::io::{Read, Write};

use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

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

pub fn bits_output(coding: &mut CodingPara, bit: u32, mut length: i32) -> BpeResult<()> {
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

pub fn segment_buffer_flush_encoder(coding: &mut CodingPara) -> BpeResult<()> {
    if coding.bits.code_word_alignment_bits != 0 {
        let shift =
            coding.bits.code_word_length as i32 - coding.bits.code_word_alignment_bits as i32;
        bits_output(coding, 0, shift)?;
    }
    if coding.header.part2.seg_byte_limit_27bits != 0
        && !coding.segment_full
        && coding.header.part2.use_fill
    {
        while !coding.segment_full {
            bits_output(coding, 0, 8)?;
        }
    }
    coding.bits.seg_bit_counter = 0;
    coding.bits.byte_buffer_4bytes = 0;
    coding.bits.code_word_alignment_bits = 0;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// Write `(value, length)` pairs, flush, then read the same widths back.
    fn assert_roundtrip(name: &str, items: &[(u32, i32)]) {
        let path = temp_path(name);
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        for &(value, length) in items {
            bits_output(&mut enc, value, length).unwrap();
        }
        segment_buffer_flush_encoder(&mut enc).unwrap();
        drop(enc.bits.file.take());

        let mut dec = CodingPara::new();
        dec.bits.open_read(path.to_str().unwrap()).unwrap();
        for &(value, length) in items {
            let got = bits_read(&mut dec, length as i16).unwrap();
            assert_eq!(got, value, "value={} length={}", value, length);
        }
    }

    #[test]
    fn single_bits_roundtrip() {
        let items: Vec<(u32, i32)> = vec![(1, 1), (0, 1), (1, 1), (1, 1), (0, 1)];
        assert_roundtrip("bits_single.bin", &items);
    }

    #[test]
    fn mixed_widths_roundtrip() {
        let items: Vec<(u32, i32)> = vec![(0x5, 3), (0xA, 4), (0x1FF, 9), (0x0, 5), (0xFFFF, 16)];
        assert_roundtrip("bits_mixed.bin", &items);
    }

    #[test]
    fn full_word_roundtrip() {
        let items: Vec<(u32, i32)> = vec![(0xDEADBEEF, 32), (0x0, 8)];
        assert_roundtrip("bits_word.bin", &items);
    }

    #[test]
    fn zero_length_writes_nothing() {
        let path = temp_path("bits_zero.bin");
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        bits_output(&mut enc, 0xFF, 0).unwrap();
        drop(enc.bits.file.take());
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
        assert_eq!(enc.bits.total_bit_counter, 0);
    }

    #[test]
    fn flush_pads_to_whole_codeword() {
        let path = temp_path("bits_flush.bin");
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        bits_output(&mut enc, 1, 3).unwrap();
        segment_buffer_flush_encoder(&mut enc).unwrap();
        drop(enc.bits.file.take());
        // 3 bits become one 8-bit codeword: 001 followed by five zero bits.
        assert_eq!(fs::read(&path).unwrap(), vec![0b0010_0000]);
    }

    #[test]
    fn segment_byte_limit_marks_segment_full() {
        let path = temp_path("bits_limit.bin");
        let mut enc = CodingPara::new();
        enc.header.part2.seg_byte_limit_27bits = 1; // 8 bits allowed
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        bits_output(&mut enc, 0xF, 4).unwrap();
        assert!(!enc.segment_full);
        bits_output(&mut enc, 0xFF, 8).unwrap();
        assert!(
            enc.segment_full,
            "writing past the limit must stop the segment"
        );
        let counter_after_full = enc.bits.total_bit_counter;
        bits_output(&mut enc, 0xFF, 8).unwrap();
        assert_eq!(
            enc.bits.total_bit_counter, counter_after_full,
            "no bits may be emitted once the segment is full"
        );
    }

    #[test]
    fn reads_stop_once_rate_is_reached() {
        let path = temp_path("bits_rate.bin");
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        for _ in 0..4 {
            bits_output(&mut enc, 0xFF, 8).unwrap();
        }
        segment_buffer_flush_encoder(&mut enc).unwrap();
        drop(enc.bits.file.take());

        let mut dec = CodingPara::new();
        dec.decoding_allowed_bits_size_in_segment = 8;
        dec.bits.open_read(path.to_str().unwrap()).unwrap();
        let _ = bits_read(&mut dec, 8).unwrap();
        assert!(
            dec.rate_reached,
            "reaching the allowed size must set rate_reached"
        );
        assert_eq!(bits_read(&mut dec, 8).unwrap(), 0);
    }
}
