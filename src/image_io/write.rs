//! Raw image output — `ImageWrite` in bpe_decoder.c (integer and float paths).

use crate::error::{BpeError, BpeResult};
use crate::image_io::common::{byte_order_differs, maybe_byte_swap};
use crate::types::{CodingPara, ImageF32, ImageI32};
use std::fs::File;
use std::io::Write;

/// Clamp + convert one 8-bit pixel (integer path).
fn clamp_u8_i32(v: i32, signed: bool) -> u8 {
    let mut v = v;
    if signed {
        if v > 127 {
            v = 127;
        }
        if v < -128 {
            v = -128;
        }
        v as i8 as u8
    } else {
        if v > 0xFF {
            v = 0xFF;
        }
        if v < 0 {
            v = 0;
        }
        v as u8
    }
}

/// Clamp + convert one 8-bit pixel (float path: clamp in f32 then cast).
fn clamp_u8_f32(v: f32, signed: bool) -> u8 {
    let mut v = v;
    if signed {
        if v > 127.0 {
            v = 127.0;
        }
        if v < -128.0 {
            v = -128.0;
        }
        (v as i32 as i8) as u8
    } else {
        if v > 255.0 {
            v = 255.0;
        }
        if v < 0.0 {
            v = 0.0;
        }
        v as u8
    }
}

/// 16-bit pixel_max / pixel_min for signed and unsigned (depth==0 vs otherwise).
fn pixel_limits_16(depth: u8, signed: bool) -> (i32, i32) {
    if !signed {
        let pixel_max: i32 = if depth == 0 {
            (1i32 << 16) - 1
        } else {
            (1i32 << depth) - 1
        };
        (0, pixel_max)
    } else {
        let pixel_max: i32 = if depth == 0 {
            (1i32 << 15) - 1
        } else {
            (1i32 << (depth - 1)) - 1
        };
        let pixel_min: i32 = -pixel_max - 1;
        (pixel_min, pixel_max)
    }
}

/// Emit rows of already-converted 8-bit samples.
fn write_u8_rows(
    file: &mut File,
    rows: usize,
    width: usize,
    sample: impl Fn(usize, usize) -> u8,
) -> BpeResult<()> {
    let mut row = vec![0u8; width];
    for r in 0..rows {
        for i in 0..width {
            row[i] = sample(r, i);
        }
        file.write_all(&row).map_err(|_| BpeError::FileError)?;
    }
    Ok(())
}

/// Emit rows of 16-bit samples (optional byte-swap, signed vs unsigned store).
fn write_u16_rows(
    file: &mut File,
    rows: usize,
    width: usize,
    signed: bool,
    swap: bool,
    sample: impl Fn(usize, usize) -> i32,
) -> BpeResult<()> {
    for r in 0..rows {
        let mut buf = Vec::with_capacity(width * 2);
        for i in 0..width {
            let out = maybe_byte_swap(sample(r, i), swap);
            if signed {
                buf.extend_from_slice(&((out as i16) as u16).to_le_bytes());
            } else {
                buf.extend_from_slice(&(out as u16).to_le_bytes());
            }
        }
        file.write_all(&buf).map_err(|_| BpeError::FileError)?;
    }
    Ok(())
}

/// Integer output path (also removes the padded rows).
pub fn image_write(coding: &mut CodingPara, image: &ImageI32) -> BpeResult<()> {
    let mut file = File::create(&coding.coding_output_file).map_err(|_| BpeError::FileError)?;
    coding.image_rows -= coding.header.part1.pad_rows_3bits as u32;
    let rows = coding.image_rows as usize;
    let width = coding.image_width as usize;
    let depth = coding.header.part4.pixel_bit_depth_4bits;
    let signed = coding.header.part4.signed_pixels;
    let swap = byte_order_differs(coding.pixel_byte_order);

    if depth != 0 && depth <= 8 {
        write_u8_rows(&mut file, rows, width, |r, i| {
            clamp_u8_i32(image[r][i], signed)
        })?;
    } else if depth == 0 || depth <= 15 {
        let (pixel_min, pixel_max) = pixel_limits_16(depth, signed);
        write_u16_rows(&mut file, rows, width, signed, swap, |r, i| {
            let mut v = image[r][i];
            if v > pixel_max {
                v = pixel_max;
            }
            if v < pixel_min {
                v = pixel_min;
            }
            v
        })?;
    }
    Ok(())
}

/// Floating output path (keeps the padded rows).
pub fn image_write_float(coding: &CodingPara, image: &ImageF32) -> BpeResult<()> {
    let mut file = File::create(&coding.coding_output_file).map_err(|_| BpeError::FileError)?;
    let rows = coding.image_rows as usize;
    let width = coding.image_width as usize;
    let depth = coding.header.part4.pixel_bit_depth_4bits;
    let signed = coding.header.part4.signed_pixels;
    let swap = byte_order_differs(coding.pixel_byte_order);

    if depth != 0 && depth <= 8 {
        write_u8_rows(&mut file, rows, width, |r, i| {
            clamp_u8_f32(image[r][i], signed)
        })?;
    } else if depth == 0 || depth <= 15 {
        let (pixel_min, pixel_max) = pixel_limits_16(depth, signed);
        write_u16_rows(&mut file, rows, width, signed, swap, |r, i| {
            let mut v = image[r][i];
            if v > pixel_max as f32 {
                v = pixel_max as f32;
            }
            if v < pixel_min as f32 {
                v = pixel_min as f32;
            }
            v as i32
        })?;
    }
    Ok(())
}
