//! Raw image input — `ImageRead` in bpe_encoder.c.

use crate::error::{BpeError, BpeResult};
use crate::image_io::common::{byte_order_differs, byte_swap_16};
use crate::types::{alloc_image_i32, CodingPara, ImageI32};
use std::fs::File;
use std::io::Read;

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
        if byte_order_differs(coding.pixel_byte_order) {
            for r in 0..rows {
                for i in 0..cols {
                    image[r][i] = byte_swap_16(image[r][i]);
                }
            }
        }
    }
    Ok(image)
}
