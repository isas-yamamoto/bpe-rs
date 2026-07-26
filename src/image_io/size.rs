//! Image dimensions derived from (or validated against) the raw file size.

use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;
use std::fs::metadata;

/// Determine (or validate) image rows/width from file size.
pub fn image_size(coding: &mut CodingPara) -> BpeResult<()> {
    let img_len = metadata(&coding.input_file)
        .map_err(|_| BpeError::FileError)?
        .len();

    if coding.image_rows > 0 && coding.image_width > 0 {
        let depth = coding.header.part4.pixel_bit_depth_4bits;
        let temp: u64 = if depth == 0 || depth > 8 { 16 } else { 8 };
        if img_len == coding.image_rows as u64 * coding.image_width as u64 * temp / 8 {
            return Ok(());
        }
        return Err(BpeError::FileError);
    }

    match img_len {
        16384 => {
            coding.image_rows = 128;
            coding.image_width = 128;
        }
        64000 => {
            coding.image_rows = 200;
            coding.image_width = 320;
        }
        65536 => {
            coding.image_rows = 256;
            coding.image_width = 256;
        }
        98304 => {
            coding.image_rows = 256;
            coding.image_width = 384;
        }
        196608 => {
            coding.image_rows = 512;
            coding.image_width = 384;
        }
        262144 => {
            coding.image_rows = 512;
            coding.image_width = 512;
        }
        307200 => {
            coding.image_rows = 480;
            coding.image_width = 640;
        }
        345600 => {
            coding.image_rows = 720;
            coding.image_width = 480;
        }
        414720 => {
            coding.image_rows = 576;
            coding.image_width = 720;
        }
        524288 => {
            coding.image_rows = 512;
            coding.image_width = 512;
            coding.header.part4.pixel_bit_depth_4bits = 0;
        }
        _ => return Err(BpeError::FileError),
    }

    let depth = coding.header.part4.pixel_bit_depth_4bits as u64;
    if coding.image_rows as u64 * coding.image_width as u64 * depth / 8 != img_len {
        return Err(BpeError::FileError);
    }
    Ok(())
}
