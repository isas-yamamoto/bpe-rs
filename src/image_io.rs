//! Image I/O — ImageSize/Read/Write in bpe_encoder.c / bpe_decoder.c

use crate::error::{BpeError, BpeResult};
use crate::types::{alloc_image_i32, CodingPara, ImageF32, ImageI32};
use std::fs::{metadata, File};
use std::io::{Read, Write};

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

/// Read the raw image (unpadded) into an rows x width array.
pub fn image_read(coding: &CodingPara) -> BpeResult<ImageI32> {
    let mut file = File::open(&coding.input_file).map_err(|_| BpeError::FileError)?;
    let rows = coding.image_rows as usize;
    let cols = coding.image_width as usize;
    let mut image = alloc_image_i32(rows, cols);
    let depth = coding.header.part4.pixel_bit_depth_4bits;
    let signed = coding.header.part4.signed_pixels;

    if depth != 0 && depth <= 8 {
        let mut rowbuf = vec![0u8; cols];
        for r in 0..rows {
            file.read_exact(&mut rowbuf)
                .map_err(|_| BpeError::FileError)?;
            for i in 0..cols {
                image[r][i] = if signed {
                    rowbuf[i] as i8 as i32
                } else {
                    rowbuf[i] as i32
                };
            }
        }
    } else {
        let mut rowbuf = vec![0u8; cols * 2];
        for r in 0..rows {
            file.read_exact(&mut rowbuf)
                .map_err(|_| BpeError::FileError)?;
            for i in 0..cols {
                let v = u16::from_le_bytes([rowbuf[i * 2], rowbuf[i * 2 + 1]]);
                image[r][i] = if signed { v as i16 as i32 } else { v as i32 };
            }
        }
        // Kiely endian fix: swap when image byte order differs from the host.
        let machineendianness: u8 = 0; // little-endian host
        if coding.pixel_byte_order != machineendianness {
            for r in 0..rows {
                for i in 0..cols {
                    let v = image[r][i];
                    image[r][i] = (v >> 8) + ((v << 8) & 0xFF00);
                }
            }
        }
    }
    Ok(image)
}

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

/// Optional endian swap: `((v << 8) & 0xFF00) + (v >> 8)`.
fn maybe_byte_swap(v: i32, swap: bool) -> i32 {
    if swap {
        ((v << 8) & 0xFF00) + (v >> 8)
    } else {
        v
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
    let machineendianness: u8 = 0;
    let swap = coding.pixel_byte_order != machineendianness;

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
    let machineendianness: u8 = 0;
    let swap = coding.pixel_byte_order != machineendianness;

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
